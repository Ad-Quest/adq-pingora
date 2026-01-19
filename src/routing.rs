use crate::types::{RequestContext, ServiceType};
use pingora::prelude::*;
use log::info;

/// Обрабатывает HTTP -> HTTPS редирект
pub async fn handle_https_redirect(
    session: &mut Session, 
    host: &str, 
    _uri: &str
) -> Result<bool> {
    // ВРЕМЕННО ОТКЛЮЧАЕМ ПРИНУДИТЕЛЬНЫЙ HTTPS РЕДИРЕКТ ДЛЯ ОТЛАДКИ
    // Проверяем, является ли соединение HTTPS
    let is_https = session.req_header().uri.scheme().is_some_and(|s| s == "https") ||
                  session.req_header().headers.get("x-forwarded-proto").is_some_and(|v| v == "https") ||
                  session.server_addr().is_some_and(|addr| {
                      // Проверяем порт через строковое представление
                      addr.to_string().ends_with(":443")
                  });
    
    let host_without_port = host.split(':').next().unwrap_or(host);
    
    // Логируем только если это не стандартный HTTP запрос
    if !is_https && (host_without_port.contains("ad-quest.ru") || host_without_port == "localhost") {
        info!("HTTP request allowed for host: {} (HTTPS: {})", host_without_port, is_https);
    }
    Ok(false)
}

/// Определяет маршрутизацию запроса
pub fn route_request(host: &str, uri: &str, ctx: &mut RequestContext) {
    let host_without_port = host.split(':').next().unwrap_or(host);
    
    // Сначала проверяем маршрутизацию по URI для localhost/127.0.0.1
    if (host_without_port == "127.0.0.1" || host_without_port == "localhost") && uri.starts_with("/api/") {
        // API запросы на localhost идут на Core API, а не на Zitadel
        route_localhost_api(uri, ctx, host);
        return;
    }
    
    if host_without_port == "auth.ad-quest.ru" || 
       (host_without_port == "localhost" && (host.contains(":8085") || host.contains(":8091"))) {
        // Zitadel Auth Service
        ctx.service_type = ServiceType::ZitadelAuth;
        ctx.upstream_port = 8091;  // Zitadel работает на порту 8091 (маппинг Docker)
        info!("Routing to ZITADEL AUTH service for host: {}", host_without_port);
        
    } else if host_without_port == "localhost" || host_without_port == "127.0.0.1" {
        // Для localhost/127.0.0.1 без /api/ - проверяем, может быть Zitadel консоль
        if uri.starts_with("/ui/") || uri.starts_with("/.well-known/") || uri.starts_with("/oauth/") {
            ctx.service_type = ServiceType::ZitadelAuth;
            ctx.upstream_port = 8091;
            info!("Routing to ZITADEL AUTH service for host: {} (Zitadel endpoint)", host_without_port);
        } else {
            // Localhost для разработки
            ctx.service_type = ServiceType::Static;
        }
        
    } else if host_without_port == "api.ad-quest.ru" {
        route_api_domain(uri, ctx);
        
    } else {
        route_localhost_api(uri, ctx, host);
    }
}

/// Маршрутизация для домена api.ad-quest.ru
fn route_api_domain(uri: &str, ctx: &mut RequestContext) {
    if uri.starts_with("/api/v1/logs") || uri.starts_with("/api/v1/analytics") || uri.starts_with("/api/v1/health") || uri == "/health" {
        // Логирование, аналитика и health check - направляем на Shared Services
        ctx.service_type = ServiceType::SharedApi;
        ctx.upstream_port = 8083;
        info!("Routing to SHARED API service for api.ad-quest.ru logs/analytics/health path: {}", uri);
        
    } else if uri.starts_with("/challenge") {
        ctx.service_type = ServiceType::ChallengeApi;
        ctx.upstream_port = 8080;
        info!("Routing to CHALLENGE API service for api.ad-quest.ru path: {}", uri);
        
    } else if uri.starts_with("/billing") {
        ctx.service_type = ServiceType::BillingApi;
        ctx.upstream_port = 8081;
        info!("Routing to BILLING API service for api.ad-quest.ru path: {}", uri);
        
    } else if uri.starts_with("/erir") {
        ctx.service_type = ServiceType::ErirApi;
        ctx.upstream_port = 8082;
        info!("Routing to ERIR API service for api.ad-quest.ru path: {}", uri);
        
    } else if uri.starts_with("/shared") || uri.starts_with("/tbank") {
        ctx.service_type = ServiceType::SharedApi;
        ctx.upstream_port = 8083;
        info!("Routing to SHARED API service for api.ad-quest.ru path: {}", uri);
        
    } else {
        // Общие API запросы на api.ad-quest.ru - направляем на Core API балансировщик
        ctx.service_type = ServiceType::CoreApi;
        info!("Routing to CORE API service for api.ad-quest.ru path: {}", uri);
    }
}

/// Маршрутизация для localhost и других доменов
fn route_localhost_api(uri: &str, ctx: &mut RequestContext, host: &str) {
    if uri.starts_with("/api/challenge") {
        // Challenge Engine API
        ctx.service_type = ServiceType::ChallengeApi;
        ctx.upstream_port = 8080;
        info!("Routing to CHALLENGE API service for path: {}", uri);
        
    } else if uri.starts_with("/api/billing") {
        // Billing Engine API
        ctx.service_type = ServiceType::BillingApi;
        ctx.upstream_port = 8081;
        info!("Routing to BILLING API service for path: {}", uri);
        
    } else if uri.starts_with("/api/erir") {
        // ERIR Integration API
        ctx.service_type = ServiceType::ErirApi;
        ctx.upstream_port = 8082;
        info!("Routing to ERIR API service for path: {}", uri);
        
    } else if uri.starts_with("/api/shared") || uri.starts_with("/api/tbank") {
        // Shared Services / T-Bank Integration API
        ctx.service_type = ServiceType::SharedApi;
        ctx.upstream_port = 8083;
        info!("Routing to SHARED API service for path: {}", uri);
        
    } else if uri.starts_with("/api/") {
        // Общие API запросы - направляем на Core API балансировщик
        ctx.service_type = ServiceType::CoreApi;
        info!("Routing to CORE API service for path: {}", uri);
        
    } else {
        // Для неопознанных доменов показываем информационную страницу
        ctx.service_type = ServiceType::Static;
        info!("Routing to STATIC page for unknown host: {} (uri: {})", host, uri);
    }
}