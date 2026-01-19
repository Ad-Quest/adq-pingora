# Configuration Reference

ADQ Pingora uses a two-tier configuration system:
1. **Main Configuration** (`/etc/adq-pingora/proxy.yaml`) - Global settings
2. **Site Configurations** (`/etc/adq-pingora/sites-available/`) - Nginx-style server blocks

## Main Configuration

The main configuration file `/etc/adq-pingora/proxy.yaml` contains global settings:

```yaml
version: 1

# Global settings
global:
  default_timeout: 30
  max_retries: 3
  health_check_interval: 5

# Security headers
security:
  headers:
    x_frame_options: "SAMEORIGIN"
    x_content_type_options: "nosniff"
    x_xss_protection: "1; mode=block"
    strict_transport_security: "max-age=31536000; includeSubDomains"

# Caching configuration
cache:
  enabled: true
  default_ttl: 300
  rules:
    - path: "/static/*"
      ttl: 3600
    - path: "*.css"
      ttl: 86400

# Logging configuration
logging:
  format: "json"
  level: "info"
  access_log:
    enabled: true
    path: "/var/log/adq-pingora/access.log"
  error_log:
    enabled: true
    path: "/var/log/adq-pingora/error.log"

# Circuit breaker
circuit_breaker:
  enabled: true
  failure_threshold: 5
  recovery_timeout: 30

# IP filtering
ip_filter:
  enabled: false
  whitelist:
    - "127.0.0.1"
    - "10.0.0.0/8"
```

## Site Configuration

Site configurations use nginx-like syntax in `/etc/adq-pingora/sites-available/`:

### Basic Server Block

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name example.com www.example.com;
    
    # SSL configuration
    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    
    # Locations
    location / {
        proxy_pass backend;
        rate_limit 100 200;
        cors_enable;
    }
}

upstream backend {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
}
```

## Directives Reference

### Server Block Directives

#### listen
Specifies the address and port for the server to listen on.

```nginx
listen 80;                    # IPv4, port 80
listen [::]:80;              # IPv6, port 80
listen 443 ssl;              # SSL on port 443
listen 443 ssl http2;        # SSL with HTTP/2
```

#### server_name
Sets names of a virtual server.

```nginx
server_name example.com;                    # Single domain
server_name example.com www.example.com;    # Multiple domains
server_name *.example.com;                  # Wildcard
```

#### ssl_certificate / ssl_certificate_key
Specifies SSL certificate files.

```nginx
ssl_certificate /etc/ssl/certs/example.com.crt;
ssl_certificate_key /etc/ssl/private/example.com.key;
```

### Location Block Directives

#### proxy_pass
Sets the protocol and address of a proxied server.

```nginx
location / {
    proxy_pass backend;           # Upstream name
}

location /api {
    proxy_pass api_servers;       # Different upstream
}
```

#### rate_limit
Configures rate limiting for the location.

```nginx
rate_limit 100 200;              # 100 req/s, burst 200
rate_limit 50;                   # 50 req/s, no burst
```

#### cors_enable
Enables CORS headers for the location.

```nginx
location /api {
    proxy_pass backend;
    cors_enable;
}
```

### Upstream Block Directives

#### server
Defines a server in the upstream group.

```nginx
upstream backend {
    server 127.0.0.1:8080;       # Basic server
    server 127.0.0.1:8081;       # Load balanced
    server 192.168.1.10:8080;    # Remote server
}
```

## Configuration Examples

### Simple Web Server

```nginx
server {
    listen 80;
    server_name mysite.com;
    
    location / {
        proxy_pass web_backend;
    }
}

upstream web_backend {
    server 127.0.0.1:3000;
}
```

### API Gateway with Rate Limiting

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name api.example.com;
    
    ssl_certificate /etc/ssl/certs/api.example.com.crt;
    ssl_certificate_key /etc/ssl/private/api.example.com.key;
    
    # Public API - limited
    location /api/v1/public {
        proxy_pass public_api;
        rate_limit 10 20;
        cors_enable;
    }
    
    # Private API - higher limits
    location /api/v1/private {
        proxy_pass private_api;
        rate_limit 100 200;
        cors_enable;
    }
    
    # Health check - no limits
    location /health {
        proxy_pass health_check;
    }
}

upstream public_api {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
}

upstream private_api {
    server 127.0.0.1:9080;
    server 127.0.0.1:9081;
}

upstream health_check {
    server 127.0.0.1:8080;
}
```

### Multi-Service Setup

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name app.example.com;
    
    ssl_certificate /etc/ssl/certs/app.example.com.crt;
    ssl_certificate_key /etc/ssl/private/app.example.com.key;
    
    # Frontend application
    location / {
        proxy_pass frontend;
        rate_limit 50 100;
    }
    
    # API backend
    location /api {
        proxy_pass api_backend;
        rate_limit 100 200;
        cors_enable;
    }
    
    # Static assets
    location /static {
        proxy_pass static_files;
        rate_limit 1000 2000;
    }
    
    # WebSocket endpoint
    location /ws {
        proxy_pass websocket_backend;
    }
}

upstream frontend {
    server 127.0.0.1:3000;
}

upstream api_backend {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;
}

upstream static_files {
    server 127.0.0.1:9000;
}

upstream websocket_backend {
    server 127.0.0.1:8090;
}
```

## Configuration Testing

Always test your configuration before applying:

```bash
# Test configuration syntax
adq-pingora -t

# Test specific configuration file
adq-pingora -t -c /path/to/config.yaml
```

## Configuration Reload

Reload configuration without downtime:

```bash
# Reload configuration
sudo systemctl reload adq-pingora

# Or send HUP signal
sudo kill -HUP $(cat /var/run/adq-pingora.pid)
```