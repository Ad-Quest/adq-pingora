use env_logger;
use log::info;
use std::time::Duration;
use std::sync::Arc;
use clap::{Arg, Command};

use pingora_core::server::configuration::Opt;
use pingora_core::server::Server;
use pingora_core::services::background::background_service;
use pingora_load_balancing::{
    health_check::TcpHealthCheck,
    LoadBalancer,
};
use pingora_proxy::http_proxy_service;

mod proxy;
mod routing;
mod cors;
mod ssl;
mod types;
mod rate_limit;
mod metrics;
mod filter;
mod config;
mod cache;
mod circuit_breaker;
mod logging;

use proxy::AdQuestProxy;
use config::Config;
use cache::CacheManager;
use circuit_breaker::CircuitBreaker;
use logging::{init_logging, LoggingMiddleware};
use filter::IPFilter;
use metrics::init_metrics;

fn main() {
    // Парсим аргументы командной строки
    let matches = Command::new("adqp")
        .version(env!("CARGO_PKG_VERSION"))
        .about("ADQ Pingora - High-performance HTTP/HTTPS proxy server")
        .arg(Arg::new("test")
            .short('t')
            .long("test")
            .help("Test configuration and exit")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("config")
            .short('c')
            .long("config")
            .value_name("FILE")
            .help("Configuration file path")
            .default_value("/etc/adq-pingora/proxy.yaml"))
        .get_matches();

    // Если запрошена проверка конфигурации
    if matches.get_flag("test") {
        // Инициализируем базовое логирование только для тестирования
        env_logger::init();
        let config_path = matches.get_one::<String>("config").unwrap();
        test_configuration(config_path);
        return;
    }

    // Читаем аргументы командной строки для Pingora
    let opt = Opt::parse_args();
    let mut server = Server::new(Some(opt)).unwrap();
    server.bootstrap();

    // Загружаем основную конфигурацию
    let config_path = matches.get_one::<String>("config").unwrap();
    let config = Arc::new(Config::load_from_file(config_path).unwrap_or_else(|error| {
        eprintln!("Failed to load config from {config_path}: {error}");
        std::process::exit(1);
    }));

    // Инициализируем структурированное логирование
    if let Err(e) = init_logging(&config.logging) {
        eprintln!("Failed to initialize logging: {}, falling back to env_logger", e);
        env_logger::init();
    }

    info!("Starting ADQ Pingora v{}...", env!("CARGO_PKG_VERSION"));

    // Инициализируем Prometheus метрики
    init_metrics();

    // Создаем менеджер кеширования
    let cache_manager = if config.cache.enabled {
        match CacheManager::new(config.cache.clone()) {
            Ok(manager) => {
                info!("Cache manager initialized with {} rules", config.cache.rules.len());
                Some(Arc::new(manager))
            }
            Err(e) => {
                log::error!("Failed to initialize cache manager: {}", e);
                None
            }
        }
    } else {
        info!("Caching is disabled");
        None
    };

    // Создаем Circuit Breaker
    let circuit_breaker = if config.circuit_breaker.enabled {
        info!("Circuit breaker initialized with failure threshold: {}", 
              config.circuit_breaker.failure_threshold);
        Some(Arc::new(CircuitBreaker::new(config.circuit_breaker.clone())))
    } else {
        info!("Circuit breaker is disabled");
        None
    };

    // Создаем middleware для логирования
    let logging_middleware = Arc::new(LoggingMiddleware::new(config.logging.clone()));

    // Создаем IP фильтр
    let ip_filter = if config.ip_filter.enabled {
        let filter = Arc::new(IPFilter::new());
        
        // Загружаем whitelist и blacklist в блокирующем контексте
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Загружаем whitelist
            if let Some(whitelist) = &config.ip_filter.whitelist {
                for ip_str in whitelist {
                    if let Ok(ip) = ip_str.parse() {
                        filter.add_to_whitelist(ip).await;
                    }
                }
            }

            // Загружаем blacklist из файла
            if let Some(blacklist_file) = &config.ip_filter.blacklist_file {
                if let Err(e) = filter.load_blacklist_from_file(blacklist_file).await {
                    log::warn!("Failed to load blacklist file '{}': {}", blacklist_file, e);
                }
            }
        });

        info!("IP filter initialized");
        Some(filter)
    } else {
        info!("IP filtering is disabled");
        None
    };

    // Создаем load balancers на основе nginx-style конфигурации
    let mut load_balancers = std::collections::HashMap::new();

    if let Some(nginx_config) = &config.nginx_config {
        for (upstream_name, upstream_block) in &nginx_config.upstreams {
            info!("Creating load balancer for upstream: {}", upstream_name);

            // Собираем адреса серверов
            let addresses: Vec<String> = upstream_block.servers
                .iter()
                .map(|s| s.address.clone())
                .collect();

            let mut lb = LoadBalancer::try_from_iter(addresses.iter().map(|s| s.as_str()))
                .unwrap_or_else(|e| {
                    log::error!("Failed to create load balancer for '{}': {}", upstream_name, e);
                    std::process::exit(1);
                });

            // Настраиваем health checks (по умолчанию TCP)
            let hc = TcpHealthCheck::new();
            lb.set_health_check(hc);
            lb.health_check_frequency = Some(Duration::from_secs(config.global.health_check_interval));
            
            info!("TCP health check configured for '{}'", upstream_name);
            load_balancers.insert(upstream_name.clone(), lb);
        }
    } else {
        log::warn!("No nginx configuration found in sites-enabled/");
        log::info!("Please create configuration files in sites-available/ and link them to sites-enabled/");
    }

    // Создаем background сервисы для health checks
    let mut background_services = Vec::new();
    let mut lb_handles = std::collections::HashMap::new();

    for (upstream_name, lb) in load_balancers {
        let bg_service = background_service(
            &format!("{} health check", upstream_name), 
            lb
        );
        let lb_handle = bg_service.task();
        lb_handles.insert(upstream_name, lb_handle);
        background_services.push(bg_service);
    }

    let proxy = AdQuestProxy::new(
        lb_handles,
        config.clone(),
        cache_manager,
        circuit_breaker,
        logging_middleware,
        ip_filter,
    );

    let mut proxy_service = http_proxy_service(&server.configuration, proxy);
    
    let nginx_config = config.nginx_config.as_ref().expect("site configuration was loaded");
    let listeners = ssl::listener_settings(nginx_config).unwrap_or_else(|error| {
        eprintln!("Invalid listener configuration: {error}");
        std::process::exit(1);
    });
    for (port, tls) in listeners {
        let addr = format!("0.0.0.0:{port}");
        if let Some(tls) = tls {
            proxy_service.add_tls_with_settings(&addr, None, tls);
        } else {
            proxy_service.add_tcp(&addr);
        }
    }

    // Добавляем все сервисы в сервер
    for bg_service in background_services {
        server.add_service(bg_service);
    }
    
    server.add_service(proxy_service);

    // Добавляем Prometheus metrics сервис если включен
    if config.logging.metrics.enabled {
        let mut prometheus_service = pingora_prometheus::prometheus_http_service();
        prometheus_service.add_tcp(&format!("127.0.0.1:{}", config.logging.metrics.port));
        server.add_service(prometheus_service);
        info!("Prometheus metrics service started on port {}", config.logging.metrics.port);
    }

    info!("ADQ Pingora started successfully!");
    
    if let Some(nginx_config) = &config.nginx_config {
        info!("Configuration loaded: {} servers, {} upstreams", 
              nginx_config.servers.len(), nginx_config.upstreams.len());
        
        // Выводим информацию о настроенных серверах
        for server in &nginx_config.servers {
            let server_names = server.server_names.join(", ");
            let ports: Vec<String> = server.listen_ports.iter()
                .map(|p| format!("{}{}", p.port, if p.ssl { " (SSL)" } else { "" }))
                .collect();
            
            info!("Server '{}' listening on ports: {}", server_names, ports.join(", "));
            
            for location in &server.locations {
                let rate_info = if let Some(rate) = &location.rate_limit {
                    format!(" (Rate limit: {} req/s, burst: {})", rate.requests_per_second, rate.burst)
                } else {
                    String::new()
                };
                
                info!("  {} -> {}{}", 
                      location.path, 
                      location.proxy_pass.as_deref().unwrap_or("no upstream"),
                      rate_info);
            }
        }
    } else {
        info!("No server configurations loaded from sites-enabled/");
    }

    server.run_forever();
}

/// Функция проверки конфигурации (как nginx -t)
fn test_configuration(config_path: &str) {
    println!("adq-pingora: testing configuration file...");
    
    let mut errors = 0;
    let mut warnings = 0;

    // Проверяем основную конфигурацию
    match Config::load_from_file(config_path) {
        Ok(config) => {
            println!("adq-pingora: configuration file {} syntax is ok", config_path);
            
            // Проверяем nginx-style конфигурацию
            if let Some(nginx_config) = &config.nginx_config {
                println!("adq-pingora: found {} server(s) and {} upstream(s)", 
                         nginx_config.servers.len(), 
                         nginx_config.upstreams.len());

                if let Err(error) = ssl::listener_settings(nginx_config) {
                    println!("adq-pingora: [error] {error}");
                    errors += 1;
                }
                for upstream in nginx_config.upstreams.values() {
                    let addresses = upstream.servers.iter().map(|server| server.address.as_str());
                    if let Err(error) = LoadBalancer::<pingora_load_balancing::selection::RoundRobin>::try_from_iter(addresses) {
                        println!("adq-pingora: [error] upstream {}: {error}", upstream.name);
                        errors += 1;
                    }
                }
            } else {
                println!("adq-pingora: [warn] no server configurations found in sites-enabled/");
                warnings += 1;
            }

            // Проверяем директории
            let sites_enabled = Config::sites_enabled_dir(std::path::Path::new(config_path));
            if !sites_enabled.exists() {
                println!("adq-pingora: [warn] sites-enabled directory not found");
                warnings += 1;
            } else {
                let count = std::fs::read_dir(sites_enabled)
                    .map(|entries| entries.count())
                    .unwrap_or(0);
                println!("adq-pingora: found {} enabled site(s)", count);
            }

            // Проверяем права на порты
            if std::env::var("USER").unwrap_or_default() != "root" {
                println!("adq-pingora: [warn] not running as root, may not be able to bind to ports 80/443");
                warnings += 1;
            }

        }
        Err(e) => {
            println!("adq-pingora: [error] configuration file {} test failed: {}", config_path, e);
            errors += 1;
        }
    }

    // Выводим результат
    if errors > 0 {
        println!("adq-pingora: configuration file {} test failed", config_path);
        std::process::exit(1);
    } else if warnings > 0 {
        println!("adq-pingora: configuration file {} test is successful (with {} warning(s))", config_path, warnings);
    } else {
        println!("adq-pingora: configuration file {} test is successful", config_path);
    }
}