#!/usr/bin/env python3
import http.client
import socket
import subprocess
import sys
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class Upstream(BaseHTTPRequestHandler):
    def do_GET(self):
        body = f"upstream {self.server.server_port} received {self.path}\n".encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args):
        pass


def request(port, path, host=None):
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=1)
    try:
        connection.request("GET", path, headers={"Host": host} if host else {})
        response = connection.getresponse()
        return response.status, response.read().decode()
    finally:
        connection.close()


def wait_for_response(proxy, port, path, expected, host=None):
    deadline = time.monotonic() + 15
    last_result = None
    while time.monotonic() < deadline:
        if proxy.poll() is not None:
            raise RuntimeError(f"proxy exited with status {proxy.returncode}")
        try:
            status, body = request(port, path, host)
            if status == 200 and expected in body:
                return
            last_result = f"HTTP {status}, body={body[:500]!r}"
        except (OSError, http.client.HTTPException) as error:
            last_result = repr(error)
        time.sleep(0.1)
    raise RuntimeError(f"{path} on port {port} did not return {expected!r}: {last_result}")


def smoke(binary, upstream_port, directory):
    with socket.socket() as proxy_socket, socket.socket() as metrics_socket:
        proxy_socket.bind(("127.0.0.1", 0))
        metrics_socket.bind(("127.0.0.1", 0))
        proxy_port = proxy_socket.getsockname()[1]
        metrics_port = metrics_socket.getsockname()[1]

    sites = directory / "sites-enabled"
    sites.mkdir()
    (sites / "smoke.conf").write_text(f"""
upstream backend {{
    server 127.0.0.1:{upstream_port};
}}
server {{
    listen {proxy_port};
    server_name smoke.test;
    location /api/health {{ proxy_pass backend; }}
}}
""", encoding="utf-8")

    config = directory / "proxy.yaml"
    template = Path(__file__).resolve().parents[1] / "config/proxy.yaml"
    config.write_text(template.read_text(encoding="utf-8")
        .replace("enabled: true\n    path:", "enabled: false\n    path:")
        .replace("enabled: false\n    endpoint:", "enabled: true\n    endpoint:")
        .replace("port: 9091", f"port: {metrics_port}"), encoding="utf-8")

    log_path = directory / "proxy.log"
    with log_path.open("w", encoding="utf-8") as log:
        with subprocess.Popen([str(binary), "-c", str(config)], cwd=directory,
                              stdout=log, stderr=subprocess.STDOUT) as proxy:
            try:
                wait_for_response(proxy, proxy_port, "/api/health", f"upstream {upstream_port} received /api/health", "smoke.test")
                wait_for_response(
                    proxy, metrics_port, "/metrics",
                    'http_requests_total{method="GET",service="core_api",status="200"}',
                )
            except Exception:
                log_lines = log_path.read_text(encoding="utf-8").splitlines()
                print("\n".join(log_lines[-30:]), file=sys.stderr)
                raise
            finally:
                if proxy.poll() is None:
                    proxy.terminate()
                    try:
                        proxy.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        proxy.kill()


if __name__ == "__main__":
    with ThreadingHTTPServer(("127.0.0.1", 0), Upstream) as upstream:
        worker = threading.Thread(target=upstream.serve_forever)
        worker.start()
        try:
            with tempfile.TemporaryDirectory(prefix="adq-smoke-") as temp:
                smoke(Path(sys.argv[1]).resolve(strict=True), upstream.server_port, Path(temp))
        finally:
            upstream.shutdown()
            worker.join()
    print("Proxy forwarded the request and /metrics exposed application metrics.")
