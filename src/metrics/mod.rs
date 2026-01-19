use once_cell::sync::Lazy;
use prometheus::{
    register_int_counter, register_int_counter_vec, register_histogram, register_gauge,
    IntCounter, IntCounterVec, Histogram, Gauge,
};
use log::info;

/// Общее количество HTTP запросов
pub static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        &["method", "status", "service"]
    )
    .expect("Failed to register http_requests_total metric")
});

/// Длительность обработки HTTP запросов
pub static HTTP_REQUEST_DURATION: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds"
    )
    .expect("Failed to register http_request_duration_seconds metric")
});

/// Количество соединений к upstream серверам
pub static UPSTREAM_CONNECTIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "upstream_connections_total",
        "Total upstream connections",
        &["upstream", "status"]
    )
    .expect("Failed to register upstream_connections_total metric")
});

/// Количество срабатываний rate limit
pub static RATE_LIMIT_HITS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "rate_limit_hits_total",
        "Total rate limit hits"
    )
    .expect("Failed to register rate_limit_hits_total metric")
});

/// Количество retry попыток
pub static RETRY_ATTEMPTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "retry_attempts_total",
        "Total retry attempts",
        &["service", "result"]
    )
    .expect("Failed to register retry_attempts_total metric")
});

/// Активные соединения
pub static ACTIVE_CONNECTIONS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "active_connections",
        "Number of active connections"
    )
    .expect("Failed to register active_connections metric")
});

/// Инициализация метрик
pub fn init_metrics() {
    info!("Prometheus metrics initialized");
    info!("Available metrics:");
    info!("  - http_requests_total");
    info!("  - http_request_duration_seconds");
    info!("  - upstream_connections_total");
    info!("  - rate_limit_hits_total");
    info!("  - retry_attempts_total");
    info!("  - active_connections");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        // Просто проверяем, что метрики создаются без ошибок
        let _ = HTTP_REQUESTS_TOTAL.with_label_values(&["GET", "200", "core_api"]);
        let _ = HTTP_REQUEST_DURATION.observe(0.1);
        let _ = RATE_LIMIT_HITS.inc();
    }
}
