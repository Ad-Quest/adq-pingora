use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;
use log::{info, warn, debug};
use crate::config::CircuitBreakerConfig;

/// Состояния Circuit Breaker
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,    // Нормальная работа
    Open,      // Блокируем запросы
    HalfOpen,  // Тестируем восстановление
}

/// Статистика для Circuit Breaker
#[derive(Debug, Clone)]
struct CircuitStats {
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    state: CircuitState,
    next_attempt: Option<Instant>,
}

impl Default for CircuitStats {
    fn default() -> Self {
        Self {
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            state: CircuitState::Closed,
            next_attempt: None,
        }
    }
}

/// Circuit Breaker для защиты от каскадных сбоев
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    circuits: Arc<RwLock<HashMap<String, CircuitStats>>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            circuits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Проверяет, можно ли выполнить запрос к upstream
    pub async fn can_execute(&self, upstream_name: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        let mut circuits = self.circuits.write().await;
        let stats = circuits.entry(upstream_name.to_string()).or_default();

        let now = Instant::now();

        match stats.state {
            CircuitState::Closed => {
                // Нормальная работа - разрешаем запрос
                true
            }
            CircuitState::Open => {
                // Проверяем, не пора ли перейти в HalfOpen
                if let Some(next_attempt) = stats.next_attempt {
                    if now >= next_attempt {
                        info!("Circuit breaker for '{}' transitioning to HalfOpen", upstream_name);
                        stats.state = CircuitState::HalfOpen;
                        stats.success_count = 0;
                        true
                    } else {
                        debug!("Circuit breaker for '{}' is Open, blocking request", upstream_name);
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                // В состоянии тестирования - разрешаем ограниченное количество запросов
                true
            }
        }
    }

    /// Регистрирует успешный запрос
    pub async fn record_success(&self, upstream_name: &str) {
        if !self.config.enabled {
            return;
        }

        let mut circuits = self.circuits.write().await;
        let stats = circuits.entry(upstream_name.to_string()).or_default();

        match stats.state {
            CircuitState::Closed => {
                // Сбрасываем счетчик ошибок при успехе
                stats.failure_count = 0;
                debug!("Circuit breaker for '{}': success recorded, failure count reset", upstream_name);
            }
            CircuitState::HalfOpen => {
                stats.success_count += 1;
                debug!("Circuit breaker for '{}': success in HalfOpen state ({}/{})", 
                       upstream_name, stats.success_count, self.config.success_threshold);

                // Если достигли порога успешных запросов, закрываем circuit
                if stats.success_count >= self.config.success_threshold {
                    info!("Circuit breaker for '{}' transitioning to Closed after {} successes", 
                          upstream_name, stats.success_count);
                    stats.state = CircuitState::Closed;
                    stats.failure_count = 0;
                    stats.success_count = 0;
                    stats.next_attempt = None;
                }
            }
            CircuitState::Open => {
                // В открытом состоянии успехи не должны происходить
                warn!("Unexpected success recorded for open circuit breaker '{}'", upstream_name);
            }
        }
    }

    /// Регистрирует неудачный запрос
    pub async fn record_failure(&self, upstream_name: &str) {
        if !self.config.enabled {
            return;
        }

        let mut circuits = self.circuits.write().await;
        let stats = circuits.entry(upstream_name.to_string()).or_default();

        let now = Instant::now();
        stats.failure_count += 1;
        stats.last_failure_time = Some(now);

        match stats.state {
            CircuitState::Closed => {
                debug!("Circuit breaker for '{}': failure recorded ({}/{})", 
                       upstream_name, stats.failure_count, self.config.failure_threshold);

                // Проверяем, не достигли ли порога ошибок
                if stats.failure_count >= self.config.failure_threshold {
                    warn!("Circuit breaker for '{}' transitioning to Open after {} failures", 
                          upstream_name, stats.failure_count);
                    stats.state = CircuitState::Open;
                    stats.next_attempt = Some(now + Duration::from_secs(self.config.recovery_timeout));
                }
            }
            CircuitState::HalfOpen => {
                // При ошибке в HalfOpen сразу возвращаемся в Open
                warn!("Circuit breaker for '{}' transitioning back to Open due to failure in HalfOpen", 
                      upstream_name);
                stats.state = CircuitState::Open;
                stats.success_count = 0;
                stats.next_attempt = Some(now + Duration::from_secs(self.config.recovery_timeout));
            }
            CircuitState::Open => {
                // В открытом состоянии просто обновляем время следующей попытки
                stats.next_attempt = Some(now + Duration::from_secs(self.config.recovery_timeout));
                debug!("Circuit breaker for '{}': failure in Open state, next attempt at {:?}", 
                       upstream_name, stats.next_attempt);
            }
        }
    }

    /// Получает текущее состояние circuit breaker
    pub async fn get_state(&self, upstream_name: &str) -> CircuitState {
        if !self.config.enabled {
            return CircuitState::Closed;
        }

        let circuits = self.circuits.read().await;
        circuits.get(upstream_name)
            .map(|stats| stats.state.clone())
            .unwrap_or(CircuitState::Closed)
    }

    /// Получает статистику всех circuit breakers
    pub async fn get_all_stats(&self) -> HashMap<String, (CircuitState, u32, u32)> {
        let circuits = self.circuits.read().await;
        circuits.iter()
            .map(|(name, stats)| {
                (name.clone(), (stats.state.clone(), stats.failure_count, stats.success_count))
            })
            .collect()
    }

    /// Принудительно сбрасывает circuit breaker в состояние Closed
    pub async fn reset(&self, upstream_name: &str) {
        let mut circuits = self.circuits.write().await;
        if let Some(stats) = circuits.get_mut(upstream_name) {
            info!("Manually resetting circuit breaker for '{}'", upstream_name);
            stats.state = CircuitState::Closed;
            stats.failure_count = 0;
            stats.success_count = 0;
            stats.next_attempt = None;
            stats.last_failure_time = None;
        }
    }

    /// Принудительно открывает circuit breaker
    pub async fn force_open(&self, upstream_name: &str) {
        let mut circuits = self.circuits.write().await;
        let stats = circuits.entry(upstream_name.to_string()).or_default();
        
        info!("Manually opening circuit breaker for '{}'", upstream_name);
        stats.state = CircuitState::Open;
        stats.next_attempt = Some(Instant::now() + Duration::from_secs(self.config.recovery_timeout));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[test]
    async fn test_circuit_breaker_transitions() {
        let config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 3,
            recovery_timeout: 1, // 1 секунда для быстрого тестирования
            success_threshold: 2,
        };

        let cb = CircuitBreaker::new(config);
        let upstream = "test_upstream";

        // Начальное состояние - Closed
        assert_eq!(cb.get_state(upstream).await, CircuitState::Closed);
        assert!(cb.can_execute(upstream).await);

        // Регистрируем ошибки
        cb.record_failure(upstream).await;
        assert_eq!(cb.get_state(upstream).await, CircuitState::Closed);
        
        cb.record_failure(upstream).await;
        assert_eq!(cb.get_state(upstream).await, CircuitState::Closed);
        
        cb.record_failure(upstream).await;
        // После 3 ошибок должен открыться
        assert_eq!(cb.get_state(upstream).await, CircuitState::Open);
        assert!(!cb.can_execute(upstream).await);

        // Ждем время восстановления
        sleep(Duration::from_secs(2)).await;
        
        // Должен перейти в HalfOpen при следующей проверке
        assert!(cb.can_execute(upstream).await);
        assert_eq!(cb.get_state(upstream).await, CircuitState::HalfOpen);

        // Регистрируем успехи для закрытия
        cb.record_success(upstream).await;
        assert_eq!(cb.get_state(upstream).await, CircuitState::HalfOpen);
        
        cb.record_success(upstream).await;
        // После 2 успехов должен закрыться
        assert_eq!(cb.get_state(upstream).await, CircuitState::Closed);
    }

    #[test]
    async fn test_circuit_breaker_disabled() {
        let config = CircuitBreakerConfig {
            enabled: false,
            failure_threshold: 1,
            recovery_timeout: 1,
            success_threshold: 1,
        };

        let cb = CircuitBreaker::new(config);
        let upstream = "test_upstream";

        // Даже после ошибок должен оставаться доступным
        cb.record_failure(upstream).await;
        cb.record_failure(upstream).await;
        cb.record_failure(upstream).await;
        
        assert_eq!(cb.get_state(upstream).await, CircuitState::Closed);
        assert!(cb.can_execute(upstream).await);
    }
}