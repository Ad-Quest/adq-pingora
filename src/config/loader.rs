use super::types::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load_from_directory<P: AsRef<Path>>(config_dir: P) -> Result<ProxyConfig, Box<dyn std::error::Error>> {
        let mut config = ProxyConfig::default();
        
        // Загружаем конфиги из sites-enabled (как nginx)
        let sites_enabled = config_dir.as_ref().join("sites-enabled");
        if sites_enabled.exists() {
            for entry in fs::read_dir(sites_enabled)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_file() {
                    let server_config = Self::parse_server_config(&path)?;
                    config.servers.push(server_config);
                }
            }
        }
        
        Ok(config)
    }
    
    fn parse_server_config<P: AsRef<Path>>(path: P) -> Result<ServerConfig, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        
        // Простой парсер конфигурации (можно расширить)
        let mut server_name = String::new();
        let mut listen_http = None;
        let mut listen_https = None;
        let mut ssl_cert = None;
        let mut ssl_key = None;
        let mut locations = Vec::new();
        
        for line in content.lines() {
            let line = line.trim();
            
            if line.starts_with("server_name ") {
                server_name = line.replace("server_name ", "").replace(";", "").trim().to_string();
            } else if line.starts_with("listen 80") {
                listen_http = Some(80);
            } else if line.starts_with("listen 443") {
                listen_https = Some(443);
            } else if line.starts_with("ssl_certificate ") {
                ssl_cert = Some(line.replace("ssl_certificate ", "").replace(";", "").trim().to_string());
            } else if line.starts_with("ssl_certificate_key ") {
                ssl_key = Some(line.replace("ssl_certificate_key ", "").replace(";", "").trim().to_string());
            } else if line.starts_with("location ") {
                // Простая обработка location блоков
                let path = line.replace("location ", "").replace(" {", "").trim().to_string();
                locations.push(LocationConfig {
                    path,
                    upstream: "default".to_string(), // Будет определяться из proxy_pass
                    rate_limit_rps: None,
                    rate_limit_burst: None,
                    enable_cors: true,
                });
            }
        }
        
        Ok(ServerConfig {
            server_name,
            listen_http,
            listen_https,
            ssl_cert,
            ssl_key,
            locations,
        })
    }
}