# SSL/TLS Configuration

ADQ Pingora provides comprehensive SSL/TLS support with modern security features, SNI (Server Name Indication), and Let's Encrypt integration.

## Basic SSL Configuration

### Certificate Files

Place your SSL certificates in standard locations:

```bash
# Certificate files
/etc/ssl/certs/example.com.crt      # Certificate
/etc/ssl/private/example.com.key    # Private key

# Or Let's Encrypt location
/etc/letsencrypt/live/example.com/fullchain.pem
/etc/letsencrypt/live/example.com/privkey.pem
```

### Server Configuration

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name example.com www.example.com;
    
    # SSL certificate configuration
    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    
    location / {
        proxy_pass backend;
    }
}

upstream backend {
    server 127.0.0.1:8080;
}
```

## Let's Encrypt Integration

### Automatic Certificate Management

ADQ Pingora automatically detects Let's Encrypt certificates:

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name example.com;
    
    # ADQ Pingora will automatically use:
    # /etc/letsencrypt/live/example.com/fullchain.pem
    # /etc/letsencrypt/live/example.com/privkey.pem
    
    location / {
        proxy_pass backend;
    }
}
```

### Obtaining Let's Encrypt Certificates

```bash
# Install certbot
sudo apt install certbot

# Obtain certificate (standalone mode)
sudo certbot certonly --standalone -d example.com -d www.example.com

# Or use webroot mode
sudo certbot certonly --webroot -w /var/www/html -d example.com
```

### Certificate Renewal

Set up automatic renewal:

```bash
# Add to crontab
sudo crontab -e

# Add this line for automatic renewal
0 12 * * * /usr/bin/certbot renew --quiet && systemctl reload adq-pingora
```

## Multi-Domain SSL (SNI)

ADQ Pingora supports multiple SSL certificates for different domains:

```nginx
# First domain
server {
    listen 443 ssl http2;
    server_name example.com www.example.com;
    
    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    
    location / {
        proxy_pass example_backend;
    }
}

# Second domain
server {
    listen 443 ssl http2;
    server_name api.example.com;
    
    ssl_certificate /etc/ssl/certs/api.example.com.crt;
    ssl_certificate_key /etc/ssl/private/api.example.com.key;
    
    location / {
        proxy_pass api_backend;
    }
}

# Third domain with Let's Encrypt
server {
    listen 443 ssl http2;
    server_name blog.example.com;
    
    # Let's Encrypt certificates (auto-detected)
    
    location / {
        proxy_pass blog_backend;
    }
}
```

## HTTP to HTTPS Redirect

ADQ Pingora automatically redirects HTTP to HTTPS when SSL is configured:

```nginx
server {
    listen 80;              # HTTP
    listen 443 ssl http2;   # HTTPS
    server_name example.com;
    
    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    
    # HTTP requests are automatically redirected to HTTPS
    
    location / {
        proxy_pass backend;
    }
}
```

## SSL Security Configuration

ADQ Pingora includes secure SSL defaults in the main configuration:

```yaml
# In /etc/adq-pingora/proxy.yaml
security:
  headers:
    strict_transport_security: "max-age=31536000; includeSubDomains"
    x_frame_options: "SAMEORIGIN"
    x_content_type_options: "nosniff"
    x_xss_protection: "1; mode=block"
```

## Configuration Examples

### Simple HTTPS Site

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name mysite.com;
    
    ssl_certificate /etc/ssl/certs/mysite.com.crt;
    ssl_certificate_key /etc/ssl/private/mysite.com.key;
    
    location / {
        proxy_pass web_backend;
    }
}

upstream web_backend {
    server 127.0.0.1:3000;
}
```

### API Gateway with SSL

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name api.example.com;
    
    ssl_certificate /etc/ssl/certs/api.example.com.crt;
    ssl_certificate_key /etc/ssl/private/api.example.com.key;
    
    location /api/v1 {
        proxy_pass api_v1;
        rate_limit 100 200;
        cors_enable;
    }
    
    location /api/v2 {
        proxy_pass api_v2;
        rate_limit 200 400;
        cors_enable;
    }
}

upstream api_v1 {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
}

upstream api_v2 {
    server 127.0.0.1:9080;
    server 127.0.0.1:9081;
}
```

### Wildcard SSL Certificate

```nginx
server {
    listen 443 ssl http2;
    server_name *.example.com;
    
    # Wildcard certificate
    ssl_certificate /etc/ssl/certs/wildcard.example.com.crt;
    ssl_certificate_key /etc/ssl/private/wildcard.example.com.key;
    
    location / {
        proxy_pass wildcard_backend;
    }
}

upstream wildcard_backend {
    server 127.0.0.1:8080;
}
```

## Certificate Management

### Certificate Formats

ADQ Pingora supports standard certificate formats:

- **PEM Format** (recommended): `.crt`, `.pem`
- **Certificate Chain**: Full chain including intermediates

### Certificate Validation

Test your SSL configuration:

```bash
# Test configuration
adq-pingora -t

# Check certificate details
openssl x509 -in /etc/ssl/certs/example.com.crt -text -noout

# Test SSL connection
openssl s_client -connect example.com:443 -servername example.com
```

### Certificate Monitoring

Monitor certificate expiration:

```bash
# Check certificate expiration
openssl x509 -in /etc/ssl/certs/example.com.crt -noout -dates

# Create monitoring script
#!/bin/bash
CERT="/etc/ssl/certs/example.com.crt"
DAYS_UNTIL_EXPIRY=$(openssl x509 -in "$CERT" -noout -checkend $((30*24*3600)) && echo "OK" || echo "EXPIRING")

if [ "$DAYS_UNTIL_EXPIRY" = "EXPIRING" ]; then
    echo "Certificate expiring soon!" | mail -s "SSL Certificate Alert" admin@example.com
fi
```

## Troubleshooting

### Certificate Not Found

Check certificate paths and permissions:

```bash
# Verify certificate files exist
ls -la /etc/ssl/certs/example.com.crt
ls -la /etc/ssl/private/example.com.key

# Check permissions
sudo chmod 644 /etc/ssl/certs/example.com.crt
sudo chmod 600 /etc/ssl/private/example.com.key
sudo chown root:root /etc/ssl/certs/example.com.crt
sudo chown root:root /etc/ssl/private/example.com.key
```

### SSL Handshake Failures

Check certificate chain:

```bash
# Verify certificate chain
openssl verify -CAfile /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/example.com.crt

# Test SSL connection
curl -I https://example.com
```

### Mixed Content Issues

Ensure all resources are served over HTTPS:

```nginx
server {
    listen 443 ssl http2;
    server_name example.com;
    
    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    
    # Add security headers
    location / {
        proxy_pass backend;
        # HSTS header is automatically added
    }
}
```

### Let's Encrypt Issues

Common Let's Encrypt troubleshooting:

```bash
# Check certbot logs
sudo journalctl -u certbot

# Test certificate renewal
sudo certbot renew --dry-run

# Force certificate renewal
sudo certbot renew --force-renewal
```

## Best Practices

### 1. Use Strong Certificates

- Use at least 2048-bit RSA keys
- Consider ECC certificates for better performance
- Include full certificate chain

### 2. Automate Certificate Management

```bash
# Automated renewal script
#!/bin/bash
certbot renew --quiet
if [ $? -eq 0 ]; then
    systemctl reload adq-pingora
fi
```

### 3. Monitor Certificate Health

Set up monitoring for:
- Certificate expiration dates
- SSL handshake success rates
- Certificate chain validation

### 4. Security Headers

ADQ Pingora automatically adds security headers:
- `Strict-Transport-Security`
- `X-Frame-Options`
- `X-Content-Type-Options`
- `X-XSS-Protection`

### 5. Regular Security Updates

Keep certificates and security practices up to date:

```bash
# Update certificate store
sudo apt update && sudo apt upgrade ca-certificates

# Monitor security advisories
# Subscribe to security mailing lists for your certificate provider
```