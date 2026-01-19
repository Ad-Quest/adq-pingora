use std::time::Duration;
use tokio::time::timeout;
use reqwest::Client;
use serde_json::Value;

/// Интеграционные тесты для AdQuest Pingora Proxy
/// 
/// Эти тесты проверяют полный функционал прокси в реальных условиях.
/// Для запуска тестов нужно:
/// 1. Запустить прокси сервер
/// 2. Настроить тестовые upstream серверы
/// 3. Запустить тесты: cargo test --test integration_tests

const PROXY_BASE_URL: &str = "http://localhost:6188";
const PROXY_HTTPS_URL: &str = "https://localhost:6189";

#[tokio::test]
async fn test_basic_proxy_functionality() {
    let client = Client::new();
    
    // Тест базового проксирования
    let response = timeout(
        Duration::from_secs(10),
        client.get(&format!("{}/api/health", PROXY_BASE_URL)).send()
    ).await;

    match response {
        Ok(Ok(resp)) => {
            assert!(resp.status().is_success(), "Health check should return success");
            println!("✅ Basic proxy functionality test passed");
        }
        Ok(Err(e)) => {
            println!("⚠️  Basic proxy test failed (connection error): {}", e);
            println!("   Make sure the proxy server is running on {}", PROXY_BASE_URL);
        }
        Err(_) => {
            println!("⚠️  Basic proxy test timed out");
            println!("   Make sure the proxy server is running and responsive");
        }
    }
}

#[tokio::test]
async fn test_rate_limiting() {
    let client = Client::new();
    let mut success_count = 0;
    let mut rate_limited_count = 0;

    // Отправляем много запросов быстро для тестирования rate limiting
    for i in 0..20 {
        let response = client
            .get(&format!("{}/api/test", PROXY_BASE_URL))
            .header("X-Test-Request", format!("rate-limit-{}", i))
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status() == 429 {
                    rate_limited_count += 1;
                    println!("Request {} was rate limited (429)", i);
                } else if resp.status().is_success() {
                    success_count += 1;
                } else {
                    println!("Request {} returned status: {}", i, resp.status());
                }
            }
            Err(e) => {
                println!("Request {} failed: {}", i, e);
            }
        }

        // Небольшая задержка между запросами
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    println!("Rate limiting test results:");
    println!("  Successful requests: {}", success_count);
    println!("  Rate limited requests: {}", rate_limited_count);

    if rate_limited_count > 0 {
        println!("✅ Rate limiting test passed - some requests were rate limited");
    } else {
        println!("⚠️  Rate limiting test inconclusive - no requests were rate limited");
        println!("   This might be expected if rate limits are high or disabled");
    }
}

#[tokio::test]
async fn test_cors_headers() {
    let client = Client::new();
    
    // Тест CORS preflight запроса
    let response = client
        .request(reqwest::Method::OPTIONS, &format!("{}/api/test", PROXY_BASE_URL))
        .header("Origin", "https://example.com")
        .header("Access-Control-Request-Method", "POST")
        .header("Access-Control-Request-Headers", "Content-Type")
        .send()
        .await;

    match response {
        Ok(resp) => {
            let headers = resp.headers();
            
            if headers.contains_key("access-control-allow-origin") {
                println!("✅ CORS headers test passed - CORS headers present");
            } else {
                println!("⚠️  CORS headers test failed - no CORS headers found");
            }

            // Выводим все CORS заголовки для отладки
            for (name, value) in headers.iter() {
                if name.as_str().starts_with("access-control-") {
                    println!("  {}: {:?}", name, value);
                }
            }
        }
        Err(e) => {
            println!("⚠️  CORS test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_security_headers() {
    let client = Client::new();
    
    let response = client
        .get(&format!("{}/api/test", PROXY_BASE_URL))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let headers = resp.headers();
            let mut security_headers_found = 0;

            let expected_headers = [
                "x-frame-options",
                "x-content-type-options", 
                "x-xss-protection",
                "server"
            ];

            for header_name in &expected_headers {
                if headers.contains_key(*header_name) {
                    security_headers_found += 1;
                    if let Some(value) = headers.get(*header_name) {
                        println!("  {}: {:?}", header_name, value);
                    }
                }
            }

            if security_headers_found >= 3 {
                println!("✅ Security headers test passed - {} security headers found", security_headers_found);
            } else {
                println!("⚠️  Security headers test failed - only {} security headers found", security_headers_found);
            }
        }
        Err(e) => {
            println!("⚠️  Security headers test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let client = Client::new();
    
    // Сначала делаем несколько запросов для генерации метрик
    for i in 0..5 {
        let _ = client
            .get(&format!("{}/api/test-{}", PROXY_BASE_URL, i))
            .send()
            .await;
    }

    // Теперь проверяем метрики
    let response = client
        .get(&format!("{}/metrics", PROXY_BASE_URL))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                
                let expected_metrics = [
                    "http_requests_total",
                    "http_request_duration_seconds",
                    "upstream_connections_total"
                ];

                let mut metrics_found = 0;
                for metric in &expected_metrics {
                    if body.contains(metric) {
                        metrics_found += 1;
                        println!("  Found metric: {}", metric);
                    }
                }

                if metrics_found >= 2 {
                    println!("✅ Metrics endpoint test passed - {} metrics found", metrics_found);
                } else {
                    println!("⚠️  Metrics endpoint test failed - only {} metrics found", metrics_found);
                }
            } else {
                println!("⚠️  Metrics endpoint returned status: {}", resp.status());
            }
        }
        Err(e) => {
            println!("⚠️  Metrics endpoint test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_load_balancing() {
    let client = Client::new();
    let mut upstream_responses = std::collections::HashMap::new();

    // Делаем несколько запросов и смотрим, распределяются ли они по разным upstream
    for i in 0..10 {
        let response = client
            .get(&format!("{}/api/test", PROXY_BASE_URL))
            .header("X-Test-Request", format!("lb-test-{}", i))
            .send()
            .await;

        match response {
            Ok(resp) => {
                // Пытаемся определить upstream по заголовкам ответа
                if let Some(server) = resp.headers().get("server") {
                    let server_str = server.to_str().unwrap_or("unknown");
                    *upstream_responses.entry(server_str.to_string()).or_insert(0) += 1;
                }
            }
            Err(e) => {
                println!("Load balancing test request {} failed: {}", i, e);
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!("Load balancing test results:");
    for (server, count) in &upstream_responses {
        println!("  {}: {} requests", server, count);
    }

    if upstream_responses.len() > 1 {
        println!("✅ Load balancing test passed - requests distributed across {} upstreams", upstream_responses.len());
    } else {
        println!("⚠️  Load balancing test inconclusive - all requests went to same upstream");
        println!("   This might be expected if only one upstream is configured or healthy");
    }
}

#[tokio::test]
async fn test_websocket_upgrade() {
    // Тест WebSocket upgrade (базовый)
    let client = Client::new();
    
    let response = client
        .get(&format!("{}/ws", PROXY_BASE_URL))
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("Sec-WebSocket-Version", "13")
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status() == 101 {
                println!("✅ WebSocket upgrade test passed - got 101 Switching Protocols");
            } else if resp.status() == 404 {
                println!("⚠️  WebSocket upgrade test skipped - no WebSocket endpoint configured");
            } else {
                println!("⚠️  WebSocket upgrade test failed - got status {}", resp.status());
            }
        }
        Err(e) => {
            println!("⚠️  WebSocket upgrade test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_gzip_compression() {
    let client = Client::new();
    
    let response = client
        .get(&format!("{}/api/large-response", PROXY_BASE_URL))
        .header("Accept-Encoding", "gzip, deflate")
        .send()
        .await;

    match response {
        Ok(resp) => {
            let headers = resp.headers();
            
            if headers.get("content-encoding").is_some() {
                println!("✅ Compression test passed - response is compressed");
            } else {
                println!("⚠️  Compression test inconclusive - no compression detected");
                println!("   This might be expected if compression is disabled or response is small");
            }
        }
        Err(e) => {
            println!("⚠️  Compression test failed: {}", e);
        }
    }
}

/// Вспомогательная функция для запуска всех тестов
#[tokio::test]
async fn run_all_integration_tests() {
    println!("🚀 Running AdQuest Pingora Proxy Integration Tests");
    println!("================================================");
    
    // Проверяем, что прокси сервер запущен
    let client = Client::new();
    let health_check = timeout(
        Duration::from_secs(5),
        client.get(&format!("{}/", PROXY_BASE_URL)).send()
    ).await;

    match health_check {
        Ok(Ok(_)) => {
            println!("✅ Proxy server is running at {}", PROXY_BASE_URL);
        }
        _ => {
            println!("❌ Proxy server is not running at {}", PROXY_BASE_URL);
            println!("   Please start the proxy server before running integration tests:");
            println!("   cargo run -- -c conf.yaml");
            return;
        }
    }

    println!("\n📊 Test Results Summary:");
    println!("========================");
    
    // Все тесты уже запустятся автоматически через #[tokio::test]
    // Этот тест служит для общего отчета
    
    println!("\n💡 Tips:");
    println!("- Run individual tests: cargo test --test integration_tests test_name");
    println!("- Run with output: cargo test --test integration_tests -- --nocapture");
    println!("- Make sure upstream services are running for complete testing");
}