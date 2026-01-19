use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::info;

/// Фильтр соединений для блокировки/разрешения IP адресов
#[derive(Debug, Clone)]
pub struct IPFilter {
    /// Blacklist IP адресов
    blacklist: Arc<RwLock<HashSet<IpAddr>>>,
    /// Whitelist IP адресов (если установлен, разрешены только эти IP)
    whitelist: Option<Arc<RwLock<HashSet<IpAddr>>>>,
    /// Максимальное количество соединений с одного IP
    max_connections_per_ip: Option<usize>,
    /// Счетчик активных соединений по IP
    connection_counts: Arc<RwLock<std::collections::HashMap<IpAddr, usize>>>,
}

impl IPFilter {
    /// Создает новый фильтр без ограничений
    pub fn new() -> Self {
        Self {
            blacklist: Arc::new(RwLock::new(HashSet::new())),
            whitelist: None,
            max_connections_per_ip: None,
            connection_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Создает фильтр с whitelist (разрешены только IP из whitelist)
    pub fn with_whitelist(whitelist: HashSet<IpAddr>) -> Self {
        Self {
            blacklist: Arc::new(RwLock::new(HashSet::new())),
            whitelist: Some(Arc::new(RwLock::new(whitelist))),
            max_connections_per_ip: None,
            connection_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Добавляет IP в blacklist
    pub async fn add_to_blacklist(&self, ip: IpAddr) {
        self.blacklist.write().await.insert(ip);
        info!("Added {} to blacklist", ip);
    }

    /// Удаляет IP из blacklist
    pub async fn remove_from_blacklist(&self, ip: IpAddr) {
        if self.blacklist.write().await.remove(&ip) {
            info!("Removed {} from blacklist", ip);
        }
    }

    /// Добавляет IP в whitelist
    pub async fn add_to_whitelist(&self, ip: IpAddr) {
        if let Some(whitelist) = &self.whitelist {
            whitelist.write().await.insert(ip);
            info!("Added {} to whitelist", ip);
        }
    }

    /// Загружает blacklist из файла (по одному IP на строку)
    pub async fn load_blacklist_from_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let mut blacklist = self.blacklist.write().await;
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue; // Пропускаем пустые строки и комментарии
            }
            
            if let Ok(ip) = line.parse::<IpAddr>() {
                blacklist.insert(ip);
            } else {
                // Попытка парсинга CIDR (базовая поддержка)
                if let Some((ip_str, _)) = line.split_once('/') {
                    if let Ok(ip) = ip_str.trim().parse::<IpAddr>() {
                        blacklist.insert(ip);
                        info!("Added {} from CIDR notation to blacklist", ip);
                    }
                }
            }
        }
        
        info!("Loaded {} IPs from blacklist file: {}", blacklist.len(), path);
        Ok(())
    }

    /// Устанавливает максимальное количество соединений с одного IP
    pub fn set_max_connections_per_ip(&mut self, max: usize) {
        self.max_connections_per_ip = Some(max);
    }

    /// Увеличивает счетчик соединений для IP (вызывается при установке соединения)
    pub async fn increment_connection_count(&self, ip: IpAddr) {
        if self.max_connections_per_ip.is_some() {
            let mut counts = self.connection_counts.write().await;
            *counts.entry(ip).or_insert(0) += 1;
        }
    }

    /// Уменьшает счетчик соединений для IP (вызывается при закрытии соединения)
    pub async fn decrement_connection_count(&self, ip: IpAddr) {
        if self.max_connections_per_ip.is_some() {
            let mut counts = self.connection_counts.write().await;
            if let Some(count) = counts.get_mut(&ip) {
                if *count > 0 {
                    *count -= 1;
                }
                if *count == 0 {
                    counts.remove(&ip);
                }
            }
        }
    }

    /// Получает количество активных соединений для IP
    pub async fn get_connection_count(&self, ip: IpAddr) -> usize {
        self.connection_counts
            .read()
            .await
            .get(&ip)
            .copied()
            .unwrap_or(0)
    }
}

impl IPFilter {
    /// Проверяет, должен ли IP быть заблокирован
    /// Используется в request_filter для фильтрации запросов
    pub async fn should_block_ip(&self, ip: IpAddr) -> bool {

        // Проверяем whitelist (если установлен, разрешены только эти IP)
        if let Some(whitelist) = &self.whitelist {
            if !whitelist.read().await.contains(&ip) {
                info!("Blocking request from {} (not in whitelist)", ip);
                return true; // Блокируем
            }
        }

        // Проверяем blacklist
        if self.blacklist.read().await.contains(&ip) {
            info!("Blocking request from {} (in blacklist)", ip);
            return true; // Блокируем
        }

        // Проверяем лимит соединений с одного IP
        // Проверяем, не превысит ли новое соединение лимит
        if let Some(max) = self.max_connections_per_ip {
            let count = self.get_connection_count(ip).await;
            // Если текущее количество уже >= max, блокируем
            if count >= max {
                info!(
                    "Blocking request from {} (max connections exceeded: {}/{})",
                    ip, count, max
                );
                return true; // Блокируем
            }
        }

        false // Не блокируем
    }
}

impl Default for IPFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_ip_filter_new() {
        let filter = IPFilter::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(!filter.should_block_ip(ip).await);
    }

    #[tokio::test]
    async fn test_ip_filter_blacklist() {
        let filter = IPFilter::new();
        filter.add_to_blacklist("192.168.1.100".parse().unwrap()).await;
        
        let blocked_ip: IpAddr = "192.168.1.100".parse().unwrap();
        assert!(filter.should_block_ip(blocked_ip).await);
        
        let allowed_ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(!filter.should_block_ip(allowed_ip).await);
    }

    #[tokio::test]
    async fn test_ip_filter_whitelist() {
        let mut whitelist = HashSet::new();
        whitelist.insert("127.0.0.1".parse().unwrap());
        whitelist.insert("10.0.0.1".parse().unwrap());
        
        let filter = IPFilter::with_whitelist(whitelist);
        
        let allowed_ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(!filter.should_block_ip(allowed_ip).await);
        
        let blocked_ip: IpAddr = "192.168.1.100".parse().unwrap();
        assert!(filter.should_block_ip(blocked_ip).await);
    }

    #[tokio::test]
    async fn test_ip_filter_max_connections() {
        let mut filter = IPFilter::new();
        filter.set_max_connections_per_ip(2);
        
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        
        // Без соединений - не блокируем
        assert!(!filter.should_block_ip(ip).await);
        
        // Первое соединение - не блокируем (count=1, max=2)
        filter.increment_connection_count(ip).await;
        assert!(!filter.should_block_ip(ip).await);
        
        // Второе соединение - не блокируем (count=2, max=2, count == max, но еще можно)
        filter.increment_connection_count(ip).await;
        // После второго increment count=2, что равно max, поэтому следующее будет заблокировано
        assert!(filter.should_block_ip(ip).await); // count=2 >= max=2, блокируем
        
        // После уменьшения счетчика должно быть разрешено
        filter.decrement_connection_count(ip).await;
        assert!(!filter.should_block_ip(ip).await); // count=1 < max=2, разрешаем
    }
}
