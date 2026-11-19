#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
env -u DESTDIR cargo build --locked --release --target-dir target
if [ "$(id -u)" -eq 0 ] || [ -n "$(printenv DESTDIR || true)" ]; then
    exec ./scripts/install-system.sh target/release/adq-pingora standalone
fi
exec sudo ./scripts/install-system.sh target/release/adq-pingora standalone
