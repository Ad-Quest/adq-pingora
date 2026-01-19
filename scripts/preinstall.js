#!/usr/bin/env node

const fs = require('fs');
const { execSync } = require('child_process');
const os = require('os');

console.log('Checking system requirements...');

// Check if running on supported OS
const platform = os.platform();
if (platform !== 'linux' && platform !== 'darwin') {
    console.error('ERROR: ADQ Pingora is only supported on Linux and macOS');
    process.exit(1);
}

// Check if Rust/Cargo is available
try {
    execSync('cargo --version', { stdio: 'ignore' });
    console.log('Rust/Cargo found');
} catch (error) {
    console.error('ERROR: Rust/Cargo not found. Please install Rust: https://rustup.rs/');
    process.exit(1);
}

// Check if running as root for global install
if (process.getuid && process.getuid() === 0) {
    console.log('Running with root privileges');
} else if (process.env.npm_config_global === 'true') {
    console.log('WARNING: Global install detected. You may need to run with sudo.');
}

console.log('System requirements check passed');