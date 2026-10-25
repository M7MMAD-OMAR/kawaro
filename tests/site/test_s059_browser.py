"""S-059 AC3 headless check: search and CSP on the staged upload set.

Serves .deploy-out/ (built by scripts/prepare-site-deploy.sh in a temp repo
copy) from a private local HTTPS server that applies the staged _headers rules
to every response (Manager decision D3), then drives headless Chromium with
Playwright. No live desktop, no network beyond 127.0.0.1.

Run WITHOUT -I (the user site holds playwright) and from a neutral cwd.
"""
import functools
import glob
import http.server
import os
import ssl
import subprocess
import sys
import threading
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import s059lib as S  # noqa: E402

CSP_HOOK = """
window.__csp = [];
document.addEventListener('securitypolicyviolation', e => {
  window.__csp.push(e.violatedDirective + ' ' + e.blockedURI);
});
"""


def make_handler(directory, rules):
    class H(http.server.SimpleHTTPRequestHandler):
        def __init__(self, *a, **kw):
            super().__init__(*a, directory=str(directory), **kw)

        def log_message(self, *a):
            pass

        def end_headers(self):
            path = self.path.split("?", 1)[0].split("#", 1)[0]
            for k, v in S.headers_for(rules, path).items():
                self.send_header(k, v)
            super().end_headers()

    return H


def start_server(directory, rules):
    cert = S.TMP / "s059-cert.pem"
    key = S.TMP / "s059-key.pem"
    subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
                    "-subj", "/CN=127.0.0.1", "-keyout", str(key), "-out", str(cert)],
                   check=True, capture_output=True)
    # Port 0: the kernel picks a free port, so no running server is disturbed.
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), make_handler(directory, rules))
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(str(cert), str(key))
    srv.socket = ctx.wrap_socket(srv.socket, server_side=True)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, f"https://127.0.0.1:{srv.server_address[1]}"


class AC3HeadlessSearch(unittest.TestCase):
    def test_AC3_headless_search_both_languages_zero_csp_violations(self):
        from playwright.sync_api import sync_playwright
        out = S.staged()
        hfile = out / "_headers"
        self.assertTrue(hfile.exists(), "no .deploy-out/_headers to serve")
        rules = S.parse_headers(hfile.read_text(encoding="utf-8"))
        srv, base = start_server(out, rules)
        found = sorted(glob.glob(os.path.expanduser("~/.cache/ms-playwright/chromium-*/chrome-linux64/chrome")))
        exe = os.environ.get("S061_CHROMIUM") or (found[-1] if found else None)
        pw = sync_playwright().start()
        browser = pw.chromium.launch(executable_path=exe)
        bad = []
        try:
            # Pagefind searches the index of the page language, so each query
            # runs on the page of its language.
            for path, q in (("/docs/start/", "تثبيت"), ("/en/docs/install/", "install")):
                ctx = browser.new_context(ignore_https_errors=True)
                ctx.set_default_timeout(15000)
                pg = ctx.new_page()
                console = []
                pg.on("console", lambda m: console.append(m.text) if (
                    "Content Security Policy" in m.text or "Refused to" in m.text) else None)
                pg.add_init_script(CSP_HOOK)
                resp = pg.goto(base + path, wait_until="load")
                csp = (resp.headers.get("content-security-policy") if resp else "") or ""
                if "default-src 'none'" not in csp:
                    bad.append(f"{path}: response has no staged CSP header")
                pg.fill("form.search input", q)
                n = 0
                try:
                    pg.wait_for_selector("form.search .results li a", timeout=15000)
                    n = pg.eval_on_selector_all("form.search .results li a", "els => els.length")
                except Exception:
                    n = 0
                pg.wait_for_timeout(300)
                viol = pg.evaluate("window.__csp || []") + console
                if viol:
                    bad.append(f"{path} q={q}: {len(viol)} CSP violations: {viol[:3]}")
                if n < 1:
                    bad.append(f"{path} q={q}: {n} results")
                ctx.close()
        finally:
            browser.close()
            pw.stop()
            srv.shutdown()
            srv.server_close()
        S.no_problems(self, bad)


if __name__ == "__main__":
    unittest.main(verbosity=2)
