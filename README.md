# ADQ Pingora

ADQ Pingora is a high-performance HTTP/HTTPS reverse proxy and load balancer based on Cloudflare's Pingora framework. It provides nginx-like configuration syntax with enhanced performance and modern features.

## Features

- **High Performance**: Built on Cloudflare Pingora for superior performance
- **Nginx-like Configuration**: Familiar configuration syntax and management tools
- **Load Balancing**: Round-robin with health checks and automatic failover
- **Rate Limiting**: Configurable per-location rate limiting with burst support
- **SSL/TLS**: Full SSL/TLS support with SNI and Let's Encrypt integration
- **Caching**: In-memory caching with TTL and path-based rules
- **Circuit Breaker**: Fault tolerance with automatic recovery
- **Monitoring**: Structured JSON logging and Prometheus metrics
- **Security**: IP filtering, CORS support, and security headers

## Quick Start

### Installation

#### Option 1: NPM (Recommended)

```bash
# Install globally with NPM
npm install -g adq-pingora

# Or with Yarn
yarn global add adq-pingora
```

#### Option 2: Manual Installation

```bash
# Clone the repository
git clone https://github.com/Ad-Quest/adq-pingora.git
cd adq-pingora

# Build and install
sudo ./scripts/install.sh
```

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source ~/.cargo/env
  ```
- **Node.js**: Required version 14.0.0+ (18.x recommended)
  ```bash
  # Ubuntu/Debian
  curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
  sudo apt-get install -y nodejs
  
  # macOS
  brew install node
  ```
- **cmake**: Required for building native dependencies
  ```bash
  # Ubuntu/Debian
  sudo apt-get install cmake
  
  # macOS
  brew install cmake
  ```
- **Linux/macOS**: Currently supported platforms

### Basic Configuration

Create a server configuration in `/etc/adq-pingora/sites-available/`:

```nginx
server {
    listen 80;
    listen 443 ssl http2;
    server_name example.com;
    
    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;
    
    location / {
        proxy_pass backend;
        rate_limit 100 200;  # 100 req/s, burst 200
    }
}

upstream backend {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
}
```

Enable the site:

```bash
sudo adq-ensite example.com
sudo systemctl start adq-pingora
```

### Quick Test

After installation, a default configuration is automatically enabled. Test it:

```bash
# Test configuration
sudo adq-pingora -t

# Start the service
sudo systemctl start adq-pingora

# Test the server (default listens on port 8080)
curl http://localhost:8080/health
curl http://localhost:8080/

# Check status
systemctl status adq-pingora
```

## Documentation

- [Installation Guide](docs/installation.md)
- [Configuration Reference](docs/configuration.md)
- [Load Balancing](docs/load-balancing.md)
- [SSL/TLS Setup](docs/ssl.md)
- [Rate Limiting](docs/rate-limiting.md)
- [Monitoring & Logging](docs/monitoring.md)
- [Migration from Nginx](docs/migration.md)

## Management Commands

```bash
# Test configuration
adq-pingora -t

# Enable/disable sites
adq-ensite example.com
adq-dissite example.com

# Service management
systemctl start adq-pingora
systemctl stop adq-pingora
systemctl reload adq-pingora
systemctl status adq-pingora
```

## Performance

ADQ Pingora delivers superior performance compared to traditional reverse proxies:

- **Lower Memory Usage**: Efficient memory management with Rust
- **Higher Throughput**: Async I/O with minimal overhead
- **Better Latency**: Optimized connection handling and reuse
- **Scalability**: Handles thousands of concurrent connections

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Support

- [Documentation](docs/)
- [Issues](https://github.com/Ad-Quest/adquest-pingora-proxy/issues)
- [Discussions](https://github.com/Ad-Quest/adquest-pingora-proxy/discussions)