#!/bin/sh
# Deploy a built binary. DESTDIR stages files without touching systemd.
set -eu
umask 022

source_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
binary=$1
mode=$2
root=$(printenv DESTDIR || true)
if [ -z "$root" ]; then
    if [ "$(uname -s)" != Linux ] || [ "$(id -u)" -ne 0 ] || [ ! -d /run/systemd/system ]; then
        echo "System installation requires root on Linux with a running systemd instance." >&2
        exit 1
    fi
    if ! getent passwd adq-pingora >/dev/null; then
        useradd --system --user-group --home-dir /var/lib/adq-pingora --shell /bin/false adq-pingora
    fi
fi

config_dir="$root/etc/adq-pingora"
libexec="$root/usr/local/libexec/adq-pingora"
unit_dir="$root/etc/systemd/system"
mkdir -p "$libexec" "$unit_dir" "$config_dir/sites-available" "$config_dir/sites-enabled"

copy_default() {
    if [ ! -e "$2" ] && [ ! -L "$2" ]; then
        install -m 644 "$1" "$2"
    fi
}

fresh=false
if [ ! -e "$config_dir/proxy.yaml" ] && [ ! -L "$config_dir/proxy.yaml" ]; then
    fresh=true
fi
copy_default "$source_dir/config/proxy.yaml" "$config_dir/proxy.yaml"
for site in default example.com; do
    copy_default "$source_dir/sites-available/$site" "$config_dir/sites-available/$site"
done
if [ "$fresh" = true ] && [ ! -e "$config_dir/sites-enabled/default" ] && [ ! -L "$config_dir/sites-enabled/default" ]; then
    ln -s ../sites-available/default "$config_dir/sites-enabled/default"
fi

new_binary=$(mktemp "$libexec/.adq-pingora.XXXXXX")
trap 'rm -f "$new_binary"' EXIT
install -m 755 "$binary" "$new_binary"
if [ -z "$root" ]; then
    runuser -u adq-pingora -- "$new_binary" -t -c "$config_dir/proxy.yaml"
fi
mv -f "$new_binary" "$libexec/adq-pingora"
for command in adq-ensite adq-dissite; do
    install -m 755 "$source_dir/scripts/$command" "$libexec/$command"
done

# npm owns the public command links for an npm installation.
if [ "$mode" = standalone ]; then
    mkdir -p "$root/usr/local/bin"
    ln -sfn "../libexec/adq-pingora/adq-pingora" "$root/usr/local/bin/adqp"
    for command in adq-pingora adq-ensite adq-dissite; do
        ln -sfn "../libexec/adq-pingora/$command" "$root/usr/local/bin/$command"
    done
fi

install -m 644 "$source_dir/adq-pingora.service" "$unit_dir/adq-pingora.service"
if [ -z "$root" ]; then
    systemctl daemon-reload
    if [ "$fresh" = true ]; then
        systemctl enable --now adq-pingora
    else
        echo "To apply the upgrade: systemctl restart adq-pingora"
    fi
fi
