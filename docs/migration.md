# Migration from Nginx

This guide helps you migrate from Nginx to ADQ Pingora with minimal downtime and configuration changes.

## Pre-Migration Assessment

### 1. Analyze Current Nginx Configuration

```bash
# Backup current nginx configuration
sudo cp -r /etc/nginx /etc/nginx.backup.$(date +%Y%m%d)

# List enabled sites
ls -la /etc/nginx/sites-enabled/

# Check nginx configuration
sudo nginx -t

# View current nginx status
sudo systemctl status nginx
```

### 2. Identify Nginx Features in Use

Common nginx features and ADQ Pingora equivalents:

| Nginx Feature | ADQ Pingora Equivalent | Status |
|---------------|------------------------|---------|
| `proxy_pass` | `proxy_pass` | ✅ Supported |
| `upstream` | `upstream` | ✅ Supported |
| `server` blocks | `server` blocks | ✅ Supported |
| `location` blocks | `location` blocks | ✅ Supported |
| `listen` | `listen` | ✅ Supported |
| `server_name` | `server_name` | ✅ Supported |
| `ssl_certificate` | `ssl_certificate` | ✅ Supported |
| Rate limiting | `rate_limit` | ✅ Enhanced |
| Load balancing | Built-in with health checks | ✅ Enhanced |
| CORS headers | `cors_enable` | ✅ Simplified |
| Access logs | JSON structured logs | ✅ Enhanced |

## Migration Process

### Step 1: Install ADQ Pingora

```bash
# Install ADQ Pingora alongside nginx (don't stop nginx yet)
git clone https://github.com/Ad-Quest/adquest-pingora-proxy.git
cd adquest-pingora-proxy
sudo ./scripts/install.sh
```

### Step 2: Convert Nginx Configuration

#### Basic Server Block Conversion

**Nginx configuration:**
```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name example.com www.example.com;
    
    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    
    location / {
        proxy_pass http://backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
    
    location /api {
        proxy_pass http://api_backend;
        proxy_set_header Host $host;
    }
}

upstream backend {
    server 127.0.0.1:3000;
    server 127.0.0.1:3001;
}

upstream api_backend {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
}
```

**ADQ Pingora equivalent:**
```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name example.com www.example.com;
    
    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    
    location / {
        proxy_pass backend;
        # Headers are automatically set
    }
    
    location /api {
        proxy_pass api_backend;
        rate_limit 100 200;  # Optional: add rate limiting
        cors_enable;         # Optional: enable CORS
    }
}

upstream backend {
    server 127.0.0.1:3000;
    server 127.0.0.1:3001;
    # Health checks are automatic
}

upstream api_backend {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
}
```

#### Advanced Configuration Conversion

**Nginx with rate limiting:**
```nginx
http {
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
    
    server {
        listen 80;
        server_name api.example.com;
        
        location /api {
            limit_req zone=api burst=20 nodelay;
            proxy_pass http://api_backend;
        }
    }
}
```

**ADQ Pingora equivalent:**
```nginx
server {
    listen 80;
    server_name api.example.com;
    
    location /api {
        proxy_pass api_backend;
        rate_limit 10 20;  # 10 req/s, burst 20 - much simpler!
    }
}

upstream api_backend {
    server 127.0.0.1:8080;
}
```

### Step 3: Create ADQ Pingora Configuration

```bash
# Create site configuration
sudo nano /etc/adq-pingora/sites-available/example.com

# Copy your converted configuration here

# Enable the site
sudo adq-ensite example.com

# Test configuration
sudo adq-pingora -t
```

### Step 4: Parallel Testing

Test ADQ Pingora on different ports while nginx is running:

```nginx
# Temporary configuration for testing
server {
    listen 8080;  # Different port for testing
    listen 8443 ssl http2;
    server_name example.com;
    
    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    
    location / {
        proxy_pass backend;
    }
}
```

Test the configuration:

```bash
# Start ADQ Pingora
sudo systemctl start adq-pingora

# Test with curl
curl -H "Host: example.com" http://localhost:8080/
curl -k -H "Host: example.com" https://localhost:8443/

# Compare responses with nginx
curl -H "Host: example.com" http://localhost:80/
```

### Step 5: Switch Over

#### Option A: Immediate Switch (Minimal Downtime)

```bash
# Stop nginx
sudo systemctl stop nginx

# Update ADQ Pingora to use standard ports
sudo nano /etc/adq-pingora/sites-available/example.com
# Change listen 8080 to listen 80
# Change listen 8443 to listen 443

# Reload ADQ Pingora
sudo systemctl reload adq-pingora

# Disable nginx from starting
sudo systemctl disable nginx
```

#### Option B: Gradual Migration (Zero Downtime)

Use a load balancer or DNS to gradually shift traffic:

1. **DNS-based migration:**
   ```bash
   # Update DNS TTL to 60 seconds
   # Gradually change DNS records to point to ADQ Pingora
   ```

2. **Load balancer migration:**
   ```bash
   # Configure upstream load balancer to send traffic to both
   # Gradually increase ADQ Pingora weight
   ```

## Configuration Conversion Examples

### Complex Nginx Site

**Original nginx configuration:**
```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name myapp.com www.myapp.com;
    
    ssl_certificate /etc/letsencrypt/live/myapp.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/myapp.com/privkey.pem;
    
    # Security headers
    add_header X-Frame-Options SAMEORIGIN;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    
    # Rate limiting
    limit_req_zone $binary_remote_addr zone=login:10m rate=5r/m;
    limit_req_zone $binary_remote_addr zone=api:10m rate=100r/s;
    
    # Main application
    location / {
        proxy_pass http://web_app;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    # API with rate limiting
    location /api/ {
        limit_req zone=api burst=200 nodelay;
        proxy_pass http://api_servers;
        proxy_set_header Host $host;
        
        # CORS headers
        add_header Access-Control-Allow-Origin *;
        add_header Access-Control-Allow-Methods "GET, POST, PUT, DELETE";
        add_header Access-Control-Allow-Headers "Content-Type, Authorization";
    }
    
    # Login endpoint with strict rate limiting
    location /auth/login {
        limit_req zone=login burst=10;
        proxy_pass http://auth_service;
        proxy_set_header Host $host;
    }
    
    # Static files
    location /static/ {
        proxy_pass http://static_files;
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}

upstream web_app {
    server 127.0.0.1:3000;
    server 127.0.0.1:3001;
    server 127.0.0.1:3002;
}

upstream api_servers {
    server 127.0.0.1:8080 weight=3;
    server 127.0.0.1:8081 weight=2;
    server 127.0.0.1:8082 weight=1;
}

upstream auth_service {
    server 127.0.0.1:9000;
    server 127.0.0.1:9001 backup;
}

upstream static_files {
    server 127.0.0.1:8090;
}
```

**Converted ADQ Pingora configuration:**
```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name myapp.com www.myapp.com;
    
    # SSL certificates (Let's Encrypt auto-detected)
    ssl_certificate /etc/letsencrypt/live/myapp.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/myapp.com/privkey.pem;
    
    # Security headers are automatically added
    
    # Main application
    location / {
        proxy_pass web_app;
        # Headers are automatically set
    }
    
    # API with rate limiting and CORS
    location /api/ {
        proxy_pass api_servers;
        rate_limit 100 200;  # 100 req/s, burst 200
        cors_enable;         # Automatic CORS headers
    }
    
    # Login endpoint with strict rate limiting
    location /auth/login {
        proxy_pass auth_service;
        rate_limit 5 10;     # 5 req/min converted to req/s: ~0.08, but minimum is 1
    }
    
    # Static files
    location /static/ {
        proxy_pass static_files;
        # Caching handled in main config
    }
}

upstream web_app {
    server 127.0.0.1:3000;
    server 127.0.0.1:3001;
    server 127.0.0.1:3002;
    # Automatic health checks and load balancing
}

upstream api_servers {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;
    # Weights not supported yet, but round-robin is automatic
}

upstream auth_service {
    server 127.0.0.1:9000;
    server 127.0.0.1:9001;
    # Backup servers handled automatically by health checks
}

upstream static_files {
    server 127.0.0.1:8090;
}
```

**Main configuration (`/etc/adq-pingora/proxy.yaml`):**
```yaml
version: 1

global:
  default_timeout: 30
  max_retries: 3
  health_check_interval: 5

security:
  headers:
    x_frame_options: "SAMEORIGIN"
    x_content_type_options: "nosniff"
    x_xss_protection: "1; mode=block"
    strict_transport_security: "max-age=31536000; includeSubDomains"

cache:
  enabled: true
  default_ttl: 300
  rules:
    - path: "/static/*"
      ttl: 31536000  # 1 year

logging:
  format: "json"
  level: "info"
  access_log:
    enabled: true
    path: "/var/log/adq-pingora/access.log"
```

## Migration Checklist

### Pre-Migration
- [ ] Backup nginx configuration
- [ ] Document current nginx features in use
- [ ] Install ADQ Pingora
- [ ] Convert configuration files
- [ ] Test ADQ Pingora on alternate ports

### During Migration
- [ ] Stop nginx service
- [ ] Update ADQ Pingora to use standard ports
- [ ] Start ADQ Pingora service
- [ ] Verify all endpoints work
- [ ] Check SSL certificates
- [ ] Test rate limiting
- [ ] Verify upstream health checks

### Post-Migration
- [ ] Monitor logs for errors
- [ ] Check performance metrics
- [ ] Verify all applications work correctly
- [ ] Update monitoring systems
- [ ] Update documentation
- [ ] Disable nginx service

## Common Issues and Solutions

### 1. Port Conflicts

**Problem:** ADQ Pingora can't bind to ports 80/443

**Solution:**
```bash
# Make sure nginx is stopped
sudo systemctl stop nginx
sudo systemctl status nginx

# Check what's using the ports
sudo netstat -tlnp | grep :80
sudo netstat -tlnp | grep :443
```

### 2. SSL Certificate Issues

**Problem:** SSL certificates not found

**Solution:**
```bash
# Check certificate paths
ls -la /etc/ssl/certs/example.com.crt
ls -la /etc/ssl/private/example.com.key

# For Let's Encrypt
ls -la /etc/letsencrypt/live/example.com/

# Fix permissions
sudo chmod 644 /etc/ssl/certs/example.com.crt
sudo chmod 600 /etc/ssl/private/example.com.key
```

### 3. Upstream Connection Issues

**Problem:** Can't connect to upstream servers

**Solution:**
```bash
# Test upstream connectivity
telnet 127.0.0.1 8080
curl http://127.0.0.1:8080/health

# Check ADQ Pingora logs
sudo journalctl -u adq-pingora -f
```

### 4. Configuration Syntax Errors

**Problem:** Configuration test fails

**Solution:**
```bash
# Test configuration
sudo adq-pingora -t

# Check for common issues:
# - Missing semicolons (not needed in ADQ Pingora)
# - Unsupported directives
# - Incorrect upstream references
```

## Performance Comparison

After migration, you should see improvements in:

### Memory Usage
```bash
# Check memory usage
ps aux | grep nginx
ps aux | grep adq-pingora

# ADQ Pingora typically uses 30-50% less memory
```

### Request Handling
```bash
# Benchmark with ab or wrk
ab -n 10000 -c 100 http://example.com/
wrk -t12 -c400 -d30s http://example.com/

# ADQ Pingora typically shows:
# - 20-40% better throughput
# - Lower latency
# - Better connection reuse
```

### Resource Efficiency
- Lower CPU usage under load
- Better memory management
- More efficient connection handling
- Built-in health checks reduce failed requests

## Rollback Plan

If issues occur, you can quickly rollback:

```bash
# Stop ADQ Pingora
sudo systemctl stop adq-pingora

# Start nginx
sudo systemctl start nginx

# Verify nginx is working
curl http://example.com/
sudo systemctl status nginx
```

## Getting Help

If you encounter issues during migration:

1. Check the [troubleshooting guide](troubleshooting.md)
2. Review logs: `sudo journalctl -u adq-pingora -f`
3. Test configuration: `sudo adq-pingora -t`
4. Compare with working nginx config
5. Open an issue on GitHub with your configuration