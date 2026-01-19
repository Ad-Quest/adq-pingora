use pingora::prelude::*;
use pingora::http::ResponseHeader;
use log::info;

/// Обрабатывает CORS preflight запросы
pub async fn handle_cors_preflight(session: &mut Session, uri: &str) -> Result<bool> {
    if session.req_header().method != "OPTIONS" {
        return Ok(false);
    }

    let mut response = ResponseHeader::build(200, None)?;
    add_cors_headers_for_request(session, &mut response)?;
    
    // Для gRPC-Web запросов добавляем специальные заголовки
    if let Some(request_headers) = session.req_header().headers.get("access-control-request-headers") {
        let requested_headers = request_headers.to_str().unwrap_or("");
        if requested_headers.contains("grpc") || requested_headers.contains("x-grpc") {
            response.insert_header("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Requested-With, Accept, Origin, X-CSRF-Token, X-Grpc-Web, X-User-Agent, grpc-timeout, X-Grpc-Web-Protocol")?;
        }
    }
    
    response.insert_header("Access-Control-Max-Age", "86400")?;
    response.insert_header("Content-Length", "0")?;
    response.insert_header("Server", "Pingora/0.6.0")?;
    
    session.write_response_header(Box::new(response), false).await?;
    session.write_response_body(None, true).await?;
    
    info!("CORS preflight response sent for: {}", uri);
    Ok(true)
}

/// Добавляет CORS заголовки к ответу на основе Origin запроса
/// Не добавляет заголовки, если они уже есть (например, от Zitadel)
pub fn add_cors_headers_for_request(session: &Session, response: &mut ResponseHeader) -> Result<()> {
    // Проверяем, есть ли уже CORS заголовки от upstream (например, от Zitadel)
    // Если есть, не добавляем свои, чтобы не конфликтовать
    if response.headers.contains_key("access-control-allow-origin") {
        // CORS заголовки уже установлены upstream, не перезаписываем
        return Ok(());
    }
    
    // Получаем Origin из запроса
    let origin = session
        .req_header()
        .headers
        .get("origin")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Разрешенные домены для CORS
    let allowed_origins = [
        "https://auth.ad-quest.ru", 
        "https://api.ad-quest.ru",
        "http://localhost:3000",  // для разработки
        "http://localhost:5173",  // для Vite dev server
        "http://localhost:8085",  // для Zitadel (старый порт)
        "http://localhost:8091",  // для Zitadel (новый порт)
    ];

    // Проверяем, разрешен ли Origin
    if allowed_origins.contains(&origin) {
        response.insert_header("Access-Control-Allow-Origin", origin)?;
        response.insert_header("Access-Control-Allow-Credentials", "true")?;
    } else if origin.is_empty() {
        // Если Origin не указан (например, same-origin запросы), разрешаем auth.ad-quest.ru
        response.insert_header("Access-Control-Allow-Origin", "https://auth.ad-quest.ru")?;
        response.insert_header("Access-Control-Allow-Credentials", "true")?;
    } else {
        // Для неразрешенных доменов используем wildcard без credentials
        response.insert_header("Access-Control-Allow-Origin", "*")?;
    }

    response.insert_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS, PATCH")?;
    response.insert_header("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Requested-With, Accept, Origin, X-CSRF-Token, X-Grpc-Web, X-User-Agent, grpc-timeout, X-Grpc-Web-Protocol")?;
    response.insert_header("Access-Control-Expose-Headers", "grpc-status, grpc-message, grpc-encoding, grpc-accept-encoding, Grpc-Status, Grpc-Message")?;
    response.insert_header("Vary", "Origin")?;
    
    Ok(())
}

/// Добавляет CORS заголовки к ответу (упрощенная версия для обратной совместимости)
pub fn add_cors_headers(response: &mut ResponseHeader) -> Result<()> {
    // Для обратной совместимости - используем wildcard для простых запросов
    response.insert_header("Access-Control-Allow-Origin", "*")?;
    response.insert_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS, PATCH")?;
    response.insert_header("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Requested-With, Accept, Origin, X-CSRF-Token, X-Grpc-Web, X-User-Agent, grpc-timeout, X-Grpc-Web-Protocol")?;
    response.insert_header("Access-Control-Expose-Headers", "grpc-status, grpc-message, grpc-encoding, grpc-accept-encoding, Grpc-Status, Grpc-Message")?;
    Ok(())
}

/// Добавляет security заголовки
pub fn add_security_headers(response: &mut ResponseHeader) -> Result<()> {
    response.insert_header("X-Frame-Options", "SAMEORIGIN")?;
    response.insert_header("X-Content-Type-Options", "nosniff")?;
    response.insert_header("X-XSS-Protection", "1; mode=block")?;
    response.insert_header("Referrer-Policy", "strict-origin-when-cross-origin")?;
    
    // Добавляем расширенную CSP политику для Zitadel
    // Разрешаем HTTPS и HTTP для auth.ad-quest.ru (для .well-known endpoints)
    response.insert_header("Content-Security-Policy", 
        "default-src 'self'; \
         connect-src 'self' https://auth.ad-quest.ru http://auth.ad-quest.ru https://api.ad-quest.ru wss://auth.ad-quest.ru wss://api.ad-quest.ru https: wss:; \
         script-src 'self' 'unsafe-eval' 'unsafe-inline'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' https://auth.ad-quest.ru data: blob: https:; \
         font-src 'self' data:; \
         frame-src 'none'; \
         object-src 'none'; \
         media-src 'none'; \
         manifest-src 'self'")?;
    
    response.insert_header("Server", "Pingora/0.6.0")?;
    Ok(())
}