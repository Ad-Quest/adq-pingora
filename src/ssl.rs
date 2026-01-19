use pingora_core::listeners::tls::TlsSettings;
use pingora_core::listeners::TlsAccept;
use pingora_core::services::listening::Service;
use pingora_proxy::HttpProxy;
use pingora_core::protocols::tls::TlsRef;
use pingora_core::tls::ssl::{NameType, SslFiletype};
use log::info;
use std::path::Path;
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

/// Настраивает SSL/TLS для прокси сервиса с поддержкой нескольких доменов
pub fn configure_ssl(proxy_service: &mut Service<HttpProxy<crate::proxy::AdQuestProxy>>) {
    // Создаем менеджер сертификатов
    let mut cert_manager = MultiCertManager::new();
    
    // Добавляем все доступные сертификаты
    let cert_configs = [
        ("auth.ad-quest.ru", "/etc/letsencrypt/live/auth.ad-quest.ru/fullchain.pem", "/etc/letsencrypt/live/auth.ad-quest.ru/privkey.pem"),
        ("api.ad-quest.ru", "/etc/letsencrypt/live/api.ad-quest.ru/fullchain.pem", "/etc/letsencrypt/live/api.ad-quest.ru/privkey.pem"),
    ];
    
    let mut default_cert_path = None;
    let mut default_key_path = None;
    
    for (domain, cert_path, key_path) in cert_configs.iter() {
        if Path::new(cert_path).exists() && Path::new(key_path).exists() {
            cert_manager.add_certificate(domain, cert_path, key_path);
            info!("Added certificate for domain: {}", domain);
            
            // Используем первый найденный сертификат как default
            if default_cert_path.is_none() {
                default_cert_path = Some(cert_path);
                default_key_path = Some(key_path);
            }
        } else {
            info!("Certificate not found for domain: {} at {} and {}", domain, cert_path, key_path);
        }
    }
    
    // Настраиваем TLS с callback для динамического выбора сертификатов
    if let (Some(default_cert), Some(default_key)) = (default_cert_path, default_key_path) {
        match TlsSettings::with_callbacks(Box::new(cert_manager)) {
            Ok(mut tls_settings) => {
                tls_settings.enable_h2();
                
                // Устанавливаем default сертификат (будет использован если SNI не совпадает)
                if let Err(e) = tls_settings.set_certificate_chain_file(default_cert) {
                    info!("Failed to set default certificate: {}", e);
                    return;
                }
                if let Err(e) = tls_settings.set_private_key_file(default_key, SslFiletype::PEM) {
                    info!("Failed to set default private key: {}", e);
                    return;
                }
                
                proxy_service.add_tls_with_settings("0.0.0.0:443", None, tls_settings);
                info!("HTTPS enabled on port 443 with multi-domain certificate support");
                info!("Default certificate: {}", default_cert);
                info!("Supported domains: auth.ad-quest.ru, api.ad-quest.ru");
            }
            Err(e) => {
                info!("Failed to create TLS settings with callbacks: {}", e);
            }
        }
    } else {
        info!("No valid TLS certificates found, HTTPS disabled");
    }
}