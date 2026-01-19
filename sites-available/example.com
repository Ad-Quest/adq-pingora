# Example server configuration
# Copy this file and modify for your needs

server {
    listen 9080;
    listen 9443 ssl http2;
    server_name example.com www.example.com;
    
    # SSL Configuration (optional)
    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    
    # API endpoints
    location /api {
        proxy_pass api_backend;
        rate_limit 100 200;  # 100 rps, burst 200
        cors_enable;
    }
    
    # Static files
    location /static {
        proxy_pass static_backend;
        rate_limit 1000 2000;  # 1000 rps, burst 2000
        cors_enable;
    }
    
    # Health check (no rate limiting)
    location /health {
        proxy_pass api_backend;
        cors_enable;
    }
    
    # Default location
    location / {
        proxy_pass web_backend;
        rate_limit 50 100;   # 50 rps, burst 100
        cors_enable;
    }
}

# Upstream definitions
upstream api_backend {
    server 127.0.0.1:8099;
    server 127.0.0.1:8098;
}

upstream static_backend {
    server 127.0.0.1:8097;
}

upstream web_backend {
    server 127.0.0.1:8099;
}