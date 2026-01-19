use std::collections::HashMap;
use std::fs;
use std::path::Path;
use regex::Regex;
use log::{info, warn, error};

#[derive(Debug, Clone)]
pub struct NginxConfig {
    pub servers: Vec<ServerBlock>,
    pub upstreams: HashMap<String, UpstreamBlock>,
}

#[derive(Debug, Clone)]
pub struct ServerBlock {
    pub listen_ports: Vec<ListenDirective>,
    pub server_names: Vec<String>,
    pub ssl_certificate: Option<String>,
    pub ssl_certificate_key: Option<String>,
    pub locations: Vec<LocationBlock>,
}

#[derive(Debug, Clone)]
pub struct ListenDirective {
    pub port: u16,
    pub ssl: bool,
    pub http2: bool,
}

#[derive(Debug, Clone)]
pub struct LocationBlock {
    pub path: String,
    pub proxy_pass: Option<String>,
    pub rate_limit: Option<RateLimit>,
    pub cors_enable: bool,
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_second: u32,
    pub burst: u32,
}

#[derive(Debug, Clone)]
pub struct UpstreamBlock {
    pub name: String,
    pub servers: Vec<UpstreamServer>,
}

#[derive(Debug, Clone)]
pub struct UpstreamServer {
    pub address: String,
    pub weight: u32,
}

impl NginxConfig {
    /// Загружает все конфиги из директории sites-enabled
    pub fn load_from_sites_enabled<P: AsRef<Path>>(sites_enabled_dir: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut servers = Vec::new();
        let mut upstreams = HashMap::new();

        let dir = fs::read_dir(sites_enabled_dir)?;
        
        for entry in dir {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                match Self::parse_config_file(&path) {
                    Ok(mut config) => {
                        info!("Loaded config from: {}", path.display());
                        servers.extend(config.servers);
                        upstreams.extend(config.upstreams);
                    }
                    Err(e) => {
                        error!("Failed to parse config {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(NginxConfig { servers, upstreams })
    }

    /// Парсит один конфигурационный файл
    pub fn parse_config_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        Self::parse_config_content(&content)
    }

    /// Парсит содержимое конфига
    pub fn parse_config_content(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut servers = Vec::new();
        let mut upstreams = HashMap::new();

        // Удаляем комментарии
        let content = Self::remove_comments(content);
        
        // Парсим server блоки
        let server_regex = Regex::new(r"server\s*\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}")?;
        for cap in server_regex.captures_iter(&content) {
            if let Some(server_content) = cap.get(1) {
                match Self::parse_server_block(server_content.as_str()) {
                    Ok(server) => servers.push(server),
                    Err(e) => warn!("Failed to parse server block: {}", e),
                }
            }
        }

        // Парсим upstream блоки
        let upstream_regex = Regex::new(r"upstream\s+(\w+)\s*\{([^{}]*)\}")?;
        for cap in upstream_regex.captures_iter(&content) {
            if let (Some(name), Some(upstream_content)) = (cap.get(1), cap.get(2)) {
                match Self::parse_upstream_block(name.as_str(), upstream_content.as_str()) {
                    Ok(upstream) => {
                        upstreams.insert(upstream.name.clone(), upstream);
                    }
                    Err(e) => warn!("Failed to parse upstream block {}: {}", name.as_str(), e),
                }
            }
        }

        Ok(NginxConfig { servers, upstreams })
    }

    /// Удаляет комментарии из конфига
    fn remove_comments(content: &str) -> String {
        let comment_regex = Regex::new(r"#.*$").unwrap();
        content.lines()
            .map(|line| comment_regex.replace(line, "").trim().to_string())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Парсит server блок
    fn parse_server_block(content: &str) -> Result<ServerBlock, Box<dyn std::error::Error>> {
        let mut listen_ports = Vec::new();
        let mut server_names = Vec::new();
        let mut ssl_certificate = None;
        let mut ssl_certificate_key = None;
        let mut locations = Vec::new();

        // Парсим listen директивы
        let listen_regex = Regex::new(r"listen\s+([^;]+);")?;
        for cap in listen_regex.captures_iter(content) {
            if let Some(listen_str) = cap.get(1) {
                if let Ok(listen) = Self::parse_listen_directive(listen_str.as_str()) {
                    listen_ports.push(listen);
                }
            }
        }

        // Парсим server_name
        let server_name_regex = Regex::new(r"server_name\s+([^;]+);")?;
        if let Some(cap) = server_name_regex.captures(content) {
            if let Some(names_str) = cap.get(1) {
                server_names = names_str.as_str()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        // Парсим SSL сертификаты
        let ssl_cert_regex = Regex::new(r"ssl_certificate\s+([^;]+);")?;
        if let Some(cap) = ssl_cert_regex.captures(content) {
            ssl_certificate = cap.get(1).map(|m| m.as_str().to_string());
        }

        let ssl_key_regex = Regex::new(r"ssl_certificate_key\s+([^;]+);")?;
        if let Some(cap) = ssl_key_regex.captures(content) {
            ssl_certificate_key = cap.get(1).map(|m| m.as_str().to_string());
        }

        // Парсим location блоки
        let location_regex = Regex::new(r"location\s+([^\s{]+)\s*\{([^{}]*)\}")?;
        for cap in location_regex.captures_iter(content) {
            if let (Some(path), Some(location_content)) = (cap.get(1), cap.get(2)) {
                match Self::parse_location_block(path.as_str(), location_content.as_str()) {
                    Ok(location) => locations.push(location),
                    Err(e) => warn!("Failed to parse location block {}: {}", path.as_str(), e),
                }
            }
        }

        Ok(ServerBlock {
            listen_ports,
            server_names,
            ssl_certificate,
            ssl_certificate_key,
            locations,
        })
    }

    /// Парсит listen директиву
    fn parse_listen_directive(listen_str: &str) -> Result<ListenDirective, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = listen_str.split_whitespace().collect();
        let port_str = parts[0];
        
        let port = port_str.parse::<u16>()?;
        let ssl = parts.contains(&"ssl");
        let http2 = parts.contains(&"http2");

        Ok(ListenDirective { port, ssl, http2 })
    }

    /// Парсит location блок
    fn parse_location_block(path: &str, content: &str) -> Result<LocationBlock, Box<dyn std::error::Error>> {
        let mut proxy_pass = None;
        let mut rate_limit = None;
        let mut cors_enable = false;

        // Парсим proxy_pass
        let proxy_pass_regex = Regex::new(r"proxy_pass\s+([^;]+);")?;
        if let Some(cap) = proxy_pass_regex.captures(content) {
            proxy_pass = cap.get(1).map(|m| m.as_str().to_string());
        }

        // Парсим rate_limit
        let rate_limit_regex = Regex::new(r"rate_limit\s+(\d+)\s+(\d+);")?;
        if let Some(cap) = rate_limit_regex.captures(content) {
            if let (Some(rps), Some(burst)) = (cap.get(1), cap.get(2)) {
                if let (Ok(rps_val), Ok(burst_val)) = (rps.as_str().parse::<u32>(), burst.as_str().parse::<u32>()) {
                    rate_limit = Some(RateLimit {
                        requests_per_second: rps_val,
                        burst: burst_val,
                    });
                }
            }
        }

        // Проверяем cors_enable
        cors_enable = content.contains("cors_enable");

        Ok(LocationBlock {
            path: path.to_string(),
            proxy_pass,
            rate_limit,
            cors_enable,
        })
    }

    /// Парсит upstream блок
    fn parse_upstream_block(name: &str, content: &str) -> Result<UpstreamBlock, Box<dyn std::error::Error>> {
        let mut servers = Vec::new();

        let server_regex = Regex::new(r"server\s+([^;]+);")?;
        for cap in server_regex.captures_iter(content) {
            if let Some(server_str) = cap.get(1) {
                let parts: Vec<&str> = server_str.as_str().split_whitespace().collect();
                let address = parts[0].to_string();
                let weight = 1; // По умолчанию вес 1, можно расширить парсинг

                servers.push(UpstreamServer { address, weight });
            }
        }

        Ok(UpstreamBlock {
            name: name.to_string(),
            servers,
        })
    }

    /// Находит server блок по host
    pub fn find_server(&self, host: &str) -> Option<&ServerBlock> {
        let host_without_port = host.split(':').next().unwrap_or(host);
        
        self.servers.iter().find(|server| {
            server.server_names.iter().any(|name| name == host_without_port)
        })
    }

    /// Находит location в server блоке по пути
    pub fn find_location<'a>(&self, server: &'a ServerBlock, path: &str) -> Option<&'a LocationBlock> {
        // Сначала ищем точное совпадение
        for location in &server.locations {
            if location.path == path {
                return Some(location);
            }
        }

        // Затем ищем по префиксу (самый длинный префикс)
        let mut best_match: Option<&LocationBlock> = None;
        let mut best_match_len = 0;

        for location in &server.locations {
            if location.path.ends_with('/') || location.path == "/" {
                let prefix = location.path.trim_end_matches('/');
                if path.starts_with(prefix) && prefix.len() > best_match_len {
                    best_match = Some(location);
                    best_match_len = prefix.len();
                }
            }
        }

        best_match
    }

    /// Получает upstream по имени
    pub fn get_upstream(&self, name: &str) -> Option<&UpstreamBlock> {
        self.upstreams.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_config() {
        let config_content = r#"
            server {
                listen 80;
                server_name example.com;
                
                location / {
                    proxy_pass backend;
                    rate_limit 10 20;
                    cors_enable;
                }
            }
            
            upstream backend {
                server 127.0.0.1:8080;
                server 127.0.0.1:8081;
            }
        "#;

        let config = NginxConfig::parse_config_content(config_content).unwrap();
        
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.upstreams.len(), 1);
        
        let server = &config.servers[0];
        assert_eq!(server.server_names, vec!["example.com"]);
        assert_eq!(server.locations.len(), 1);
        
        let location = &server.locations[0];
        assert_eq!(location.path, "/");
        assert_eq!(location.proxy_pass, Some("backend".to_string()));
        assert!(location.cors_enable);
        
        let upstream = config.upstreams.get("backend").unwrap();
        assert_eq!(upstream.servers.len(), 2);
    }
}