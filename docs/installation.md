# Installation Guide

## System Requirements

- **Operating System**: Linux (Ubuntu 20.04+, CentOS 8+, Debian 11+)
- **Architecture**: x86_64, ARM64
- **Memory**: Minimum 512MB RAM
- **Disk Space**: 100MB for installation

## Dependencies

### Rust Toolchain

ADQ Pingora requires Rust 1.95 or later with the committed dependency versions:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Update to latest version
rustup update
```

### System Packages

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential cmake pkg-config libssl-dev

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install cmake openssl-devel pkg-config
```

## Installation Methods

### Method 1: Automated Installation (Recommended)

```bash
# Clone repository
git clone https://github.com/Ad-Quest/adquest-pingora-proxy.git
cd adquest-pingora-proxy

# Run installation script
./scripts/install.sh
```

This will:
- Build the binary in release mode
- Install to `/usr/local/bin/adq-pingora`
- Create configuration directories
- Install systemd service
- Set up management scripts

### Method 2: npm installation

On Linux with systemd, install the build dependencies above and make Node.js,
Cargo and CMake available on the root PATH.

```bash
sudo npm install -g adqproxy
curl --fail http://127.0.0.1:8080/health
```

The installer creates the service account, installs the service and enables
the default site on a fresh installation. The system binary lives under
`/usr/local/libexec/adq-pingora`. npm owns its command wrappers. Use `adqp` for configuration checks and other
CLI commands. `adq-pingora` remains available as an alias.

When migrating from the old npm package, remove it first so it releases its
command links, then install the new package and restart the service:

```bash
sudo npm uninstall -g adq-pingora
sudo npm install -g adqproxy
sudo systemctl enable adq-pingora
sudo -u adq-pingora adqp -t && sudo systemctl restart adq-pingora
```

Existing files under `/etc/adq-pingora` and enabled-site links are preserved.
Before applying a site change or an upgrade, validate as the service user.
The restart can interrupt traffic.

```bash
sudo -u adq-pingora adqp -t
sudo systemctl restart adq-pingora
```

For TLS sites, grant the service user read access to the certificate and key
and traversal access to their parent directories.

## Post-Installation

### 1. Verify Installation

```bash
# Check version
adqp --version

# Test configuration
adqp -t
```

### 2. Configure Firewall

```bash
# UFW (Ubuntu)
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Firewalld (CentOS)
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --reload
```

### 3. Enable Service

```bash
sudo systemctl enable adq-pingora
sudo systemctl start adq-pingora
```

## Directory Structure

After installation, the following directories are created:

```
/etc/adq-pingora/
├── proxy.yaml              # Main configuration
├── sites-available/         # Available site configurations
│   └── example.com         # Example configuration
└── sites-enabled/          # Enabled sites (symlinks)

/var/log/adq-pingora/       # Log files
├── access.log              # Access logs
└── error.log               # Error logs

/usr/local/libexec/adq-pingora/ # Installed executables
├── adq-pingora             # Main binary
├── adq-ensite              # Enable site script
└── adq-dissite             # Disable site script
```

## Troubleshooting

### Permission Issues

```bash
# Fix log directory permissions
sudo journalctl -u adq-pingora -n 50
sudo -u adq-pingora adqp -t
```

### Port Binding Issues

```bash
# Check if ports are in use
sudo netstat -tlnp | grep :80
sudo netstat -tlnp | grep :443

# Stop conflicting services
sudo systemctl stop nginx
sudo systemctl stop apache2
```

### Build Issues

```bash
# Update Rust toolchain
rustup update

# Clean build cache
cargo clean
cargo build --locked --release
```

## Uninstallation

For an npm installation, stop and disable the service before removing the
package. Remove the systemd unit and installed executables separately.
Site configuration and logs are retained.

```bash
sudo systemctl disable --now adq-pingora
sudo npm uninstall -g adqproxy
sudo rm -f /etc/systemd/system/adq-pingora.service
sudo rm -f /usr/local/libexec/adq-pingora/{adq-pingora,adq-ensite,adq-dissite}
sudo systemctl daemon-reload
```
