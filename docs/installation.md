# Installation Guide

## System Requirements

- **Operating System**: Linux (Ubuntu 20.04+, CentOS 8+, Debian 11+)
- **Architecture**: x86_64, ARM64
- **Memory**: Minimum 512MB RAM
- **Disk Space**: 100MB for installation

## Dependencies

### Rust Toolchain

ADQ Pingora requires Rust 1.70 or later:

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
sudo apt install build-essential pkg-config libssl-dev

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel pkg-config
```

## Installation Methods

### Method 1: Automated Installation (Recommended)

```bash
# Clone repository
git clone https://github.com/Ad-Quest/adquest-pingora-proxy.git
cd adquest-pingora-proxy

# Run installation script
sudo ./scripts/install.sh
```

This will:
- Build the binary in release mode
- Install to `/usr/local/bin/adq-pingora`
- Create configuration directories
- Install systemd service
- Set up management scripts

### Method 2: Manual Installation

```bash
# Build the project
cargo build --release

# Install binary
sudo cp target/release/adq-pingora /usr/local/bin/
sudo chmod +x /usr/local/bin/adq-pingora

# Create directories
sudo mkdir -p /etc/adq-pingora/{sites-available,sites-enabled}
sudo mkdir -p /var/log/adq-pingora
sudo chown nobody:nogroup /var/log/adq-pingora

# Install configuration
sudo cp config/proxy.yaml /etc/adq-pingora/
sudo cp sites-available/example.com /etc/adq-pingora/sites-available/

# Install management scripts
sudo cp scripts/adq-{en,dis}site /usr/local/bin/
sudo chmod +x /usr/local/bin/adq-{en,dis}site

# Install systemd service
sudo cp adq-pingora.service /etc/systemd/system/
sudo systemctl daemon-reload
```

## Post-Installation

### 1. Verify Installation

```bash
# Check version
adq-pingora --version

# Test configuration
adq-pingora -t
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

/usr/local/bin/             # Binaries
├── adq-pingora             # Main binary
├── adq-ensite              # Enable site script
└── adq-dissite             # Disable site script
```

## Troubleshooting

### Permission Issues

```bash
# Fix log directory permissions
sudo chown -R nobody:nogroup /var/log/adq-pingora
sudo chmod 755 /var/log/adq-pingora
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
cargo build --release
```

## Uninstallation

```bash
# Stop and disable service
sudo systemctl stop adq-pingora
sudo systemctl disable adq-pingora

# Remove files
sudo rm /usr/local/bin/adq-pingora
sudo rm /usr/local/bin/adq-{en,dis}site
sudo rm /etc/systemd/system/adq-pingora.service
sudo rm -rf /etc/adq-pingora
sudo rm -rf /var/log/adq-pingora

# Reload systemd
sudo systemctl daemon-reload
```