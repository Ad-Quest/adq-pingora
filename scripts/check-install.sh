#!/bin/bash
# Quick installation check script

echo "=== ADQ Pingora Installation Check ==="

# Check if binary exists
if command -v adq-pingora >/dev/null 2>&1; then
    echo "✓ adq-pingora binary found"
    adq-pingora --version
else
    echo "✗ adq-pingora binary not found"
    exit 1
fi

# Check configuration
if [ -f "/etc/adq-pingora/proxy.yaml" ]; then
    echo "✓ Configuration file found"
else
    echo "✗ Configuration file missing"
    exit 1
fi

# Test configuration
echo "Testing configuration..."
if sudo adq-pingora -t >/dev/null 2>&1; then
    echo "✓ Configuration is valid"
else
    echo "✗ Configuration has errors"
    sudo adq-pingora -t
    exit 1
fi

# Check if service is available
if systemctl list-unit-files | grep -q adq-pingora.service; then
    echo "✓ Systemd service installed"
else
    echo "✗ Systemd service not found"
    exit 1
fi

# Check if default site is enabled
if [ -L "/etc/adq-pingora/sites-enabled/default" ]; then
    echo "✓ Default site is enabled"
else
    echo "! Default site not enabled (this is optional)"
fi

echo ""
echo "Installation appears to be successful!"
echo "You can now start the service with: sudo systemctl start adq-pingora"
echo "Test with: curl http://localhost:8080/health"