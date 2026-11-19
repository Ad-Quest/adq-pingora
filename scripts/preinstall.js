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

if (process.env.npm_config_global === 'true' && !process.env.DESTDIR) {
    if (platform !== 'linux' || !fs.existsSync('/run/systemd/system') || process.getuid() !== 0) {
        console.error('System installation requires root on Linux with a running systemd instance.');
        process.exit(1);
    }
}

// Check Node.js version
const nodeVersion = process.version;
const majorVersion = parseInt(nodeVersion.slice(1).split('.')[0]);
if (majorVersion < 14) {
    console.error(`ERROR: Node.js version ${nodeVersion} is not supported. Please upgrade to Node.js 14.0.0 or higher.`);
    console.error('You can update Node.js by running:');
    console.error('  curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -');
    console.error('  sudo apt-get install -y nodejs');
    process.exit(1);
}

// Check if Rust/Cargo is available
try {
    execSync('cargo --version', { stdio: 'ignore' });
    console.log('Rust/Cargo found');
} catch (error) {
    console.error('ERROR: Rust/Cargo not found. Please install Rust: https://rustup.rs/');
    console.error('You can install Rust by running:');
    console.error('  curl --proto \'=https\' --tlsv1.2 -sSf https://sh.rustup.rs | sh');
    console.error('  source ~/.cargo/env');
    process.exit(1);
}

// Check if cmake is available (needed for native dependencies)
try {
    execSync('cmake --version', { stdio: 'ignore' });
    console.log('cmake found');
} catch (error) {
    console.error('ERROR: cmake not found. Please install cmake:');
    console.error('  sudo apt-get install cmake  # On Ubuntu/Debian');
    console.error('  brew install cmake          # On macOS');
    process.exit(1);
}

console.log("System requirements check passed");
