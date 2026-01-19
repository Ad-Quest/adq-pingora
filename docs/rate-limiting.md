# Rate Limiting

ADQ Pingora provides advanced rate limiting capabilities to protect your services from abuse and ensure fair resource usage.

## Basic Rate Limiting

Rate limiting is configured per location using the `rate_limit` directive:

```nginx
server {
    listen 80;
    server_name example.com;
    
    location /api {
        proxy_pass backend;
        rate_limit 100 200;  # 100 requests/second, burst 200
    }
}
```

## Rate Limit Syntax

```nginx
rate_limit <requests_per_second> [burst];
```

- **requests_per_second**: Maximum sustained rate
- **burst** (optional): Maximum burst size for handling traffic spikes

## Rate Limiting Modes

### 1. Simple Rate Limiting

Basic rate limiting without burst:

```nginx
location /api {
    proxy_pass backend;
    rate_limit 50;  # 50 requests/second, no burst
}
```

### 2. Rate Limiting with Burst

Allow traffic bursts while maintaining average rate:

```nginx
location /api {
    proxy_pass backend;
    rate_limit 100 200;  # 100 req/s sustained, 200 req burst
}
```

## Per-Location Configuration

Different locations can have different rate limits:

```nginx
server {
    listen 80;
    server_name api.example.com;
    
    # Public API - strict limits
    location /api/public {
        proxy_pass public_backend;
        rate_limit 10 20;
    }
    
    # Authenticated API - higher limits
    location /api/auth {
        proxy_pass auth_backend;
        rate_limit 100 200;
    }
    
    # Internal API - very high limits
    location /api/internal {
        proxy_pass internal_backend;
        rate_limit 1000 2000;
    }
    
    # Health checks - no limits
    location /health {
        proxy_pass health_backend;
        # No rate_limit directive = no limits
    }
}
```

## Rate Limiting Behavior

### Request Processing

1. **Under Limit**: Request processed immediately
2. **Burst Available**: Request processed, burst counter decremented
3. **Over Limit**: Request rejected with HTTP 429 (Too Many Requests)

### Response Headers

ADQ Pingora adds rate limiting headers to responses:

```http
HTTP/1.1 200 OK
X-Rate-Limit-Limit: 100
X-Rate-Limit-Remaining: 95
X-Rate-Limit-Reset: 1642608000
```

### Error Response

When rate limit is exceeded:

```http
HTTP/1.1 429 Too Many Requests
X-Rate-Limit-Limit: 100
X-Rate-Limit-Remaining: 0
X-Rate-Limit-Reset: 1642608000
Content-Type: application/json

{
  "error": "Rate limit exceeded",
  "message": "Too many requests. Please try again later.",
  "retry_after": 60
}
```

## Configuration Examples

### Web Application

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name myapp.com;
    
    ssl_certificate /etc/ssl/certs/myapp.com.crt;
    ssl_certificate_key /etc/ssl/private/myapp.com.key;
    
    # Main application - moderate limits
    location / {
        proxy_pass web_backend;
        rate_limit 50 100;
    }
    
    # Static assets - high limits
    location /static {
        proxy_pass static_backend;
        rate_limit 1000 2000;
    }
    
    # User uploads - strict limits
    location /upload {
        proxy_pass upload_backend;
        rate_limit 5 10;
    }
}

upstream web_backend {
    server 127.0.0.1:3000;
}

upstream static_backend {
    server 127.0.0.1:8080;
}

upstream upload_backend {
    server 127.0.0.1:9000;
}
```

### API Gateway

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name api.example.com;
    
    ssl_certificate /etc/ssl/certs/api.example.com.crt;
    ssl_certificate_key /etc/ssl/private/api.example.com.key;
    
    # Authentication endpoint - moderate limits
    location /auth {
        proxy_pass auth_service;
        rate_limit 20 40;
        cors_enable;
    }
    
    # User data API - standard limits
    location /api/users {
        proxy_pass user_service;
        rate_limit 100 200;
        cors_enable;
    }
    
    # Search API - higher limits for better UX
    location /api/search {
        proxy_pass search_service;
        rate_limit 200 400;
        cors_enable;
    }
    
    # Bulk operations - strict limits
    location /api/bulk {
        proxy_pass bulk_service;
        rate_limit 5 10;
        cors_enable;
    }
    
    # Webhooks - very strict limits
    location /webhooks {
        proxy_pass webhook_service;
        rate_limit 1 2;
    }
}

upstream auth_service {
    server 127.0.0.1:8001;
    server 127.0.0.1:8002;
}

upstream user_service {
    server 127.0.0.1:8010;
    server 127.0.0.1:8011;
}

upstream search_service {
    server 127.0.0.1:8020;
    server 127.0.0.1:8021;
    server 127.0.0.1:8022;
}

upstream bulk_service {
    server 127.0.0.1:8030;
}

upstream webhook_service {
    server 127.0.0.1:8040;
}
```

### E-commerce Platform

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name shop.example.com;
    
    ssl_certificate /etc/ssl/certs/shop.example.com.crt;
    ssl_certificate_key /etc/ssl/private/shop.example.com.key;
    
    # Product browsing - high limits
    location /api/products {
        proxy_pass product_service;
        rate_limit 200 400;
        cors_enable;
    }
    
    # Shopping cart - moderate limits
    location /api/cart {
        proxy_pass cart_service;
        rate_limit 50 100;
        cors_enable;
    }
    
    # Checkout process - strict limits
    location /api/checkout {
        proxy_pass checkout_service;
        rate_limit 10 20;
        cors_enable;
    }
    
    # Payment processing - very strict limits
    location /api/payment {
        proxy_pass payment_service;
        rate_limit 2 5;
        cors_enable;
    }
    
    # Order status - moderate limits
    location /api/orders {
        proxy_pass order_service;
        rate_limit 30 60;
        cors_enable;
    }
}
```

## Monitoring Rate Limits

### Log Analysis

Rate limiting events are logged:

```json
{
  "timestamp": "2024-01-19T10:30:00Z",
  "level": "WARN",
  "message": "Rate limit exceeded",
  "client_ip": "192.168.1.100",
  "path": "/api/users",
  "rate_limit": "100/200",
  "current_rate": "150"
}
```

### Metrics

Monitor rate limiting metrics:

```bash
# View rate limiting logs
sudo journalctl -u adq-pingora | grep "rate limit"

# Count rate limit violations
sudo journalctl -u adq-pingora --since "1 hour ago" | \
grep "Rate limit exceeded" | wc -l

# Top rate-limited IPs
sudo journalctl -u adq-pingora --since "1 hour ago" | \
grep "Rate limit exceeded" | \
grep -o '"client_ip":"[^"]*"' | \
sort | uniq -c | sort -nr
```

## Best Practices

### 1. Set Appropriate Limits

Consider your backend capacity:

```nginx
# If backend can handle 1000 req/s total
# And you have 10 concurrent users expected
location /api {
    proxy_pass backend;
    rate_limit 100 200;  # 100 req/s per client
}
```

### 2. Use Burst for User Experience

Allow bursts for better user experience:

```nginx
# Good: Allows quick page loads
location / {
    proxy_pass backend;
    rate_limit 10 50;  # 10 req/s sustained, 50 req burst
}

# Bad: Too strict, poor UX
location / {
    proxy_pass backend;
    rate_limit 10;  # No burst, every request after 10/s is rejected
}
```

### 3. Different Limits for Different Operations

```nginx
# Read operations - higher limits
location /api/read {
    proxy_pass backend;
    rate_limit 100 200;
}

# Write operations - lower limits
location /api/write {
    proxy_pass backend;
    rate_limit 20 40;
}

# Expensive operations - very low limits
location /api/reports {
    proxy_pass backend;
    rate_limit 2 5;
}
```

### 4. Monitor and Adjust

Regularly review rate limiting effectiveness:

```bash
# Create monitoring script
#!/bin/bash
echo "Rate Limiting Report - $(date)"
echo "================================"

# Total requests
TOTAL=$(journalctl -u adq-pingora --since "1 hour ago" | grep "Request:" | wc -l)
echo "Total requests: $TOTAL"

# Rate limited requests
LIMITED=$(journalctl -u adq-pingora --since "1 hour ago" | grep "Rate limit exceeded" | wc -l)
echo "Rate limited: $LIMITED"

# Rate limiting percentage
if [ $TOTAL -gt 0 ]; then
    PERCENTAGE=$(echo "scale=2; $LIMITED * 100 / $TOTAL" | bc)
    echo "Rate limiting percentage: $PERCENTAGE%"
fi

# Top rate-limited endpoints
echo -e "\nTop rate-limited endpoints:"
journalctl -u adq-pingora --since "1 hour ago" | \
grep "Rate limit exceeded" | \
grep -o '"path":"[^"]*"' | \
sort | uniq -c | sort -nr | head -5
```

## Troubleshooting

### High Rate Limiting

If too many requests are being rate limited:

1. **Check if limits are too strict**:
   ```nginx
   # Increase limits temporarily
   location /api {
       proxy_pass backend;
       rate_limit 200 400;  # Increased from 100 200
   }
   ```

2. **Analyze traffic patterns**:
   ```bash
   # Check request distribution
   journalctl -u adq-pingora --since "1 hour ago" | \
   grep "Request:" | \
   grep -o '"client_ip":"[^"]*"' | \
   sort | uniq -c | sort -nr
   ```

3. **Consider IP-based limits** (future feature)

### Rate Limiting Not Working

1. **Verify configuration**:
   ```bash
   adq-pingora -t
   ```

2. **Check logs for errors**:
   ```bash
   sudo journalctl -u adq-pingora | grep -i error
   ```

3. **Ensure location matches**:
   ```nginx
   # Make sure the location pattern matches your requests
   location /api {  # Matches /api/users, /api/orders, etc.
       rate_limit 100 200;
   }
   ```

### Performance Impact

Rate limiting has minimal performance impact, but monitor:

```bash
# Check CPU usage
top -p $(pgrep adq-pingora)

# Check memory usage
ps aux | grep adq-pingora
```