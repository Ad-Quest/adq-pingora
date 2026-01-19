#!/bin/bash
# ADQ Pingora installation script

set -e

echo "Installing ADQ Pingora..."

# Build the project
echo "Building ADQ Pingora..."
cargo build --release

# Create directories
echo "Creating directories..."
sudo mkdir -p /etc/adq-pingora/sites-available
sudo mkdir -p /etc/adq-pingora/sites-enabled
sudo mkdir -p /var/log/adq-pingora
sudo chmod 755 /var/log/adq-pingora
sudo chown nobody:nogroup /var/log/adq-pingora

# Copy binary
echo "Installing binary..."
sudo cp target/release/adq-pingora /usr/local/bin/
sudo chmod +x /usr/local/bin/adq-pingora

# Copy configuration
echo "Installing configuration..."
sudo cp config/proxy.yaml /etc/adq-pingora/
sudo cp conf.yaml /etc/adq-pingora/

# Copy example site configuration
sudo cp sites-available/example.com /etc/adq-pingora/sites-available/

# Copy management scripts
echo "Installing management scripts..."
sudo cp scripts/adq-ensite /usr/local/bin/
sudo cp scripts/adq-dissite /usr/local/bin/
sudo chmod +x /usr/local/bin/adq-ensite
sudo chmod +x /usr/local/bin/adq-dissite

# Install systemd service
echo "Installing systemd service..."
sudo cp adq-pingora.service /etc/systemd/system/
sudo systemctl daemon-reload

echo "ADQ Pingora installed successfully!"
echo ""
echo "Next steps:"
echo "1. Configure your sites in /etc/adq-pingora/sites-available/"
echo "2. Enable sites: sudo adq-ensite example.com"
echo "3. Test configuration: sudo adq-pingora -t"
echo "4. Start service: sudo systemctl start adq-pingora"
echo "5. Enable on boot: sudo systemctl enable adq-pingora"