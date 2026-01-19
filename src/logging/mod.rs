use tracing::{info, warn, error, debug};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::OpenOptions;
use std::io::Write;
use pingora_proxy::Session;
use crate::config::LoggingConfig;

/// Инициализирует систему логирования
pub fn init_logging(config: &LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Проверяем, не установлен ли уже глобальный логгер
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", &config.level);
    }

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    let result = if config.format == "json" {
        // JSON формат для production
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .with_span_events(FmtSpan::CLOSE)
            .with_current_span(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .try_init()
    } else {
        // Обычный текстовый формат для разработки
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_span_events(FmtSpan::CLOSE)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .try_init()
    };

    match result {
        Ok(_) => {
            info!("Logging initialized with level: {}, format: {}", config.level, config.format);
        }
        Err(_) => {
            // Логгер уже установлен, используем существующий
            eprintln!("Global logger already set, using existing configuration");
        }
    }

    Ok(())
}

/// Структура для логирования HTTP запросов
#[derive(Debug)]
pub struct AccessLogger {
    config: LoggingConfig,
}

impl AccessLogger {
    pub fn new(config: LoggingConfig) -> Self {
        Self { config }
    }

    /// Логирует HTTP запрос
    pub async fn log_request(&self, session: &Session, response_status: u16, response_size: u64, duration_ms: u64) {
        if !self.config.access_log.enabled {
            return;
        }

        let req = session.req_header();
        let client_addr = session.client_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let log_entry = if self.config.access_log.format == "json" {
            // JSON формат
            json!({
                "timestamp": timestamp,
                "level": "INFO",
                "message": "HTTP Request",
                "fields": {
                    "client_ip": client_addr,
                    "method": req.method.as_str(),
                    "uri": req.uri.to_string(),
                    "version": format!("{:?}", req.version),
                    "status": response_status,
                    "response_size": response_size,
                    "duration_ms": duration_ms,
                    "user_agent": req.headers.get("user-agent")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("-"),
                    "referer": req.headers.get("referer")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("-"),
                    "host": req.headers.get("host")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("-"),
                    "x_forwarded_for": req.headers.get("x-forwarded-for")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("-"),
                    "x_real_ip": req.headers.get("x-real-ip")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("-")
                }
            }).to_string()
        } else {
            // Nginx-like формат
            format!(
                "{} - - [{}] \"{} {} {:?}\" {} {} \"{}\" \"{}\"",
                client_addr,
                format_timestamp(timestamp),
                req.method.as_str(),
                req.uri,
                req.version,
                response_status,
                response_size,
                req.headers.get("referer")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("-"),
                req.headers.get("user-agent")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("-")
            )
        };

        // Записываем в файл
        if let Err(e) = self.write_to_file(&log_entry).await {
            error!("Failed to write access log: {}", e);
        }

        // Также логируем через tracing для консоли
        info!(
            client_ip = %client_addr,
            method = %req.method,
            uri = %req.uri,
            status = response_status,
            duration_ms = duration_ms,
            "HTTP Request"
        );
    }

    /// Записывает лог в файл
    async fn write_to_file(&self, log_entry: &str) -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.access_log.path)?;
        
        writeln!(file, "{}", log_entry)?;
        file.flush()?;
        Ok(())
    }
}

/// Структура для логирования ошибок
pub struct ErrorLogger {
    config: LoggingConfig,
}

impl ErrorLogger {
    pub fn new(config: LoggingConfig) -> Self {
        Self { config }
    }

    /// Логирует ошибку
    pub async fn log_error(&self, 
        error_type: &str, 
        message: &str, 
        details: Option<&str>,
        client_ip: Option<&str>,
        uri: Option<&str>
    ) {
        if !self.config.error_log.enabled {
            return;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let log_entry = if self.config.error_log.format == "json" {
            json!({
                "timestamp": timestamp,
                "level": "ERROR",
                "message": message,
                "fields": {
                    "error_type": error_type,
                    "details": details.unwrap_or(""),
                    "client_ip": client_ip.unwrap_or("unknown"),
                    "uri": uri.unwrap_or("unknown")
                }
            }).to_string()
        } else {
            format!(
                "[{}] [{}] {} - {} (client: {}, uri: {})",
                format_timestamp(timestamp),
                error_type,
                message,
                details.unwrap_or(""),
                client_ip.unwrap_or("unknown"),
                uri.unwrap_or("unknown")
            )
        };

        // Записываем в файл
        if let Err(e) = self.write_to_file(&log_entry).await {
            error!("Failed to write error log: {}", e);
        }

        // Также логируем через tracing
        error!(
            error_type = error_type,
            client_ip = client_ip.unwrap_or("unknown"),
            uri = uri.unwrap_or("unknown"),
            details = details.unwrap_or(""),
            "{}", message
        );
    }

    /// Записывает лог в файл
    async fn write_to_file(&self, log_entry: &str) -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.error_log.path)?;
        
        writeln!(file, "{}", log_entry)?;
        file.flush()?;
        Ok(())
    }
}

/// Форматирует timestamp в читаемый вид
fn format_timestamp(timestamp: u64) -> String {
    // Простое форматирование - в production лучше использовать chrono
    format!("{}", timestamp)
}

/// Макросы для удобного логирования
#[macro_export]
macro_rules! log_request {
    ($logger:expr, $session:expr, $status:expr, $size:expr, $duration:expr) => {
        $logger.log_request($session, $status, $size, $duration).await
    };
}

#[macro_export]
macro_rules! log_error {
    ($logger:expr, $error_type:expr, $message:expr) => {
        $logger.log_error($error_type, $message, None, None, None).await
    };
    ($logger:expr, $error_type:expr, $message:expr, $details:expr) => {
        $logger.log_error($error_type, $message, Some($details), None, None).await
    };
    ($logger:expr, $error_type:expr, $message:expr, $details:expr, $client_ip:expr, $uri:expr) => {
        $logger.log_error($error_type, $message, Some($details), Some($client_ip), Some($uri)).await
    };
}

/// Middleware для автоматического логирования запросов
pub struct LoggingMiddleware {
    access_logger: AccessLogger,
    error_logger: ErrorLogger,
}

impl LoggingMiddleware {
    pub fn new(config: LoggingConfig) -> Self {
        Self {
            access_logger: AccessLogger::new(config.clone()),
            error_logger: ErrorLogger::new(config),
        }
    }

    pub fn access_logger(&self) -> &AccessLogger {
        &self.access_logger
    }

    pub fn error_logger(&self) -> &ErrorLogger {
        &self.error_logger
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LoggingConfig, LogConfig, MetricsConfig};
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_access_logger() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("access.log");

        let config = LoggingConfig {
            format: "json".to_string(),
            level: "info".to_string(),
            access_log: LogConfig {
                enabled: true,
                path: log_path.to_string_lossy().to_string(),
                format: "json".to_string(),
            },
            error_log: LogConfig {
                enabled: false,
                path: "".to_string(),
                format: "text".to_string(),
            },
            metrics: MetricsConfig {
                enabled: false,
                endpoint: "/metrics".to_string(),
                port: 9090,
            },
        };

        let logger = AccessLogger::new(config);
        
        // Создаем mock session (в реальном коде это будет настоящая Session)
        // Для теста просто проверим, что файл создается
        let log_entry = r#"{"timestamp":1234567890,"level":"INFO","message":"Test"}"#;
        logger.write_to_file(log_entry).await.unwrap();

        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Test"));
    }
}