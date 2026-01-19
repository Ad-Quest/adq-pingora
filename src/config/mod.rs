use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub mod nginx_parser;
pub use nginx_parser::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub version: u32,
    pub global: GlobalConfig,
    pub security: SecurityConfig,
    pub cache: CacheConfig,
    pub logging: LoggingConfig,
    pub ip_filter: IpFilterConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    // Nginx-style конфигурация загружается отдельно
    #[serde(skip)]
    pub nginx_config: Option<NginxConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    pub default_timeout: u64,
    pub max_retries: u32,
    pub health_check_interval: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpstreamConfig {
    pub algorithm: String, // round_robin, weighted, hash, least_conn
    pub health_check: HealthCheckConfig,
    pub servers: Vec<ServerConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthCheckConfig {
    #[serde(rename = "type")]
    pub check_type: String, // http, tcp
    pub path: Option<String>,
    pub method: Option<String>,
    pub timeout: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub address: String,
    pub weight: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub headers: SecurityHeaders,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityHeaders {
    pub x_frame_options: String,
    pub x_content_type_options: String,
    pub x_xss_protection: String,
    pub strict_transport_security: String,
    pub content_security_policy: String,
    pub server: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub default_ttl: u64,
    pub max_size: String,
    pub rules: Vec<CacheRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheRule {
    pub path: String,
    pub ttl: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub format: String, // json или text
    pub level: String,  // error, warn, info, debug, trace
    pub access_log: LogConfig,
    pub error_log: LogConfig,
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogConfig {
    pub enabled: bool,
    pub path: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IpFilterConfig {
    pub enabled: bool,
    pub blacklist_file: Option<String>,
    pub whitelist: Option<Vec<String>>,
    pub max_connections_per_ip: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    pub failure_threshold: u32,
    pub recovery_timeout: u64,
    pub success_threshold: u32,
}

impl Config {
    /// Загружает основную конфигурацию из YAML файла
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let mut config: Config = serde_yaml::from_str(&content)?;
        
        // Загружаем nginx-style конфигурацию из sites-enabled
        config.nginx_config = Some(NginxConfig::load_from_sites_enabled("/etc/adq-pingora/sites-enabled")?);
        
        Ok(config)
    }

    /// Загружает только nginx-style конфигурацию
    pub fn load_nginx_config() -> Result<NginxConfig, Box<dyn std::error::Error>> {
        NginxConfig::load_from_sites_enabled("/etc/adq-pingora/sites-enabled")
    }

    /// Сохраняет конфигурацию в YAML файл
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        // Не сохраняем nginx_config в YAML, так как он загружается из sites-enabled
        let mut config_to_save = self.clone();
        config_to_save.nginx_config = None;
        
        let content = serde_yaml::to_string(&config_to_save)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Создает конфигурацию по умолчанию
    pub fn default() -> Self {
        Self {
            version: 1,
            global: GlobalConfig {
                default_timeout: 30,
                max_retries: 3,
                health_check_interval: 5,
            },
            security: SecurityConfig {
                headers: SecurityHeaders {
                    x_frame_options: "SAMEORIGIN".to_string(),
                    x_content_type_options: "nosniff".to_string(),
                    x_xss_protection: "1; mode=block".to_string(),
                    strict_transport_security: "max-age=31536000; includeSubDomains".to_string(),
                    content_security_policy: "default-src 'self'".to_string(),
                    server: "Pingora/0.6.0".to_string(),
                },
            },
            cache: CacheConfig {
                enabled: false,
                default_ttl: 300,
                max_size: "1GB".to_string(),
                rules: Vec::new(),
            },
            logging: LoggingConfig {
                format: "json".to_string(),
                level: "info".to_string(),
                access_log: LogConfig {
                    enabled: true,
                    path: "/var/log/pingora-proxy/access.log".to_string(),
                    format: "json".to_string(),
                },
                error_log: LogConfig {
                    enabled: true,
                    path: "/var/log/pingora-proxy/error.log".to_string(),
                    format: "json".to_string(),
                },
                metrics: MetricsConfig {
                    enabled: true,
                    endpoint: "/metrics".to_string(),
                    port: 9090,
                },
            },
            ip_filter: IpFilterConfig {
                enabled: false,
                blacklist_file: None,
                whitelist: None,
                max_connections_per_ip: None,
            },
            circuit_breaker: CircuitBreakerConfig {
                enabled: false,
                failure_threshold: 5,
                recovery_timeout: 30,
                success_threshold: 3,
            },
            nginx_config: None,
        }
    }

    /// Находит server блок по хосту (из nginx конфигурации)
    pub fn find_server(&self, host: &str) -> Option<&ServerBlock> {
        self.nginx_config.as_ref()?.find_server(host)
    }

    /// Находит location в server блоке по пути
    pub fn find_location<'a>(&self, server: &'a ServerBlock, path: &str) -> Option<&'a LocationBlock> {
        self.nginx_config.as_ref()?.find_location(server, path)
    }

    /// Получает upstream по имени
    pub fn get_upstream(&self, name: &str) -> Option<&UpstreamBlock> {
        self.nginx_config.as_ref()?.get_upstream(name)
    }

    /// Получает все upstreams
    pub fn get_all_upstreams(&self) -> HashMap<String, &UpstreamBlock> {
        if let Some(nginx_config) = &self.nginx_config {
            nginx_config.upstreams.iter().map(|(k, v)| (k.clone(), v)).collect()
        } else {
            HashMap::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_route() {
        let mut config = Config::default();
        
        // Добавляем тестовый маршрут
        config.routes.push(RouteConfig {
            name: "test".to_string(),
            hosts: vec!["api.example.com".to_string(), "localhost:8080".to_string()],
            paths: vec!["/api/*".to_string(), "/health".to_string()],
            upstream: "test_upstream".to_string(),
            ssl: SslConfig { enabled: false, cert_path: None, key_path: None },
            cors: CorsConfig { enabled: false, origins: vec![] },
            rate_limit: RateLimitConfig {
                enabled: false,
                requests_per_second: 100,
                burst: None,
                whitelist: None,
                api_key_limits: None,
            },
        });

        // Тестируем поиск маршрута
        assert!(config.find_route("api.example.com", "/api/users").is_some());
        assert!(config.find_route("api.example.com:443", "/api/users").is_some());
        assert!(config.find_route("localhost:8080", "/health").is_some());
        assert!(config.find_route("unknown.com", "/api/users").is_none());
        assert!(config.find_route("api.example.com", "/unknown").is_none());
    }
}