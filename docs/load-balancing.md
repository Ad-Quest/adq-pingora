# Load Balancing

ADQ Pingora provides advanced load balancing capabilities with health checks, automatic failover, and multiple balancing algorithms.

## Basic Load Balancing

Define multiple servers in an upstream block:

```nginx
upstream backend {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;
}

server {
    listen 80;
    server_name example.com;
    
    location / {
        proxy_pass backend;
    }
}
```

## Load Balancing Methods

### Round Robin (Default)

Requests are distributed evenly across all servers:

```nginx
upstream backend {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;
}
```

Request distribution: Server1 → Server2 → Server3 → Server1 → ...

## Health Checks

ADQ Pingora automatically performs health checks on upstream servers.

### TCP Health Checks

By default, TCP health checks are performed every 5 seconds:

```yaml
# In proxy.yaml
global:
  health_check_interval: 5  # seconds
```

### Health Check Behavior

- **Healthy Server**: Receives traffic normally
- **Failed Server**: Automatically removed from rotation
- **Recovered Server**: Automatically added back to rotation

### Monitoring Health Status

Check health status in logs:

```bash
# View health check logs
sudo journalctl -u adq-pingora | grep "health"

# Real-time monitoring
sudo journalctl -u adq-pingora -f | grep "health"
```

## Failover and Recovery

### Automatic Failover

When a server fails:
1. Health check detects the failure
2. Server is marked as unhealthy
3. Traffic is redistributed to healthy servers
4. Failed server continues to be monitored

### Automatic Recovery

When a failed server recovers:
1. Health check detects the recovery
2. Server is marked as healthy
3. Traffic is gradually restored to the server

## Configuration Examples

### Basic Web Application

```nginx
upstream web_servers {
    server 10.0.1.10:80;
    server 10.0.1.11:80;
    server 10.0.1.12:80;
}

server {
    listen 80;
    server_name myapp.com;
    
    location / {
        proxy_pass web_servers;
    }
}
```

### API Gateway with Multiple Services

```nginx
# User service
upstream user_service {
    server 10.0.2.10:8080;
    server 10.0.2.11:8080;
}

# Order service
upstream order_service {
    server 10.0.3.10:8080;
    server 10.0.3.11:8080;
    server 10.0.3.12:8080;
}

# Payment service
upstream payment_service {
    server 10.0.4.10:8080;
    server 10.0.4.11:8080;
}

server {
    listen 80;
    listen 443 ssl http2;
    server_name api.example.com;
    
    ssl_certificate /etc/ssl/certs/api.example.com.crt;
    ssl_certificate_key /etc/ssl/private/api.example.com.key;
    
    location /api/users {
        proxy_pass user_service;
        rate_limit 100 200;
    }
    
    location /api/orders {
        proxy_pass order_service;
        rate_limit 50 100;
    }
    
    location /api/payments {
        proxy_pass payment_service;
        rate_limit 20 40;
    }
}
```

### Database Load Balancing

```nginx
# Read replicas
upstream db_read {
    server db-read-1.internal:5432;
    server db-read-2.internal:5432;
    server db-read-3.internal:5432;
}

# Write master
upstream db_write {
    server db-master.internal:5432;
}

server {
    listen 80;
    server_name db-proxy.internal;
    
    # Read queries
    location /api/read {
        proxy_pass db_read;
        rate_limit 1000 2000;
    }
    
    # Write queries
    location /api/write {
        proxy_pass db_write;
        rate_limit 100 200;
    }
}
```

## Monitoring and Metrics

### Health Check Logs

Health check events are logged with structured data:

```json
{
  "timestamp": "2024-01-19T10:30:00Z",
  "level": "INFO",
  "message": "Health check passed",
  "upstream": "backend",
  "server": "127.0.0.1:8080",
  "response_time": "5ms"
}
```

### Failed Server Logs

When a server fails:

```json
{
  "timestamp": "2024-01-19T10:30:15Z",
  "level": "WARN",
  "message": "Health check failed",
  "upstream": "backend",
  "server": "127.0.0.1:8081",
  "error": "Connection refused"
}
```

### Recovery Logs

When a server recovers:

```json
{
  "timestamp": "2024-01-19T10:35:00Z",
  "level": "INFO",
  "message": "Server recovered",
  "upstream": "backend",
  "server": "127.0.0.1:8081"
}
```

## Best Practices

### 1. Use Appropriate Health Check Intervals

```yaml
# For critical services
global:
  health_check_interval: 3

# For less critical services
global:
  health_check_interval: 10
```

### 2. Plan for Capacity

Ensure remaining servers can handle the load when one fails:

```nginx
# If you need to handle 1000 req/s
# Use 4 servers (each handling 333 req/s)
# So 3 servers can handle the load if 1 fails
upstream backend {
    server server1:8080;  # 333 req/s capacity
    server server2:8080;  # 333 req/s capacity
    server server3:8080;  # 333 req/s capacity
    server server4:8080;  # 333 req/s capacity
}
```

### 3. Monitor Health Status

Set up monitoring for health check failures:

```bash
# Create alert script
#!/bin/bash
journalctl -u adq-pingora --since "1 minute ago" | \
grep "Health check failed" | \
while read line; do
    echo "ALERT: $line" | mail -s "ADQ Pingora Health Check Failed" admin@example.com
done
```

### 4. Use Circuit Breaker

Enable circuit breaker for additional fault tolerance:

```yaml
# In proxy.yaml
circuit_breaker:
  enabled: true
  failure_threshold: 5      # Open circuit after 5 failures
  recovery_timeout: 30      # Try recovery after 30 seconds
```

## Troubleshooting

### All Servers Marked as Failed

Check network connectivity:

```bash
# Test connectivity to upstream servers
telnet 127.0.0.1 8080
telnet 127.0.0.1 8081

# Check firewall rules
sudo iptables -L
sudo ufw status
```

### Uneven Load Distribution

This is normal with round-robin. For more even distribution:

1. Ensure all servers have similar capacity
2. Monitor server performance
3. Consider using multiple upstream blocks for different server classes

### Health Check False Positives

Adjust health check interval if servers are being marked as failed incorrectly:

```yaml
global:
  health_check_interval: 10  # Increase interval
```