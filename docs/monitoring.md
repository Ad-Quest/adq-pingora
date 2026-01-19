# Monitoring & Logging

ADQ Pingora provides comprehensive monitoring and logging capabilities with structured JSON logs, metrics, and health monitoring.

## Logging Configuration

### Main Configuration

Configure logging in `/etc/adq-pingora/proxy.yaml`:

```yaml
logging:
  format: "json"          # json or text
  level: "info"           # error, warn, info, debug, trace
  access_log:
    enabled: true
    path: "/var/log/adq-pingora/access.log"
    format: "json"
  error_log:
    enabled: true
    path: "/var/log/adq-pingora/error.log"
    format: "json"
  metrics:
    enabled: true
    endpoint: "/metrics"
    port: 9090
```

### Log Levels

- **error**: Only errors and critical issues
- **warn**: Warnings and errors
- **info**: General information, warnings, and errors (recommended)
- **debug**: Detailed debugging information
- **trace**: Very verbose debugging (development only)

## Log Formats

### JSON Format (Recommended)

Structured JSON logs for easy parsing:

```json
{
  "timestamp": "2024-01-19T10:30:00.123Z",
  "level": "INFO",
  "target": "adq_pingora::proxy",
  "message": "Request processed",
  "fields": {
    "method": "GET",
    "path": "/api/users",
    "status": 200,
    "duration_ms": 45,
    "client_ip": "192.168.1.100",
    "user_agent": "Mozilla/5.0...",
    "upstream": "user_service",
    "upstream_server": "127.0.0.1:8080"
  }
}
```

### Text Format

Human-readable format for development:

```
2024-01-19T10:30:00.123Z INFO adq_pingora::proxy: Request processed method=GET path=/api/users status=200 duration_ms=45
```

## Log Types

### Access Logs

Record all HTTP requests:

```json
{
  "timestamp": "2024-01-19T10:30:00.123Z",
  "level": "INFO",
  "message": "Request processed",
  "method": "GET",
  "path": "/api/users/123",
  "status": 200,
  "duration_ms": 45,
  "bytes_sent": 1024,
  "client_ip": "192.168.1.100",
  "user_agent": "curl/7.68.0",
  "referer": "https://example.com/dashboard",
  "upstream": "user_service",
  "upstream_server": "127.0.0.1:8080",
  "retries": 0
}
```

### Error Logs

Record errors and warnings:

```json
{
  "timestamp": "2024-01-19T10:30:15.456Z",
  "level": "ERROR",
  "message": "Upstream connection failed",
  "upstream": "user_service",
  "upstream_server": "127.0.0.1:8080",
  "error": "Connection refused",
  "client_ip": "192.168.1.100",
  "path": "/api/users/123"
}
```

### Health Check Logs

Monitor upstream server health:

```json
{
  "timestamp": "2024-01-19T10:30:30.789Z",
  "level": "WARN",
  "message": "Health check failed",
  "upstream": "user_service",
  "server": "127.0.0.1:8081",
  "error": "Connection timeout",
  "consecutive_failures": 3
}
```

### Rate Limiting Logs

Track rate limiting events:

```json
{
  "timestamp": "2024-01-19T10:30:45.012Z",
  "level": "WARN",
  "message": "Rate limit exceeded",
  "client_ip": "192.168.1.100",
  "path": "/api/users",
  "rate_limit": "100/200",
  "current_rate": 150,
  "action": "rejected"
}
```

## Viewing Logs

### Systemd Journal

View logs using journalctl:

```bash
# View all logs
sudo journalctl -u adq-pingora

# Follow logs in real-time
sudo journalctl -u adq-pingora -f

# View logs from last hour
sudo journalctl -u adq-pingora --since "1 hour ago"

# View only error logs
sudo journalctl -u adq-pingora -p err

# View logs with JSON formatting
sudo journalctl -u adq-pingora -o json-pretty
```

### Log Files

Direct file access:

```bash
# View access logs
sudo tail -f /var/log/adq-pingora/access.log

# View error logs
sudo tail -f /var/log/adq-pingora/error.log

# Search for specific patterns
sudo grep "Rate limit exceeded" /var/log/adq-pingora/access.log

# Parse JSON logs with jq
sudo tail -n 100 /var/log/adq-pingora/access.log | jq '.fields.status'
```

## Metrics and Monitoring

### Prometheus Metrics

ADQ Pingora exposes Prometheus metrics:

```bash
# Access metrics endpoint
curl http://localhost:9090/metrics
```

Available metrics:

```prometheus
# HTTP request counter
http_requests_total{method="GET",status="200",upstream="user_service"} 1234

# Request duration histogram
http_request_duration_seconds_bucket{le="0.1",upstream="user_service"} 800
http_request_duration_seconds_bucket{le="0.5",upstream="user_service"} 950
http_request_duration_seconds_bucket{le="1.0",upstream="user_service"} 990

# Rate limiting counter
rate_limit_hits_total{path="/api/users",action="allowed"} 5000
rate_limit_hits_total{path="/api/users",action="rejected"} 50

# Upstream connection counter
upstream_connections_total{upstream="user_service",server="127.0.0.1:8080",status="success"} 1000

# Active connections gauge
active_connections{upstream="user_service"} 25

# Retry attempts counter
retry_attempts_total{upstream="user_service",reason="connection_failed"} 10
```

### Health Monitoring

Monitor service health:

```bash
# Check service status
systemctl status adq-pingora

# Check if ports are listening
sudo netstat -tlnp | grep adq-pingora

# Test HTTP response
curl -I http://localhost:9080/health
```

## Log Analysis

### Common Log Analysis Tasks

#### Request Rate Analysis

```bash
# Requests per minute
sudo journalctl -u adq-pingora --since "1 hour ago" | \
grep "Request processed" | \
awk '{print $1 " " $2}' | \
cut -c1-16 | \
sort | uniq -c

# Top requested paths
sudo journalctl -u adq-pingora --since "1 hour ago" | \
grep "Request processed" | \
grep -o '"path":"[^"]*"' | \
sort | uniq -c | sort -nr | head -10

# Response status distribution
sudo journalctl -u adq-pingora --since "1 hour ago" | \
grep "Request processed" | \
grep -o '"status":[0-9]*' | \
sort | uniq -c
```

#### Error Analysis

```bash
# Error rate by upstream
sudo journalctl -u adq-pingora --since "1 hour ago" | \
grep "ERROR" | \
grep -o '"upstream":"[^"]*"' | \
sort | uniq -c | sort -nr

# Connection failures
sudo journalctl -u adq-pingora --since "1 hour ago" | \
grep "connection failed" | wc -l

# Rate limiting violations
sudo journalctl -u adq-pingora --since "1 hour ago" | \
grep "Rate limit exceeded" | \
grep -o '"client_ip":"[^"]*"' | \
sort | uniq -c | sort -nr
```

#### Performance Analysis

```bash
# Average response time
sudo journalctl -u adq-pingora --since "1 hour ago" | \
grep "Request processed" | \
grep -o '"duration_ms":[0-9]*' | \
cut -d: -f2 | \
awk '{sum+=$1; count++} END {print "Average:", sum/count "ms"}'

# Slow requests (>1000ms)
sudo journalctl -u adq-pingora --since "1 hour ago" | \
grep "Request processed" | \
grep '"duration_ms":[0-9][0-9][0-9][0-9]' | \
wc -l
```

## Log Rotation

### Systemd Journal Rotation

Configure journal retention:

```bash
# Edit journald configuration
sudo nano /etc/systemd/journald.conf

# Add these settings
SystemMaxUse=1G
SystemMaxFileSize=100M
SystemMaxFiles=10
MaxRetentionSec=1month
```

### File Log Rotation

Configure logrotate for file logs:

```bash
# Create logrotate configuration
sudo nano /etc/logrotate.d/adq-pingora

# Add configuration
/var/log/adq-pingora/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 nobody nogroup
    postrotate
        systemctl reload adq-pingora
    endscript
}
```

## Monitoring Setup

### Basic Monitoring Script

```bash
#!/bin/bash
# /usr/local/bin/adq-pingora-monitor.sh

LOG_FILE="/var/log/adq-pingora-monitor.log"
ALERT_EMAIL="admin@example.com"

# Check if service is running
if ! systemctl is-active --quiet adq-pingora; then
    echo "$(date): ADQ Pingora service is not running" >> $LOG_FILE
    echo "ADQ Pingora service is down" | mail -s "ADQ Pingora Alert" $ALERT_EMAIL
    exit 1
fi

# Check error rate (last 5 minutes)
ERROR_COUNT=$(journalctl -u adq-pingora --since "5 minutes ago" | grep "ERROR" | wc -l)
if [ $ERROR_COUNT -gt 10 ]; then
    echo "$(date): High error rate: $ERROR_COUNT errors in last 5 minutes" >> $LOG_FILE
    echo "High error rate detected: $ERROR_COUNT errors" | mail -s "ADQ Pingora Alert" $ALERT_EMAIL
fi

# Check if upstream servers are healthy
FAILED_HEALTH_CHECKS=$(journalctl -u adq-pingora --since "5 minutes ago" | grep "Health check failed" | wc -l)
if [ $FAILED_HEALTH_CHECKS -gt 5 ]; then
    echo "$(date): Multiple health check failures: $FAILED_HEALTH_CHECKS" >> $LOG_FILE
    echo "Multiple upstream health check failures" | mail -s "ADQ Pingora Alert" $ALERT_EMAIL
fi

echo "$(date): Monitoring check completed" >> $LOG_FILE
```

### Cron Job Setup

```bash
# Add to crontab
sudo crontab -e

# Run monitoring every 5 minutes
*/5 * * * * /usr/local/bin/adq-pingora-monitor.sh
```

### Integration with External Monitoring

#### Prometheus Integration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'adq-pingora'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
    metrics_path: /metrics
```

#### Grafana Dashboard

Create dashboards for:
- Request rate and response times
- Error rates by upstream
- Rate limiting statistics
- Upstream health status
- System resource usage

## Troubleshooting

### High Log Volume

If logs are too verbose:

```yaml
# Reduce log level
logging:
  level: "warn"  # Only warnings and errors
```

### Missing Logs

Check log file permissions:

```bash
# Fix permissions
sudo chown nobody:nogroup /var/log/adq-pingora/
sudo chmod 755 /var/log/adq-pingora/
sudo chmod 644 /var/log/adq-pingora/*.log
```

### Log Parsing Issues

For JSON log parsing:

```bash
# Install jq for JSON parsing
sudo apt install jq

# Parse logs with jq
sudo journalctl -u adq-pingora -o json | jq '.MESSAGE | fromjson'
```

## Best Practices

### 1. Use Structured Logging

Always use JSON format in production:

```yaml
logging:
  format: "json"
```

### 2. Set Appropriate Log Levels

- **Production**: `info` or `warn`
- **Development**: `debug`
- **Troubleshooting**: `trace` (temporarily)

### 3. Monitor Key Metrics

Focus on:
- Request rate and response times
- Error rates
- Upstream health
- Rate limiting effectiveness

### 4. Set Up Alerts

Create alerts for:
- Service downtime
- High error rates
- Upstream failures
- Disk space usage

### 5. Regular Log Analysis

Perform regular analysis to:
- Identify performance bottlenecks
- Detect security issues
- Optimize rate limiting
- Plan capacity