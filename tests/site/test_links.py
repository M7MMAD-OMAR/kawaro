"""S-061 AC6: scripts/site/check-links.sh (new, standalone). Run with: python3 -I test_links.py

Contract for the script:
  check-links.sh [--online] [--base URL] [ROOT]
  ROOT (default: repo root) holds site/**/*.html, README*.md and the git repo.
  It extracts every https://github.com/.../releases/download/<tag>/<file> URL from
  ROOT/site/**/*.html and ROOT/README*.md.
  Offline (default): for each URL, `git -C ROOT tag -l <tag>` must print the tag;
    otherwise print the URL and exit 1.
  --online: HTTP request to every URL; on HTTP 404 print the URL and exit 1.
    --base URL replaces the scheme and host (https://github.com) for the request only,
    so tests can point it at a local server.
  Exit 0 when all links pass (also when there are no such URLs).
"""
import http.server
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402

SCRIPT = lib.REPO / "scripts/site/check-links.sh"
U = "https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v9.9.9/sbarhive-9.9.9-linux.deb"
V = "https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v9.9.9/sbarhive-9.9.9-windows-x64.msi"


def run(root, *args):
    return subprocess.run(["bash", str(SCRIPT), *args, str(root)], capture_output=True, text=True, timeout=60)


def mkroot(tag=None, readme_url=None):
    d = Path(tempfile.mkdtemp(prefix="links-", dir=lib.TMP))
    (d / "site").mkdir()
    (d / "site/index.html").write_text(f'<a href="{U}">deb</a> <a href="{V}">msi</a>')
    (d / "README.md").write_text(f"[x]({readme_url})\n" if readme_url else "no links\n")
    subprocess.run(["git", "init", "-q"], cwd=d, check=True)
    subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "x"], cwd=d, check=True)
    if tag:
        subprocess.run(["git", "tag", tag], cwd=d, check=True)
    return d


class Handler(http.server.BaseHTTPRequestHandler):
    missing = set()

    def do_HEAD(self):
        self.send_response(404 if self.path in self.missing else 200)
        self.end_headers()

    def do_GET(self):
        self.do_HEAD()

    def log_message(self, *a):
        pass


def server(missing):
    h = type("H", (Handler,), {"missing": set(missing)})
    s = http.server.ThreadingHTTPServer(("127.0.0.1", 0), h)
    threading.Thread(target=s.serve_forever, daemon=True).start()
    return s, f"http://127.0.0.1:{s.server_address[1]}"


class AC6Links(unittest.TestCase):
    def test_AC6_script_exists_and_is_bash(self):
        self.assertTrue(SCRIPT.is_file(), "scripts/site/check-links.sh missing")

    def test_AC6_offline_missing_tag_fails_and_prints_url(self):
        r = run(mkroot())
        self.assertEqual(r.returncode, 1, r.stdout + r.stderr)
        self.assertIn(U, r.stdout + r.stderr)

    def test_AC6_offline_tag_present_passes(self):
        r = run(mkroot(tag="v9.9.9"))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_AC6_offline_reads_readme_too(self):
        d = mkroot(tag="v9.9.9", readme_url=U.replace("v9.9.9", "v8.8.8").replace("9.9.9-linux", "8.8.8-linux"))
        r = run(d)
        self.assertEqual(r.returncode, 1)
        self.assertIn("v8.8.8", r.stdout + r.stderr)

    def test_AC6_no_urls_passes(self):
        d = Path(tempfile.mkdtemp(dir=lib.TMP))
        (d / "site").mkdir()
        (d / "site/index.html").write_text("<p>nothing</p>")
        (d / "README.md").write_text("x")
        subprocess.run(["git", "init", "-q"], cwd=d, check=True)
        self.assertEqual(run(d).returncode, 0)

    def test_AC6_online_404_fails_and_prints_url(self):
        srv, base = server(["/M7MMAD-OMAR/sbarhive/releases/download/v9.9.9/sbarhive-9.9.9-windows-x64.msi"])
        try:
            r = run(mkroot(tag="v9.9.9"), "--online", "--base", base)
        finally:
            srv.shutdown()
        self.assertEqual(r.returncode, 1, r.stdout + r.stderr)
        self.assertIn("windows-x64.msi", r.stdout + r.stderr)

    def test_AC6_online_200_passes(self):
        srv, base = server([])
        try:
            r = run(mkroot(tag="v9.9.9"), "--online", "--base", base)
        finally:
            srv.shutdown()
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_AC6_online_checks_readme_urls(self):
        srv, base = server(["/M7MMAD-OMAR/sbarhive/releases/download/v7/readme.deb"])
        try:
            d = mkroot(tag="v9.9.9", readme_url="https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v7/readme.deb")
            r = run(d, "--online", "--base", base)
        finally:
            srv.shutdown()
        self.assertEqual(r.returncode, 1)
        self.assertIn("readme.deb", r.stdout + r.stderr)

    def test_AC6_built_unpublished_site_passes_offline(self):
        r = run(lib.built("0"))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_AC6_built_published_site_passes_offline_with_tag(self):
        r = run(lib.built("1"))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
