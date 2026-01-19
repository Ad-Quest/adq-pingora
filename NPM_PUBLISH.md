# Publishing ADQ Pingora to NPM

This guide explains how to publish ADQ Pingora to NPM registry for easy installation.

## Prerequisites

1. **NPM Account**: Create account at [npmjs.com](https://www.npmjs.com/)
2. **NPM CLI**: Install with `npm install -g npm`
3. **Login**: Run `npm login` and enter your credentials

## Publishing Steps

### 1. Prepare Package

```bash
# Ensure all files are committed
git add .
git commit -m "Prepare for NPM publication"

# Test package locally
npm pack
```

### 2. Version Management

```bash
# Update version (patch/minor/major)
npm version patch  # 1.0.0 -> 1.0.1
npm version minor  # 1.0.0 -> 1.1.0
npm version major  # 1.0.0 -> 2.0.0
```

### 3. Publish to NPM

```bash
# Dry run to check what will be published
npm publish --dry-run

# Publish to NPM registry
npm publish

# For scoped packages (if needed)
npm publish --access public
```

### 4. Verify Publication

```bash
# Check package info
npm info adq-pingora

# Test installation
npm install -g adq-pingora
```

## Package Structure

The NPM package includes:

- **Binary wrappers** (`bin/`) - Node.js wrappers for system binaries
- **Install scripts** (`scripts/`) - Pre/post install automation
- **Configuration** (`config/`, `sites-available/`) - Default configs
- **Documentation** (`docs/`, `README.md`) - User guides
- **Service files** (`adq-pingora.service`) - Systemd integration

## Installation Process

When users run `npm install -g adq-pingora`:

1. **Preinstall**: Checks system requirements (Rust, OS)
2. **Install**: Downloads package files
3. **Postinstall**: 
   - Builds Rust binary with `cargo build --release`
   - Creates system directories (`/etc/adq-pingora/`, `/var/log/adq-pingora/`)
   - Installs binary to `/usr/local/bin/adq-pingora`
   - Copies configuration files
   - Sets up systemd service (Linux)
   - Creates system user
   - Sets proper permissions

## User Experience

After installation, users can:

```bash
# Use like nginx
adq-pingora -t                    # Test configuration
adq-ensite example.com           # Enable site
adq-dissite example.com          # Disable site

# Service management
systemctl start adq-pingora     # Start service
systemctl enable adq-pingora    # Enable on boot
```

## Maintenance

### Updating Package

1. Make changes to source code
2. Update version: `npm version patch`
3. Commit changes: `git commit -am "Update to v1.0.1"`
4. Publish: `npm publish`
5. Push to git: `git push && git push --tags`

### Unpublishing (Emergency Only)

```bash
# Unpublish specific version (within 24 hours)
npm unpublish adq-pingora@1.0.0

# Deprecate version (preferred)
npm deprecate adq-pingora@1.0.0 "Use version 1.0.1 instead"
```

## Best Practices

1. **Semantic Versioning**: Follow semver (major.minor.patch)
2. **Testing**: Test installation on clean systems
3. **Documentation**: Keep README and docs updated
4. **Changelog**: Update CHANGELOG.md for each release
5. **Security**: Regularly update dependencies
6. **Permissions**: Ensure install scripts handle permissions correctly

## Troubleshooting

### Common Issues

1. **Permission Denied**: Users need sudo for global install
2. **Rust Not Found**: Users need to install Rust first
3. **Build Failures**: Check Cargo.toml dependencies
4. **Service Issues**: Verify systemd service file

### Support

- Check [NPM package page](https://www.npmjs.com/package/adq-pingora)
- Review installation logs
- Test on different Linux distributions
- Provide clear error messages in install scripts