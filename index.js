#!/usr/bin/env node

// ADQ Pingora NPM Package Entry Point
// This package provides system-wide installation of ADQ Pingora reverse proxy

const packageInfo = require('./package.json');

console.log(`ADQ Pingora v${packageInfo.version}`);
console.log('High-performance reverse proxy based on Cloudflare Pingora');
console.log('');
console.log('Usage:');
console.log('  adq-pingora [options]           - Start the proxy server');
console.log('  adq-pingora -t                  - Test configuration');
console.log('  adq-ensite <site>               - Enable a site');
console.log('  adq-dissite <site>              - Disable a site');
console.log('');
console.log('Documentation: https://github.com/Ad-Quest/adq-pingora/tree/main/docs');
console.log('Configuration: /etc/adq-pingora/');
console.log('');

if (process.argv.includes('--version') || process.argv.includes('-v')) {
    process.exit(0);
}

if (process.argv.includes('--help') || process.argv.includes('-h')) {
    console.log('For detailed help, run: adq-pingora --help');
    process.exit(0);
}

// If called directly, show usage
console.log('To start ADQ Pingora, use: adq-pingora');
console.log('For system service: systemctl start adq-pingora');