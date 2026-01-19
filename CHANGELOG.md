# Changelog

All notable changes to ADQ Pingora will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2024-01-19

### Added
- **Core Features**
  - High-performance HTTP/HTTPS reverse proxy based on Cloudflare Pingora
  - Nginx-like configuration syntax with enhanced features
  - Automatic load balancing with round-robin algorithm
  - Built-in health checks for upstream servers
  - Advanced rate limiting with burst support
  - SSL/TLS support with SNI and Let's Encrypt integration
  - In-memory caching with TTL and path-based rules
  - Circuit breaker pattern for fault tolerance
  - CORS support with configurable origins
  - IP filtering with whitelist/blacklist support

- **Configuration System**
  - Two-tier configuration: global YAML + nginx-style site configs
  - Sites-available/sites-enabled structure like nginx
  - Configuration validation with `adq-pingora -t` command
  - Hot reload support without downtime

- **Management Tools**
  - `adq-ensite` - Enable site configurations
  - `adq-dissite` - Disable site configurations
  - Systemd service integration
  - Automated installation script

- **Monitoring & Logging**
  - Structured JSON logging with configurable levels
  - Access logs with detailed request information
  - Error logs with context and stack traces
  - Health check monitoring and alerts
  - Rate limiting event tracking
  - Prometheus metrics endpoint (optional)

- **Security Features**
  - Automatic security headers (HSTS, X-Frame-Options, etc.)
  - Rate limiting per location with burst support
  - IP-based filtering and blocking
  - SSL/TLS with modern cipher suites
  - CORS protection with origin validation

- **Performance Optimizations**
  - Async I/O with Rust's tokio runtime
  - Connection pooling and reuse
  - Efficient memory management
  - Automatic retry with exponential backoff
  - Circuit breaker for upstream protection

### Documentation
- Comprehensive installation guide
- Configuration reference with examples
- Load balancing documentation
- SSL/TLS setup guide
- Rate limiting configuration
- Monitoring and logging guide
- Migration guide from nginx
- Professional README with quick start

### Infrastructure
- MIT license
- Automated build and test pipeline
- Docker support (planned)
- Systemd service files
- Log rotation configuration
- Health check endpoints

## [Unreleased]

### Planned Features
- Docker container support
- Kubernetes deployment manifests
- Advanced load balancing algorithms (least connections, IP hash)
- WebSocket proxy support enhancement
- gRPC proxy support
- Dynamic configuration updates via API
- Web-based management interface
- Advanced metrics and alerting
- Plugin system for custom extensions

---

## Version History

- **1.0.0** - Initial release with full nginx-like functionality
- **0.x.x** - Development versions (not released)

## Migration Notes

### From Nginx
ADQ Pingora provides a smooth migration path from nginx with:
- Compatible configuration syntax
- Automatic header handling
- Enhanced rate limiting
- Built-in health checks
- Better performance and resource usage

See the [Migration Guide](docs/migration.md) for detailed instructions.

## Support

- **Documentation**: [docs/](docs/)
- **Issues**: [GitHub Issues](https://github.com/Ad-Quest/adquest-pingora-proxy/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Ad-Quest/adquest-pingora-proxy/discussions)

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.