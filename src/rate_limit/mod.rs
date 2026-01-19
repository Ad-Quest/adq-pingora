use once_cell::sync::Lazy;
use pingora_limits::rate::Rate;
use pingora::prelude::*;
use pingora::http::ResponseHeader;
use std::collections::HashMap;
use std::time::Duration;
use log::info;

/// Глобальный rate limiter
static RATE_LIMITER: Lazy<Rate> = Lazy::new(|| Rate::new(Duration::from_secs(1)));

/// Конфигурация rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Максимальное количество запросов в секунду по умолчанию
    pub max_requests_per_second: isize,
    /// IP адреса в whitelist (без ограничений)
    pub whitelist: Vec<String>,
    /// Лимиты для конкретных API ключей
    pub per_api_key_limits: HashMap<String, isize>,
    /// Включен ли rate limiting
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_second: 100,
            whitelist: vec![],
            per_api_key_limits: HashMap::new(),
            enabled: true,
        }
    }
}

impl RateLimitConfig {
    /// Создает новую конфигурацию с настройками по умолчанию
    pub fn new() -> Self {
        Self::default()
    }

    /// Создает конфигурацию с кастомными настройками
    pub fn with_limit(max_requests_per_second: isize) -> Self {
        Self {
            max_requests_per_second,
            ..Default::default()
        }
    }

    /// Добавляет IP в whitelist
    pub fn add_to_whitelist(&mut self, ip: String) {
        self.whitelist.push(ip);
    }

    /// Добавляет лимит для API ключа
    pub fn set_api_key_limit(&mut self, api_key: String, limit: isize) {
        self.per_api_key_limits.insert(api_key, limit);
    }
}

/// Получает идентификатор клиента для rate limiting
/// Приоритет: API ключ > IP адрес
fn get_client_identifier(session: &Session) -> String {
    // Сначала проверяем API ключ
    if let Some(api_key) = session
        .req_header()
        .headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
    {
        return format!("api_key:{}", api_key);
    }

    // Иначе используем IP адрес (извлекаем IP из SocketAddr строки)
    session
        .client_addr()
        .map(|addr| {
            // SocketAddr.to_string() возвращает "IP:PORT", берем только IP часть
            let addr_str = addr.to_string();
            addr_str.split(':').next().unwrap_or("unknown").to_string()
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Проверяет rate limit для запроса
/// Возвращает Ok(true) если запрос был заблокирован (429), Ok(false) если можно продолжить
pub async fn check_rate_limit(
    session: &mut Session,
    config: &RateLimitConfig,
) -> Result<bool> {
    // Если rate limiting отключен, пропускаем
    if !config.enabled {
        return Ok(false);
    }

    // Получаем идентификатор клиента
    let client_id = get_client_identifier(session);

    // Проверяем whitelist
    if config.whitelist.contains(&client_id) {
        return Ok(false); // Пропускаем без ограничений
    }

    // Определяем лимит для клиента
    let limit = if client_id.starts_with("api_key:") {
        // Для API ключей используем специальный лимит или дефолтный
        let api_key = client_id.strip_prefix("api_key:").unwrap_or("");
        config
            .per_api_key_limits
            .get(api_key)
            .copied()
            .unwrap_or(config.max_requests_per_second)
    } else {
        // Для IP адресов используем дефолтный лимит
        config.max_requests_per_second
    };

    // Проверяем текущее количество запросов
    let current_requests = RATE_LIMITER.observe(&client_id, 1);

    if current_requests > limit {
        info!(
            "Rate limit exceeded for {}: {} req/s (limit: {})",
            client_id, current_requests, limit
        );

        // Возвращаем 429 Too Many Requests
        let mut response = ResponseHeader::build(429, None)?;
        response.insert_header("X-Rate-Limit-Limit", limit.to_string())?;
        response.insert_header("X-Rate-Limit-Remaining", "0")?;
        response.insert_header("X-Rate-Limit-Reset", "1")?;
        response.insert_header("Retry-After", "1")?;
        response.insert_header("Content-Type", "application/json")?;

        // Добавляем CORS заголовки для JSON ответа
        response.insert_header("Access-Control-Allow-Origin", "*")?;

        let error_body = r#"{"error":"Too Many Requests","message":"Rate limit exceeded"}"#;
        response.insert_header("Content-Length", error_body.len().to_string())?;

        session.set_keepalive(None);
        session.write_response_header(Box::new(response), false).await?;
        session
            .write_response_body(Some(bytes::Bytes::from(error_body)), true)
            .await?;

        return Ok(true); // Запрос обработан (заблокирован)
    }

    Ok(false) // Продолжаем обработку
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests_per_second, 100);
        assert!(config.enabled);
    }

    #[test]
    fn test_rate_limit_config_custom() {
        let config = RateLimitConfig::with_limit(50);
        assert_eq!(config.max_requests_per_second, 50);
    }

    #[test]
    fn test_rate_limit_config_whitelist() {
        let mut config = RateLimitConfig::new();
        config.add_to_whitelist("127.0.0.1".to_string());
        assert!(config.whitelist.contains(&"127.0.0.1".to_string()));
    }

    #[test]
    fn test_rate_limit_config_api_key() {
        let mut config = RateLimitConfig::new();
        config.set_api_key_limit("premium-key".to_string(), 1000);
        assert_eq!(
            config.per_api_key_limits.get("premium-key"),
            Some(&1000)
        );
    }
}
