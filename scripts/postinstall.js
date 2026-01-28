#!/usr/bin/env node

const fs = require('fs-extra');
const { execSync } = require('child_process');
const path = require('path');
const os = require('os');

console.log('Installing ADQ Pingora...');

const isGlobalInstall = process.env.npm_config_global === 'true';
const isRoot = process.getuid && process.getuid() === 0;

if (!isGlobalInstall) {
    console.log('INFO: Local installation detected. Skipping system setup.');
    console.log('      For system-wide installation, use: npm install -g adq-pingora');
    process.exit(0);
}

try {
    // Build the Rust binary
    console.log('Building ADQ Pingora binary...');
    execSync('cargo build --release', { 
        stdio: 'inherit',
        cwd: __dirname + '/..'
    });

    // Create system directories
    console.log('Creating system directories...');
    const dirs = [
        '/etc/adq-pingora',
        '/etc/adq-pingora/sites-available',
        '/etc/adq-pingora/sites-enabled',
        '/var/log/adq-pingora',
        '/var/lib/adq-pingora',
        '/etc/letsencrypt'  // Add missing directory for systemd service
    ];

    dirs.forEach(dir => {
        fs.ensureDirSync(dir);
        console.log(`  Created: ${dir}`);
    });

    // Copy binary to system location
    console.log('Installing binary...');
    const binarySource = path.join(__dirname, '..', 'target', 'release', 'adq-pingora');
    const binaryDest = '/usr/local/bin/adq-pingora';
    
    if (fs.existsSync(binarySource)) {
        fs.copySync(binarySource, binaryDest);
        fs.chmodSync(binaryDest, '755');
        console.log(`  Installed: ${binaryDest}`);
    } else {
        throw new Error('Binary not found after build');
    }

    // Copy management scripts
    console.log('Installing management scripts...');
    const scripts = ['adq-ensite', 'adq-dissite'];
    scripts.forEach(script => {
        const scriptSource = path.join(__dirname, '..', 'scripts', script);
        const scriptDest = `/usr/local/bin/${script}`;
        
        if (fs.existsSync(scriptSource)) {
            fs.copySync(scriptSource, scriptDest);
            fs.chmodSync(scriptDest, '755');
            console.log(`  Installed: ${scriptDest}`);
        }
    });

    // Copy configuration files
    console.log('Installing configuration files...');
    const configFiles = [
        { src: 'config/proxy.yaml', dest: '/etc/adq-pingora/proxy.yaml' },
        { src: 'sites-available/example.com', dest: '/etc/adq-pingora/sites-available/example.com' },
        { src: 'sites-available/default', dest: '/etc/adq-pingora/sites-available/default' }
    ];

    configFiles.forEach(({ src, dest }) => {
        const srcPath = path.join(__dirname, '..', src);
        if (fs.existsSync(srcPath)) {
            fs.copySync(srcPath, dest);
            console.log(`  Installed: ${dest}`);
        }
    });

    // Enable default site automatically
    console.log('Enabling default site...');
    const defaultSiteEnabled = '/etc/adq-pingora/sites-enabled/default';
    const defaultSiteAvailable = '/etc/adq-pingora/sites-available/default';
    if (fs.existsSync(defaultSiteAvailable) && !fs.existsSync(defaultSiteEnabled)) {
        fs.symlinkSync(defaultSiteAvailable, defaultSiteEnabled);
        console.log('  Default site enabled');
    }

    // Set proper permissions
    console.log('Setting permissions...');
    try {
        execSync('chown -R root:root /etc/adq-pingora');
        execSync('chown -R nobody:nogroup /var/log/adq-pingora');
        execSync('chown -R nobody:nogroup /var/lib/adq-pingora');
        execSync('chmod 755 /etc/adq-pingora');
        execSync('chmod 644 /etc/adq-pingora/proxy.yaml');
        execSync('chmod 755 /etc/adq-pingora/sites-available');
        execSync('chmod 755 /etc/adq-pingora/sites-enabled');
        console.log('  Permissions set successfully');
    } catch (error) {
        console.warn('WARNING: Could not set all permissions. You may need to run as root.');
    }

    // Install systemd service (Linux only)
    if (os.platform() === 'linux') {
        console.log('Installing systemd service...');
        const serviceSource = path.join(__dirname, '..', 'adq-pingora.service');
        const serviceDest = '/etc/systemd/system/adq-pingora.service';
        
        if (fs.existsSync(serviceSource)) {
            fs.copySync(serviceSource, serviceDest);
            console.log(`  Installed: ${serviceDest}`);
            
            try {
                execSync('systemctl daemon-reload');
                console.log('  Systemd daemon reloaded');
            } catch (error) {
                console.warn('WARNING: Could not reload systemd daemon');
            }
        }
    }

    // Create adq-pingora user
    console.log('Creating adq-pingora user...');
    try {
        execSync('useradd -r -s /bin/false -d /var/lib/adq-pingora adq-pingora', { stdio: 'ignore' });
        console.log('  User adq-pingora created');
    } catch (error) {
        console.log('  User adq-pingora already exists or could not be created');
    }

    console.log('\nADQ Pingora installation completed successfully!');
    console.log('\nNext steps:');
    console.log('  1. Test installation: /usr/local/bin/check-install.sh (if available)');
    console.log('  2. Start service: systemctl start adq-pingora');
    console.log('  3. Test default site: curl http://localhost:8080/health');
    console.log('  4. Configure custom sites in /etc/adq-pingora/sites-available/');
    console.log('  5. Enable sites with: adq-ensite <site-name>');
    console.log('  6. Test configuration: adq-pingora -t');
    console.log('  7. Enable on boot: systemctl enable adq-pingora');
    console.log('\nDocumentation: https://github.com/Ad-Quest/adq-pingora/tree/main/docs');

} catch (error) {
    console.error('ERROR: Installation failed:', error.message);
    process.exit(1);
}