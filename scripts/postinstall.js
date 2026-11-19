#!/usr/bin/env node

const { execFileSync } = require('child_process');
const path = require('path');

if (process.env.npm_config_global !== 'true') {
    console.log('Local installation. Skipping system setup.');
    process.exit(0);
}

const packageDir = path.join(__dirname, '..');
const targetDir = path.join(packageDir, 'target');
const options = { cwd: packageDir, stdio: 'inherit' };

try {
    const buildEnv = { ...process.env };
    delete buildEnv.DESTDIR;
    execFileSync('cargo', ['build', '--locked', '--release', '--target-dir', targetDir], {
        ...options,
        env: buildEnv,
    });
    execFileSync('sh', [
        path.join(__dirname, 'install-system.sh'),
        path.join(targetDir, 'release', 'adq-pingora'),
        'npm',
    ], options);
} catch (error) {
    console.error('ADQ Pingora installation failed:', error.message);
    process.exit(1);
}
