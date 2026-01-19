#!/usr/bin/env node

const fs = require('fs-extra');
const { execSync } = require('child_process');
const os = require('os');

console.log('Uninstalling ADQ Pingora...');

const isGlobalInstall = process.env.npm_config_global === 'true';

if (!isGlobalInstall) {
    console.log('INFO: Local uninstall detected. Nothing to clean up.');
    process.exit(0);
}

try {
    // Stop service if running
    if (os.platform() === 'linux') {
        console.log('Stopping ADQ Pingora service...');
        try {
            execSync('systemctl stop adq-pingora', { stdio: 'ignore' });
            execSync('systemctl disable adq-pingora', { stdio: 'ignore' });
            console.log('  Service stopped and disabled');
        } catch (error) {
            console.log('  Service was not running');
        }

        // Remove systemd service
        const servicePath = '/etc/systemd/system/adq-pingora.service';
        if (fs.existsSync(servicePath)) {
            fs.removeSync(servicePath);
            console.log('  Removed systemd service');
            
            try {
                execSync('systemctl daemon-reload');
            } catch (error) {
                // Ignore
            }
        }
    }

    // Remove binaries
    console.log('Removing binaries...');
    const binaries = [
        '/usr/local/bin/adq-pingora',
        '/usr/local/bin/adq-ensite',
        '/usr/local/bin/adq-dissite'
    ];

    binaries.forEach(binary => {
        if (fs.existsSync(binary)) {
            fs.removeSync(binary);
            console.log(`  Removed: ${binary}`);
        }
    });

    // Ask about configuration removal
    console.log('\nWARNING: Configuration files preserved in /etc/adq-pingora/');
    console.log('         To remove completely, run: sudo rm -rf /etc/adq-pingora/');
    console.log('         Log files preserved in /var/log/adq-pingora/');
    console.log('         To remove logs, run: sudo rm -rf /var/log/adq-pingora/');

    console.log('\nADQ Pingora uninstalled successfully!');

} catch (error) {
    console.error('ERROR: Uninstall failed:', error.message);
    process.exit(1);
}