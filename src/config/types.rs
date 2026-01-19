use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub upstreams: HashMap<String, Vec<String>>,
    pub servers: Vec<ServerConfig>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub server_name: String,
    pub listen_http: Option<u16>,
    pub listen_https: Option<u16>,
    pub ssl_cert: Option<String>,
    pub ssl_key: Option<String>,
    pub locations: Vec<LocationConfig>,
}

#[derive(Debug, Clone)]
pub struct LocationConfig {
    pub path: String,
    pub upstream: String,
    pub rate_limit_rps: Option<u32>,
    pub rate_limit_burst: Option<u32>,
    pub enable_cors: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            upstreams: HashMap::new(),
            servers: Vec::new(),
        }
    }
}