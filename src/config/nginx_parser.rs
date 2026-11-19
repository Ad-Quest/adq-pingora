use std::collections::HashMap;
use std::fs;
use std::path::Path;
use regex::Regex;
use log::info;

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
    pub response: Option<(u16, String)>,
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
        
        let mut dir = dir.collect::<Result<Vec<_>, _>>()?;
        dir.sort_by_key(|entry| entry.file_name());
        for entry in dir {
            let path = entry.path();
            
            if entry.file_type()?.is_dir() {
                continue;
            }
            let config = Self::parse_config_file(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            info!("Loaded config from: {}", path.display());
            servers.extend(config.servers);
            for (name, upstream) in config.upstreams {
                if upstreams.insert(name.clone(), upstream).is_some() {
                    return Err(format!("duplicate upstream {name}").into());
                }
            }
        }

        if servers.is_empty() {
            return Err("no server blocks enabled".into());
        }
        for server in &servers {
            for location in &server.locations {
                if let Some(name) = &location.proxy_pass {
                    if !upstreams.contains_key(name) {
                        return Err(format!("unknown upstream {name} for {}", location.path).into());
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

        let comments = Regex::new(r#""(?:\\.|[^"\\])*"|#[^\n]*"#)?;
        let content = comments.replace_all(content, |captures: &regex::Captures| {
            let token = &captures[0];
            if token.starts_with('#') { String::new() } else { token.to_string() }
        });

        let blocks = Regex::new(
            r#"(server|upstream\s+([\w-]+))\s*\{((?:[^{}"]|"(?:\\.|[^"\\])*"|\{(?:[^{}"]|"(?:\\.|[^"\\])*")*\})*)\}"#,
        )?;
        let mut end = 0;
        for capture in blocks.captures_iter(&content) {
            let matched = capture.get(0).unwrap();
            if !content[end..matched.start()].trim().is_empty() {
                return Err("invalid server or upstream block".into());
            }
            end = matched.end();
            if let Some(name) = capture.get(2) {
                let name = name.as_str();
                let upstream = Self::parse_upstream_block(name, &capture[3])?;
                if upstreams.insert(name.to_string(), upstream).is_some() {
                    return Err(format!("duplicate upstream {name}").into());
                }
            } else {
                servers.push(Self::parse_server_block(&capture[3])?);
            }
        }
        if !content[end..].trim().is_empty() {
            return Err("invalid or unterminated server or upstream block".into());
        }

        Ok(NginxConfig { servers, upstreams })
    }

    fn directives(content: &str) -> Result<Vec<(&str, &str)>, Box<dyn std::error::Error>> {
        let directive = Regex::new(r#"^([a-z_]+)(?:\s+((?:[^;"{}]|"(?:\\.|[^"\\])*")+))?\s*;"#)?;
        let mut remaining = content.trim();
        let mut result = Vec::new();
        while !remaining.is_empty() {
            let captures = directive.captures(remaining)
                .ok_or_else(|| format!("invalid directive near {:?}", remaining.lines().next().unwrap_or("")))?;
            let name = captures.get(1).unwrap().as_str();
            let value = captures.get(2).map_or("", |capture| capture.as_str().trim());
            result.push((name, value));
            remaining = remaining[captures.get(0).unwrap().end()..].trim();
        }
        Ok(result)
    }

    fn parse_server_block(content: &str) -> Result<ServerBlock, Box<dyn std::error::Error>> {
        let mut server = ServerBlock {
            listen_ports: Vec::new(),
            server_names: Vec::new(),
            ssl_certificate: None,
            ssl_certificate_key: None,
            locations: Vec::new(),
        };

        let location = Regex::new(r#"\blocation\s+(/[^\s{]*)\s*\{((?:[^{}"]|"(?:\\.|[^"\\])*")*)\}"#)?;
        for captures in location.captures_iter(content) {
            server.locations.push(Self::parse_location_block(&captures[1], &captures[2])?);
        }
        let content = location.replace_all(content, "");
        for (name, value) in Self::directives(&content)? {
            match name {
                "listen" => server.listen_ports.push(Self::parse_listen_directive(value)?),
                "server_name" if !value.is_empty() => server.server_names.extend(value.split_whitespace().map(str::to_owned)),
                "ssl_certificate" if !value.is_empty() => server.ssl_certificate = Some(value.to_owned()),
                "ssl_certificate_key" if !value.is_empty() => server.ssl_certificate_key = Some(value.to_owned()),
                _ => return Err(format!("unsupported or invalid server directive {name}").into()),
            }
        }
        if server.listen_ports.is_empty() {
            return Err("server has no listen directive".into());
        }
        Ok(server)
    }

    /// Парсит listen директиву
    fn parse_listen_directive(listen_str: &str) -> Result<ListenDirective, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = listen_str.split_whitespace().collect();
        let port_str = parts.first().ok_or("listen requires a port")?;
        
        let port = port_str.parse::<u16>()?;
        if port == 0 || parts[1..].iter().any(|flag| !matches!(*flag, "ssl" | "http2")) {
            return Err(format!("invalid listen directive: {listen_str}").into());
        }
        let ssl = parts.contains(&"ssl");
        let http2 = parts.contains(&"http2");

        Ok(ListenDirective { port, ssl, http2 })
    }

    fn parse_location_block(path: &str, content: &str) -> Result<LocationBlock, Box<dyn std::error::Error>> {
        let mut location = LocationBlock {
            path: path.to_owned(),
            proxy_pass: None,
            response: None,
            rate_limit: None,
            cors_enable: false,
        };
        for (name, value) in Self::directives(content)? {
            match name {
                "proxy_pass" if value.split_whitespace().count() == 1 && location.proxy_pass.is_none() => {
                    location.proxy_pass = Some(value.to_owned());
                }
                "return" if location.response.is_none() => {
                    let (status, body) = value.split_once(char::is_whitespace).unwrap_or((value, ""));
                    let status = status.parse::<u16>()?;
                    if !(200..=599).contains(&status) {
                        return Err(format!("invalid return status for {path}").into());
                    }
                    let body = if body.is_empty() { String::new() } else { serde_json::from_str(body.trim())? };
                    location.response = Some((status, body));
                }
                "rate_limit" => {
                    let limits: Vec<_> = value.split_whitespace().collect();
                    let [requests_per_second, burst] = limits.as_slice() else {
                        return Err("rate_limit requires a rate and burst".into());
                    };
                    location.rate_limit = Some(RateLimit {
                        requests_per_second: requests_per_second.parse()?,
                        burst: burst.parse()?,
                    });
                }
                "cors_enable" if value.is_empty() => location.cors_enable = true,
                _ => return Err(format!("unsupported or invalid location directive {name}").into()),
            }
        }
        if location.response.is_some() == location.proxy_pass.is_some() {
            return Err(format!("location {path} requires exactly one of proxy_pass or return").into());
        }
        Ok(location)
    }

    /// Парсит upstream блок
    fn parse_upstream_block(name: &str, content: &str) -> Result<UpstreamBlock, Box<dyn std::error::Error>> {
        let mut servers = Vec::new();

        for (directive, value) in Self::directives(content)? {
            let mut parts = value.split_whitespace();
            let address = parts.next().filter(|_| directive == "server")
                .ok_or("upstream requires a server address")?;
            servers.push(UpstreamServer { address: address.to_owned(), weight: 1 });
        }

        if servers.is_empty() {
            return Err(format!("upstream {name} has no servers").into());
        }
        Ok(UpstreamBlock {
            name: name.to_string(),
            servers,
        })
    }

    /// Находит server блок по host
    pub fn find_server(&self, host: &str) -> Option<&ServerBlock> {
        self.find_server_on_port(host, None)
    }

    pub fn find_server_on_port(&self, host: &str, port: Option<u16>) -> Option<&ServerBlock> {
        let host = host.parse::<http::uri::Authority>().ok()?;
        let servers = self.servers.iter().filter(|server| {
            port.is_none() || server.listen_ports.iter().any(|listen| Some(listen.port) == port)
        });
        servers.clone()
            .find(|server| server.server_names.iter().any(|name| name.eq_ignore_ascii_case(host.host())))
            .or_else(|| servers.clone().find(|server| server.server_names.iter().any(|name| name == "_")))
            .or_else(|| port.and_then(|_| servers.clone().next()))
    }

    /// Находит location в server блоке по пути
    pub fn find_location<'a>(&self, server: &'a ServerBlock, path: &str) -> Option<&'a LocationBlock> {
        // Сначала ищем точное совпадение
        for location in &server.locations {
            if location.path == path {
                return Some(location);
            }
        }

        server.locations.iter()
            .filter(|location| path.starts_with(&location.path))
            .max_by_key(|location| location.path.len())
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