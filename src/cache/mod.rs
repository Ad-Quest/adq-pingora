use pingora_cache::{CacheKey, RespCacheable, CacheMeta};
use pingora_core::Result;
use pingora_proxy::Session;
use pingora::http::{RequestHeader, ResponseHeader};
use std::time::{Duration, SystemTime};
use regex::Regex;
use log::{info, debug};
use crate::config::{CacheConfig, CacheRule};

/// Менеджер кеширования
pub struct CacheManager {
    config: CacheConfig,
    path_regexes: Vec<(Regex, u64)>, // (regex, ttl)
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Result<Self> {
        let mut path_regexes = Vec::new();
        
        // Компилируем регулярные выражения для правил кеширования
        for rule in &config.rules {
            let pattern = rule.path
                .replace("*", ".*")  // Заменяем * на .*
                .replace(".", "\\.");  // Экранируем точки
            
            match Regex::new(&format!("^{}$", pattern)) {
                Ok(regex) => {
                    path_regexes.push((regex, rule.ttl));
                    debug!("Compiled cache rule: {} -> {} seconds", rule.path, rule.ttl);
                }
                Err(e) => {
                    log::warn!("Failed to compile cache rule regex '{}': {}", rule.path, e);
                }
            }
        }

        Ok(Self {
            config,
            path_regexes,
        })
    }

    /// Создает ключ кеша для запроса
    pub fn create_cache_key(&self, session: &Session) -> Option<CacheKey> {
        if !self.config.enabled {
            return None;
        }

        let req = session.req_header();
        
        // Кешируем только GET запросы
        if req.method != "GET" {
            return None;
        }

        // Создаем ключ на основе URL и некоторых заголовков
        let mut key_parts = Vec::new();
        
        // Добавляем хост
        if let Some(host) = req.headers.get("host") {
            if let Ok(host_str) = host.to_str() {
                key_parts.push(host_str.to_string());
            }
        }
        
        // Добавляем путь и query string
        key_parts.push(req.uri.path().to_string());
        if let Some(query) = req.uri.query() {
            key_parts.push(query.to_string());
        }

        // Добавляем Accept-Encoding для правильного кеширования сжатых ответов
        if let Some(encoding) = req.headers.get("accept-encoding") {
            if let Ok(encoding_str) = encoding.to_str() {
                key_parts.push(format!("ae:{}", encoding_str));
            }
        }

        let cache_key = key_parts.join("|");
        debug!("Created cache key: {}", cache_key);
        
        Some(CacheKey::new("adquest", cache_key, ""))
    }

    /// Определяет, можно ли кешировать ответ
    pub fn is_response_cacheable(&self, 
        session: &Session, 
        resp: &ResponseHeader
    ) -> Option<RespCacheable> {
        if !self.config.enabled {
            return None;
        }

        let req = session.req_header();
        
        // Кешируем только GET запросы
        if req.method != "GET" {
            return None;
        }

        // Не кешируем ошибки (кроме 404)
        let status = resp.status.as_u16();
        if status >= 400 && status != 404 {
            return None;
        }

        // Проверяем заголовки Cache-Control
        if let Some(cache_control) = resp.headers.get("cache-control") {
            if let Ok(cc_str) = cache_control.to_str() {
                if cc_str.contains("no-cache") || cc_str.contains("no-store") || cc_str.contains("private") {
                    debug!("Response not cacheable due to Cache-Control: {}", cc_str);
                    return None;
                }
            }
        }

        // Определяем TTL на основе правил
        let path = req.uri.path();
        let ttl = self.get_ttl_for_path(path);
        
        info!("Caching response for path '{}' with TTL {} seconds", path, ttl);

        // Временно возвращаем None пока не разберемся с API
        None
    }

    /// Получает TTL для пути на основе правил
    fn get_ttl_for_path(&self, path: &str) -> u64 {
        // Проверяем правила в порядке определения
        for (regex, ttl) in &self.path_regexes {
            if regex.is_match(path) {
                debug!("Path '{}' matched cache rule with TTL {}", path, ttl);
                return *ttl;
            }
        }

        // Возвращаем TTL по умолчанию
        debug!("Path '{}' using default TTL {}", path, self.config.default_ttl);
        self.config.default_ttl
    }

    /// Проверяет, нужно ли обновить кеш (для условных запросов)
    pub fn should_serve_stale(&self, 
        _session: &Session, 
        _cache_meta: &CacheMeta
    ) -> bool {
        // Простая логика: не обслуживаем устаревший кеш
        // В production можно добавить более сложную логику
        false
    }

    /// Модифицирует заголовки кешированного ответа
    pub fn modify_cache_headers(&self, resp: &mut ResponseHeader, cache_meta: &CacheMeta) {
        // Добавляем заголовок о том, что ответ из кеша
        let _ = resp.insert_header("X-Cache", "HIT");
        
        // Добавляем информацию о возрасте кеша
        // Временно закомментируем пока не разберемся с API
        // if let Ok(age) = cache_meta.age() {
        //     let _ = resp.insert_header("Age", age.as_secs().to_string());
        // }

        // Обновляем Date заголовок
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let _ = resp.insert_header("Date", httpdate::fmt_http_date(SystemTime::UNIX_EPOCH + Duration::from_secs(now)));
    }
}

/// Вспомогательные функции для работы с HTTP датами
mod httpdate {
    use std::time::SystemTime;

    pub fn fmt_http_date(time: SystemTime) -> String {
        // Простая реализация форматирования HTTP даты
        // В production лучше использовать специализированную библиотеку
        format!("{:?}", time) // Заглушка
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CacheConfig, CacheRule};

    #[test]
    fn test_cache_ttl_rules() {
        let config = CacheConfig {
            enabled: true,
            default_ttl: 300,
            max_size: "1GB".to_string(),
            rules: vec![
                CacheRule { path: "/api/static/*".to_string(), ttl: 3600 },
                CacheRule { path: "*.css".to_string(), ttl: 86400 },
                CacheRule { path: "*.js".to_string(), ttl: 86400 },
            ],
        };

        let cache_manager = CacheManager::new(config).unwrap();

        assert_eq!(cache_manager.get_ttl_for_path("/api/static/image.png"), 3600);
        assert_eq!(cache_manager.get_ttl_for_path("/styles/main.css"), 86400);
        assert_eq!(cache_manager.get_ttl_for_path("/scripts/app.js"), 86400);
        assert_eq!(cache_manager.get_ttl_for_path("/api/users"), 300); // default
    }
}