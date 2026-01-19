use async_trait::async_trait;
use bytes::Bytes;
use log::info;
use std::sync::Arc;

use pingora::prelude::*;
use pingora::http::ResponseHeader;
use pingora_core::modules::http::{
    grpc_web::{GrpcWeb, GrpcWebBridge},
    HttpModules,
};
use pingora_load_balancing::selection::RoundRobin;

use crate::types::{RequestContext, ServiceType};
use crate::cors::{handle_cors_preflight, add_cors_headers_for_request, add_security_headers};
use crate::routing::{handle_https_redirect, route_request};
use crate::rate_limit::check_rate_limit;
use crate::metrics::*;
use crate::filter::IPFilter;
use crate::config::{Config, ServerBlock, LocationBlock};
use crate::cache::CacheManager;
use crate::circuit_breaker::CircuitBreaker;
use crate::logging::LoggingMiddleware;
use std::time::Duration;

/// Основной прокси для AdQuest
pub struct AdQuestProxy {
    core_api_lb: Arc<LoadBalancer<RoundRobin>>,  // RoundRobin поддерживает веса через Backend.weight
    zitadel_lb: Arc<LoadBalancer<RoundRobin>>,
    config: Arc<Config>,
    cache_manager: Option<Arc<CacheManager>>,
    circuit_breaker: Option<Arc<CircuitBreaker>>,
    logging_middleware: Arc<LoggingMiddleware>,
    ip_filter: Option<Arc<IPFilter>>,
}

impl AdQuestProxy {
    pub fn new(
        core_api_lb: Arc<LoadBalancer<RoundRobin>>,
        zitadel_lb: Arc<LoadBalancer<RoundRobin>>,
        config: Arc<Config>,
        cache_manager: Option<Arc<CacheManager>>,
        circuit_breaker: Option<Arc<CircuitBreaker>>,
        logging_middleware: Arc<LoggingMiddleware>,
        ip_filter: Option<Arc<IPFilter>>,
    ) -> Self {
        Self {
            core_api_lb,
            zitadel_lb,
            config,
            cache_manager,
            circuit_breaker,
            logging_middleware,
            ip_filter,
        }
    }

    fn get_static_html(&self, _uri: &str, _host: &str) -> String {
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Welcome to AdQuest Proxy!</title>
    <style>
        body {
            width: 35em;
            margin: 0 auto;
            font-family: Tahoma, Verdana, Arial, sans-serif;
        }
    </style>
</head>
<body>
    <h1>Welcome to AdQuest Proxy!</h1>
    <p>If you see this page, the AdQuest proxy server is successfully installed and
    working. Further configuration is required.</p>

    <p>For online documentation and support please refer to
    <a href="https://github.com/cloudflare/pingora">Pingora</a>.<br/>
    Commercial support is available at
    <a href="https://www.cloudflare.com/">Cloudflare</a>.</p>

    <p><em>Thank you for using AdQuest Proxy powered by Pingora.</em></p>
</body>
</html>"#.to_string()
    }
}

#[async_trait]
impl ProxyHttp for AdQuestProxy {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new()
    }

    fn init_downstream_modules(&self, modules: &mut HttpModules) {
        // Добавляем gRPC-Web модуль для поддержки gRPC-Web запросов от Zitadel консоли
        modules.add_module(Box::new(GrpcWeb));
    }

    async fn early_request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        // Определяем, является ли это запрос к Zitadel
        let host = session
            .req_header()
            .uri
            .authority()
            .map(|a| a.as_str())
            .or_else(|| {
                session
                    .req_header()
                    .headers
                    .get("host")
                    .and_then(|h| h.to_str().ok())
            })
            .unwrap_or("unknown")
            .to_string();
        
        let host_without_port = host.split(':').next().unwrap_or(&host);
        
        // Инициализируем gRPC-Web модуль для всех запросов к Zitadel
        // Модуль сам определит, является ли запрос gRPC-Web по Content-Type в request_header_filter
        if host_without_port == "auth.ad-quest.ru" || 
           (host_without_port == "localhost" && (host.contains(":8085") || host.contains(":8091"))) {
            if let Some(grpc) = session.downstream_modules_ctx.get_mut::<GrpcWebBridge>() {
                grpc.init();
            }
        }
        Ok(())
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        // IP Filtering - проверяем blacklist/whitelist
        if let Some(ip_filter) = &self.ip_filter {
            if let Some(client_addr) = session.client_addr() {
                let addr_str = client_addr.to_string();
                if let Some(ip_str) = addr_str.split(':').next() {
                    if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
                        if ip_filter.should_block_ip(ip).await {
                            // IP заблокирован, возвращаем 403 Forbidden
                            // Используем respond_error_with_body как в официальных примерах
                            let error_body = r#"{"error":"Forbidden","message":"Access denied"}"#;
                            let _ = session
                                .respond_error_with_body(403, Bytes::from(error_body))
                                .await;
                            
                            return Ok(true);
                        }
                    }
                }
            }
        }

        // Rate limiting - получаем конфигурацию из nginx config
        if let Some(nginx_config) = &self.config.nginx_config {
            let host = session
                .req_header()
                .uri
                .authority()
                .map(|a| a.as_str())
                .or_else(|| {
                    session
                        .req_header()
                        .headers
                        .get("host")
                        .and_then(|h| h.to_str().ok())
                })
                .unwrap_or("unknown");

            let uri = session.req_header().uri.path();

            // Находим соответствующий server и location
            if let Some(server) = nginx_config.find_server(host) {
                if let Some(location) = nginx_config.find_location(server, uri) {
                    if let Some(rate_limit) = &location.rate_limit {
                        // Создаем временную конфигурацию rate limit
                        let rate_config = crate::rate_limit::RateLimitConfig {
                            enabled: true,
                            max_requests_per_second: rate_limit.requests_per_second as isize,
                            whitelist: vec!["127.0.0.1".to_string(), "::1".to_string()],
                            per_api_key_limits: std::collections::HashMap::new(),
                        };

                        if check_rate_limit(session, &rate_config).await? {
                            // Запрос был заблокирован (429), увеличиваем метрику
                            RATE_LIMIT_HITS.inc();
                            return Ok(true);
                        }
                    }
                }
            }
        }

        let uri = session.req_header().uri.path().to_string();
        
        // В HTTP/2 используется :authority псевдо-заголовок, в HTTP/1.1 - Host заголовок
        let host = session
            .req_header()
            .uri
            .authority()
            .map(|a| a.as_str())
            .or_else(|| {
                session
                    .req_header()
                    .headers
                    .get("host")
                    .and_then(|h| h.to_str().ok())
            })
            .unwrap_or("unknown")
            .to_string();

        let host_without_port = host.split(':').next().unwrap_or(&host);
        
        // Логируем все запросы к Zitadel и gRPC-Web запросы для диагностики
        let is_grpc_web = uri.contains("zitadel.") || uri.contains(".v1.") || uri.contains(".v2.");
        let is_zitadel = host_without_port == "auth.ad-quest.ru";
        
        if is_grpc_web || is_zitadel || (!uri.starts_with("/health") && !uri.starts_with("/api/heartbeat")) {
            info!("Request: {} {} (Host: {})", session.req_header().method, uri, host);
            
            // Для gRPC-Web запросов логируем заголовки
            if is_grpc_web {
                if let Some(ct) = session.req_header().headers.get("content-type") {
                    info!("  Content-Type: {:?}", ct.to_str().unwrap_or("invalid"));
                }
                if let Some(origin) = session.req_header().headers.get("origin") {
                    info!("  Origin: {:?}", origin.to_str().unwrap_or("invalid"));
                }
            }
        }

        // Обработка CORS preflight запросов
        if handle_cors_preflight(session, &uri).await? {
            return Ok(true);
        }

        // HTTP -> HTTPS редирект для доменов ad-quest.ru
        if handle_https_redirect(session, &host, &uri).await? {
            return Ok(true);
        }

        // Определяем маршрутизацию
        route_request(&host, &uri, ctx);

        // Обработка статических страниц
        if ctx.service_type == ServiceType::Static {
            let html_content = self.get_static_html(&uri, &host);
            
            let mut response = ResponseHeader::build(200, None)?;
            response.insert_header("Content-Type", "text/html; charset=utf-8")?;
            response.insert_header("Content-Length", html_content.len().to_string())?;
            
            add_security_headers(&mut response)?;

            session.write_response_header(Box::new(response), false).await?;
            session.write_response_body(Some(Bytes::from(html_content)), true).await?;

            return Ok(true);
        }

        Ok(false) // Продолжаем с проксированием
    }

    fn fail_to_connect(
        &self,
        _session: &mut Session,
        _peer: &HttpPeer,
        ctx: &mut Self::CTX,
        e: Box<Error>,
    ) -> Box<Error> {
        const MAX_RETRIES: u32 = 3;

        if ctx.retries < MAX_RETRIES {
            ctx.retries += 1;
            
            let service_name = match ctx.service_type {
                ServiceType::CoreApi => "core_api",
                ServiceType::ChallengeApi => "challenge_api",
                ServiceType::BillingApi => "billing_api",
                ServiceType::ErirApi => "erir_api",
                ServiceType::SharedApi => "shared_api",
                ServiceType::ZitadelAuth => "zitadel_auth",
                ServiceType::Static => "static",
            };
            
            info!(
                "Connection failed, retry attempt {}/{} for service: {}",
                ctx.retries, MAX_RETRIES, service_name
            );
            
            // Метрика retry
            RETRY_ATTEMPTS
                .with_label_values(&[service_name, "attempt"])
                .inc();
            
            let mut retry_e = e;
            retry_e.set_retry(true);
            retry_e
        } else {
            let service_name = match ctx.service_type {
                ServiceType::CoreApi => "core_api",
                ServiceType::ChallengeApi => "challenge_api",
                ServiceType::BillingApi => "billing_api",
                ServiceType::ErirApi => "erir_api",
                ServiceType::SharedApi => "shared_api",
                ServiceType::ZitadelAuth => "zitadel_auth",
                ServiceType::Static => "static",
            };
            
            info!(
                "Max retries ({}) exceeded for service: {}",
                MAX_RETRIES, service_name
            );
            
            // Метрика failed retry
            RETRY_ATTEMPTS
                .with_label_values(&[service_name, "failed"])
                .inc();
            
            e
        }
    }

    async fn upstream_peer(&self, _session: &mut Session, ctx: &mut Self::CTX) -> Result<Box<HttpPeer>> {
        const MAX_SLEEP: Duration = Duration::from_secs(10);

        // Exponential backoff перед retry
        if ctx.retries > 0 {
            // Exponential backoff: 10ms, 100ms, 1s, 10s
            let sleep_ms = std::cmp::min(
                Duration::from_millis(u64::pow(10, ctx.retries)),
                MAX_SLEEP
            );
            
            info!("Sleeping for {:?} before retry attempt {}", sleep_ms, ctx.retries);
            tokio::time::sleep(sleep_ms).await;
        }

        let upstream = match ctx.service_type {
            ServiceType::CoreApi => {
                // Используем select() как в примерах Pingora
                // Arc автоматически разыменовывается при вызове методов через Deref
                let backend = self.core_api_lb.select(b"", 256).unwrap();
                info!("Selected core API backend: {:?}", backend);
                backend
            }
            ServiceType::ZitadelAuth => {
                let backend = self.zitadel_lb.select(b"", 256).unwrap();
                info!("Selected Zitadel backend: {:?}", backend);
                backend
            }
            ServiceType::ChallengeApi => {
                let addr = format!("127.0.0.1:{}", ctx.upstream_port);
                info!("Direct routing to Challenge API: {}", addr);
                return Ok(Box::new(HttpPeer::new(addr, false, "".to_string())));
            }
            ServiceType::BillingApi => {
                let addr = format!("127.0.0.1:{}", ctx.upstream_port);
                info!("Direct routing to Billing API: {}", addr);
                return Ok(Box::new(HttpPeer::new(addr, false, "".to_string())));
            }
            ServiceType::ErirApi => {
                let addr = format!("127.0.0.1:{}", ctx.upstream_port);
                info!("Direct routing to ERIR API: {}", addr);
                return Ok(Box::new(HttpPeer::new(addr, false, "".to_string())));
            }
            ServiceType::SharedApi => {
                let addr = format!("127.0.0.1:{}", ctx.upstream_port);
                info!("Direct routing to Shared API: {}", addr);
                return Ok(Box::new(HttpPeer::new(addr, false, "".to_string())));
            }
            ServiceType::Static => {
                return Err(Error::new(ErrorType::InternalError));
            }
        };

        let peer = Box::new(HttpPeer::new(upstream, false, "".to_string()));
        Ok(peer)
    }

    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        // Добавляем стандартные proxy заголовки
        if let Some(client_ip) = session.client_addr() {
            upstream_request.insert_header("X-Real-IP", client_ip.to_string())?;
            upstream_request.insert_header("X-Forwarded-For", client_ip.to_string())?;
        }

        // Передаем оригинальный Host заголовок
        if let Some(host) = session.req_header().headers.get("host") {
            upstream_request.insert_header("Host", host.to_str().unwrap_or("unknown"))?;
        }

        match ctx.service_type {
            ServiceType::CoreApi | 
            ServiceType::ChallengeApi | ServiceType::BillingApi | 
            ServiceType::ErirApi | ServiceType::SharedApi | ServiceType::ZitadelAuth => {
                // Определяем протокол для upstream запроса
                let upstream_proto = if ctx.service_type == ServiceType::ZitadelAuth {
                    // Для Zitadel используем HTTP для подключения к контейнеру
                    "http"
                } else {
                    if session.req_header().uri.scheme().is_some_and(|s| s == "https") ||
                       session.req_header().headers.get("x-forwarded-proto").is_some_and(|v| v == "https") {
                        "https"
                    } else {
                        "http"
                    }
                };
                
                // Определяем протокол для X-Forwarded-Proto заголовка
                let forwarded_proto = if ctx.service_type == ServiceType::ZitadelAuth {
                    // Для Zitadel всегда передаем https, так как он работает за HTTPS прокси
                    "https"
                } else {
                    upstream_proto
                };
                
                upstream_request.insert_header("X-Forwarded-Proto", forwarded_proto)?;
                
                // Для Zitadel добавляем дополнительные заголовки для правильной генерации URLs
                if ctx.service_type == ServiceType::ZitadelAuth {
                    if let Some(host) = session.req_header().headers.get("host") {
                        upstream_request.insert_header("X-Forwarded-Host", host.to_str().unwrap_or("auth.ad-quest.ru"))?;
                    }
                    
                    // Добавляем X-Forwarded-Port для HTTPS
                    if forwarded_proto == "https" {
                        upstream_request.insert_header("X-Forwarded-Port", "443")?;
                    } else {
                        upstream_request.insert_header("X-Forwarded-Port", "80")?;
                    }
                }
                
                // Поддержка WebSocket
                if let Some(upgrade) = session.req_header().headers.get("upgrade") {
                    upstream_request.insert_header("Upgrade", upgrade.to_str().unwrap_or(""))?;
                    upstream_request.insert_header("Connection", "upgrade")?;
                } else {
                    upstream_request.insert_header("Connection", "close")?;
                }
            }
            ServiceType::Static => {}
        }

        Ok(())
    }

    async fn response_filter(
        &self,
        session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        // Для gRPC-Web запросов проверяем, был ли модуль активирован
        // Если ответ не gRPC (например, 404 JSON), модуль должен быть отключен
        if ctx.service_type == ServiceType::ZitadelAuth {
            if let Some(_grpc) = session.downstream_modules_ctx.get_mut::<GrpcWebBridge>() {
                // Если модуль был активирован, но ответ не gRPC, отключаем его
                let content_type = upstream_response
                    .headers
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                
                if !content_type.starts_with("application/grpc") && 
                   !content_type.starts_with("application/grpc-web") {
                    // Ответ не gRPC, но модуль был активирован - это нормально для ошибок
                    // Модуль сам отключится в response_header_filter
                }
            }
            
            // Zitadel сам управляет CORS заголовками, не добавляем свои
            // Добавляем только security заголовки
            add_security_headers(upstream_response)?;
        } else {
            // Для других сервисов добавляем и security, и CORS заголовки
            add_security_headers(upstream_response)?;
            add_cors_headers_for_request(session, upstream_response)?;
        }

        Ok(())
    }

    async fn logging(
        &self,
        session: &mut Session,
        _e: Option<&Error>,
        ctx: &mut Self::CTX,
    ) {
        let response_code = session
            .response_written()
            .map_or(0, |resp| resp.status.as_u16());

        let service_name = match ctx.service_type {
            ServiceType::CoreApi => "CORE_API",
            ServiceType::ChallengeApi => "CHALLENGE_API",
            ServiceType::BillingApi => "BILLING_API",
            ServiceType::ErirApi => "ERIR_API",
            ServiceType::SharedApi => "SHARED_API",
            ServiceType::ZitadelAuth => "ZITADEL_AUTH",
            ServiceType::Static => "STATIC",
        };

        let service_name_metric = match ctx.service_type {
            ServiceType::CoreApi => "core_api",
            ServiceType::ChallengeApi => "challenge_api",
            ServiceType::BillingApi => "billing_api",
            ServiceType::ErirApi => "erir_api",
            ServiceType::SharedApi => "shared_api",
            ServiceType::ZitadelAuth => "zitadel_auth",
            ServiceType::Static => "static",
        };

        let method = session.req_header().method.as_str();
        let duration = ctx.start_time.elapsed().as_secs_f64();

        // Prometheus метрики
        HTTP_REQUESTS_TOTAL
            .with_label_values(&[method, &response_code.to_string(), service_name_metric])
            .inc();

        HTTP_REQUEST_DURATION.observe(duration);

        let client_addr = session.client_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        info!(
            "[{}] {} {} -> {}, response: {} (duration: {:.3}s, retries: {})",
            service_name,
            session.req_header().method,
            session.req_header().uri,
            client_addr,
            response_code,
            duration,
            ctx.retries
        );
    }
}