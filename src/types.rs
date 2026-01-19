/// Типы сервисов для маршрутизации
#[derive(Debug, PartialEq)]
pub enum ServiceType {
    CoreApi,
    ChallengeApi,
    BillingApi,
    ErirApi,
    SharedApi,
    ZitadelAuth,
    Static,
}

/// Контекст запроса
#[derive(Debug)]
pub struct RequestContext {
    pub service_type: ServiceType,
    pub upstream_host: String,
    pub upstream_port: u16,
    /// Количество попыток retry
    pub retries: u32,
    /// Время начала запроса для измерения длительности
    pub start_time: std::time::Instant,
}

impl RequestContext {
    pub fn new() -> Self {
        Self {
            service_type: ServiceType::Static,
            upstream_host: String::new(),
            upstream_port: 0,
            retries: 0,
            start_time: std::time::Instant::now(),
        }
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}