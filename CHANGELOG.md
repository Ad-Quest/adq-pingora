# Changelog

All notable changes to ADQ Pingora will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

### Removed
- Obsolete `adq-pingora-1.0.0.tgz` build artifact from the repository

## [1.0.3] - 2026-11-26

### Added
- npm package renamed to `adqproxy` and published as `adqproxy@1.0.3`; the old `adq-pingora` package remains at 1.0.2
- `adqp` command alias for the main binary (`adq-pingora` remains available)
- CI workflow with a locked build, unit tests/compile checks and a proxy smoke test
- Weekly dependency audit workflow and Dependabot configuration for Cargo and npm
- Installation smoke tests in `tests/install_smoke.py`

### Changed
- Upgraded the Pingora stack from 0.6.0 to 0.9.1, Prometheus to 0.14 and reqwest to 0.13, and added `pingora-prometheus`; `Cargo.lock` is committed and builds use `--locked`
- System installations place binaries in `/usr/local/libexec/adq-pingora/`, with npm-owned command links in `/usr/local/bin`
- Updated documentation for the npm package rename, system install paths and Rust 1.95+/cmake requirements
- Refactored configuration parsing, proxy and TLS startup for the new Pingora APIs and install layout

### Fixed
- Install scripts preserve existing configuration and enabled sites, validate the new binary as the service user and keep npm command links separate from system binaries
- systemd unit and certificate access for the `adq-pingora` service user

## [1.0.2] - 2026-01-28

### Changed
- Maintenance version bump; no functional changes

## [1.0.1] - 2026-01-28

### Added
- npm package `adq-pingora` with system-wide install scripts and `adq-pingora`, `adq-ensite`, `adq-dissite` commands
- `scripts/check-install.sh` and the npm publishing guide (`NPM_PUBLISH.md`)
- Default site configuration and expanded installation documentation

### Fixed
- npm packaging configuration and systemd service setup
- Installation script handling and configuration defaults

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

---

## Version History

- **1.0.3** - Pingora 0.9.1, CI and dependency audit, system install fixes, npm package renamed to `adqproxy`
- **1.0.2** - Maintenance release (version bump only)
- **1.0.1** - npm packaging and installation improvements
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
- **Issues**: [GitHub Issues](https://github.com/Ad-Quest/adq-pingora/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Ad-Quest/adq-pingora/discussions)

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.