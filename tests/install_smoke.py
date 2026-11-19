#!/usr/bin/env python3
import http.client
import os
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
from contextlib import contextmanager, ExitStack
from http.server import ThreadingHTTPServer
from pathlib import Path

from smoke import Upstream, request, wait_for_response


REPO = Path(__file__).resolve().parents[1]


def available_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


@contextmanager
def running_proxy(binary, config):
    with tempfile.TemporaryFile() as log:
        with subprocess.Popen([binary, "-c", config], stdout=log,
                              stderr=subprocess.STDOUT) as proxy:
            try:
                yield proxy
            except Exception:
                log.seek(0)
                print(log.read().decode(errors="replace"), file=sys.stderr)
                raise
            finally:
                if proxy.poll() is None:
                    proxy.kill()


def stage_install(binary, root, mode):
    subprocess.run(
        [REPO / "scripts/install-system.sh", binary, mode],
        env={**os.environ, "DESTDIR": str(root)}, check=True,
    )


def check_config(binary, config, valid):
    result = subprocess.run([binary, "-t", "-c", config], capture_output=True, text=True)
    assert (result.returncode == 0) == valid, result.stdout + result.stderr


def main(binary, root):
    stage_install(binary, root, "standalone")
    binary = root / "usr/local/bin/adqp"
    assert binary.resolve() == (root / "usr/local/bin/adq-pingora").resolve()
    config_dir = root / "etc/adq-pingora"
    config = config_dir / "proxy.yaml"
    enabled = config_dir / "sites-enabled"
    default = enabled / "default"
    assert default.is_symlink()
    check_config(binary, config, True)

    original = config.read_text()
    custom = original.replace("enabled: true\n    path:", "enabled: false\n    path:") + "\n# local edit\n"
    config.write_text(custom)
    port = available_port()
    default.write_text(default.read_text().replace("listen 8080", f"listen {port}"))
    with running_proxy(binary, config) as proxy:
        wait_for_response(proxy, port, "/health", "OK\n", "localhost")
        assert request(port, "/health", "localhost") == (200, "OK\n")

    default.unlink()
    malformed = enabled / "broken"
    malformed.write_text("server { listen 8080; location / {")
    check_config(binary, config, False)
    malformed.write_text("server { listen 8080; location / { proxy_pass missing; } }")
    check_config(binary, config, False)
    malformed.unlink()

    sites = config_dir / "sites-available"
    (sites / "custom").write_text(f'server {{ listen {port}; location / {{ return 200 "custom"; }} }}')
    env = {**os.environ, "DESTDIR": str(root)}
    commands = root / "usr/local/bin"
    subprocess.run([commands / "adq-ensite", "custom"], env=env, check=True, capture_output=True)
    stage_install(binary.resolve(), root, "npm")
    assert not default.exists()
    assert (enabled / "custom").is_symlink()
    assert config.read_text() == custom
    check_config(binary, config, True)
    subprocess.run([commands / "adq-dissite", "custom"], env=env, check=True, capture_output=True)
    assert not (enabled / "custom").exists()

    cert = root / "cert.pem"
    key = root / "key.pem"
    subprocess.run([
        "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
        "-subj", "/CN=localhost", "-addext", "subjectAltName=DNS:localhost",
        "-keyout", key, "-out", cert,
    ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)

    with ExitStack() as stack:
        backends = []
        for _ in range(2):
            server = stack.enter_context(ThreadingHTTPServer(("127.0.0.1", 0), Upstream))
            backends.append(server.server_port)
            worker = threading.Thread(target=server.serve_forever, daemon=True)
            worker.start()
            stack.callback(server.shutdown)
        http_port = available_port()
        tls_port = available_port()
        (enabled / "custom").write_text(f"""
server {{
    listen {http_port};
    listen {tls_port} ssl http2;
    server_name localhost;
    ssl_certificate {cert};
    ssl_certificate_key {key};
    location /api/one {{ proxy_pass first; }}
    location /api/two {{ proxy_pass second; }}
}}
upstream first {{ server 127.0.0.1:{backends[0]}; }}
upstream second {{ server 127.0.0.1:{backends[1]}; }}
""")
        check_config(binary, config, True)
        with running_proxy(binary, config) as proxy:
            wait_for_response(proxy, http_port, "/api/one", f"upstream {backends[0]}", "localhost")
            assert request(http_port, "/api/two", "localhost") == (
                200, f"upstream {backends[1]} received /api/two\n",
            )

            assert request(http_port, "/api/missing", "localhost")[0] == 404

            context = ssl.create_default_context(cafile=str(cert))
            connection = http.client.HTTPSConnection("localhost", tls_port, context=context, timeout=3)
            try:
                connection.request("GET", "/api/one")
                response = connection.getresponse()
                assert response.status == 200
                assert response.read().decode() == f"upstream {backends[0]} received /api/one\n"
            finally:
                connection.close()

    print("Clean install, config validation, upgrade preservation, named upstreams and HTTPS passed.")


if __name__ == "__main__":
    with tempfile.TemporaryDirectory(prefix="adq-install-smoke-") as temp:
        main(Path(sys.argv[1]).resolve(strict=True), Path(temp))
