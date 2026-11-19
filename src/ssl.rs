use pingora_core::listeners::tls::TlsSettings;
use pingora_core::listeners::TlsAccept;
use pingora_core::protocols::tls::TlsRef;
use pingora_core::tls::ssl::{NameType, SslFiletype};
use log::info;
use std::collections::HashMap;
use async_trait::async_trait;

/// Структура для управления несколькими SSL сертификатами
pub struct MultiCertManager {
    certificates: HashMap<String, (String, String)>, // domain -> (cert_path, key_path)
}

impl MultiCertManager {
    pub fn new() -> Self {
        Self {
            certificates: HashMap::new(),
        }
    }

    pub fn add_certificate(&mut self, domain: &str, cert_path: &str, key_path: &str) {
        self.certificates.insert(domain.to_string(), (cert_path.to_string(), key_path.to_string()));
    }
}

#[async_trait]
impl TlsAccept for MultiCertManager {
    async fn certificate_callback(&self, ssl: &mut TlsRef) -> () {
        // Получаем SNI (Server Name Indication) из TLS handshake
        let servername = ssl.servername(NameType::HOST_NAME).map(|s| s.to_string());
        
        if let Some(servername) = servername {
            info!("SNI requested: {}", servername);
            
            // Ищем подходящий сертификат
            if let Some((cert_path, key_path)) = self.certificates.get(&servername) {
                info!("Loading certificate for domain: {} from {}", servername, cert_path);
                
                // Загружаем сертификат и ключ
                if let Err(e) = ssl.set_certificate_chain_file(cert_path) {
                    log::error!("Failed to load certificate for {}: {}", servername, e);
                    return;
                }
                
                if let Err(e) = ssl.set_private_key_file(key_path, SslFiletype::PEM) {
                    log::error!("Failed to load private key for {}: {}", servername, e);
                    return;
                }
                
                info!("Successfully loaded certificate for domain: {}", servername);
            } else {
                info!("No certificate found for domain: {}, using default", servername);
            }
        } else {
            info!("No SNI provided, using default certificate");
        }
    }
}

/// Validate and group the listeners by port before starting the server.
pub fn listener_settings(
    config: &crate::config::NginxConfig,
) -> Result<std::collections::BTreeMap<u16, Option<TlsSettings>>, Box<dyn std::error::Error>> {
    use std::collections::BTreeMap;

    let mut ports = BTreeMap::<u16, Vec<_>>::new();
    for server in &config.servers {
        for listen in &server.listen_ports {
            ports.entry(listen.port).or_default().push((server, listen));
        }
    }

    let mut settings = BTreeMap::new();
    for (port, servers) in ports {
        let ssl = servers[0].1.ssl;
        if servers.iter().any(|(_, listen)| listen.ssl != ssl) {
            return Err(format!("port {port} mixes HTTP and TLS listeners").into());
        }
        if !ssl {
            settings.insert(port, None);
            continue;
        }

        let mut certificates = MultiCertManager::new();
        let mut default = None;
        for (server, _) in &servers {
            let (Some(cert), Some(key)) = (&server.ssl_certificate, &server.ssl_certificate_key) else {
                return Err(format!("TLS listener {port} requires a certificate and private key").into());
            };
            TlsSettings::intermediate(cert, key)?.check_private_key()?;
            if default.is_none() {
                default = Some((cert, key));
            }
            for domain in &server.server_names {
                certificates.add_certificate(domain, cert, key);
            }
        }
        let (cert, key) = default.unwrap();
        let mut tls = TlsSettings::with_callbacks(Box::new(certificates))?;
        tls.set_certificate_chain_file(cert)?;
        tls.set_private_key_file(key, SslFiletype::PEM)?;
        if servers.iter().any(|(_, listen)| listen.http2) {
            tls.enable_h2();
        }
        settings.insert(port, Some(tls));
    }
    Ok(settings)
}
