"""Executable proof for ADR 0010 Part A. Run: python3 -I test_download.py
Local servers on 127.0.0.1 only. Nothing touches the internet."""
import hashlib, os, subprocess, sys, tempfile, threading, time, unittest
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # needed under python3 -I
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import downloader as dl

SIZE = 400_000
SRC = hashlib.sha256(b"seed").digest() * (SIZE // 32)  # deterministic 400000 bytes
SHA = hashlib.sha256(SRC).hexdigest()
OTHER = bytes(reversed(SRC))                      # same size, different hash
NOMARGIN = dict(margin=0)


class H(BaseHTTPRequestHandler):
    """Routes by path. HTTP/1.0 so a body without Content-Length ends on close."""
    def log_message(self, *a): pass

    def _range(self, body):
        rng = self.headers.get("Range")
        self.server.log.append((self.path, rng))
        if rng:
            start = int(rng.split("=")[1].split("-")[0])
            self.send_response(206)
            self.send_header("Content-Range", f"bytes {start}-{len(body)-1}/{len(body)}")
            return body[start:]
        self.send_response(200)
        return body

    def _send(self, body, cut=None, delay=0.0, chunk=65536, length=True):
        if length:
            self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        try:
            for i in range(0, len(body), chunk):
                if cut is not None and i >= cut:
                    self.connection.shutdown(2)  # drop the connection mid body
                    return
                self.wfile.write(body[i:i+chunk]); self.wfile.flush()
                if delay: time.sleep(delay)
        except (BrokenPipeError, ConnectionResetError): pass

    def do_GET(self):
        s, p = self.server, self.path
        if p == "/ok.bin":
            self._send(self._range(SRC))
        elif p == "/drop.bin":      # first plain request dies at 40 percent
            body = self._range(SRC)
            first = not self.headers.get("Range") and not s.dropped
            s.dropped = s.dropped or first
            self._send(body, cut=int(SIZE * .4) if first else None, chunk=10_000)
        elif p == "/bad.bin":       # right size, wrong content
            self._send(self._range(OTHER))
        elif p == "/short.bin":     # honest Content-Length, but short file
            self.server.log.append((p, None)); self.send_response(200)
            self._send(SRC[:int(SIZE * .7)])
        elif p == "/big.bin":       # larger than pinned
            self.server.log.append((p, None)); self.send_response(200)
            self._send(SRC + SRC)
        elif p == "/lie.bin":       # no Content-Length, endless-ish, throttled
            self.server.log.append((p, None)); self.send_response(200)
            self._send(SRC * 20, delay=0.05, chunk=50_000, length=False)
        elif p == "/exact-nolen.bin":  # honest body of exactly SIZE, no Content-Length
            self.server.log.append((p, None)); self.send_response(200)
            self._send(SRC, length=False)
        elif p == "/stall.bin":     # announces a body, then 1 byte per second
            self.server.log.append((p, None)); self.send_response(200)
            self.send_header("Content-Length", str(SIZE)); self.end_headers()
            try:
                for _ in range(60):
                    self.wfile.write(b"x"); self.wfile.flush(); time.sleep(1)
            except (BrokenPipeError, ConnectionResetError): pass
        elif p == "/slow.bin":      # steady 10 KB per 0.1 s, 400 KB takes 4 s
            self._send(self._range(SRC), delay=0.1, chunk=10_000)
        elif p.startswith("/hop/"): # /hop/N -> N-1 ... -> /hop/0 -> other server
            n = int(p[5:]); self.send_response(302)
            self.send_header("Location", f"http://127.0.0.1:{s.peer}/ok.bin" if n == 0
                             else f"http://127.0.0.1:{s.server_port}/hop/{n-1}")
            self.send_header("Content-Length", "0"); self.end_headers()
        elif p == "/to-http":       # 302 to a plain http URL (stand-in for a downgrade)
            self.send_response(302)
            self.send_header("Location", f"http://127.0.0.1:{s.peer}/ok.bin")
            self.send_header("Content-Length", "0"); self.end_headers()
        else:
            self.send_error(404)


def serve():
    s = ThreadingHTTPServer(("127.0.0.1", 0), H)
    s.log, s.dropped, s.peer = [], False, 0
    threading.Thread(target=s.serve_forever, daemon=True).start()
    return s


class Base(unittest.TestCase):
    def setUp(self):
        self.a, self.b = serve(), serve()
        self.a.peer, self.b.peer = self.b.server_port, self.a.server_port
        self.tmp = tempfile.TemporaryDirectory()
        self.dest = os.path.join(self.tmp.name, "model.gguf")
        self.reports = []

    def tearDown(self):
        for s in (self.a, self.b): s.shutdown(); s.server_close()
        self.tmp.cleanup()

    def url(self, path, srv=None): return f"http://127.0.0.1:{(srv or self.a).server_port}{path}"

    def get(self, path, size=SIZE, sha=SHA, **kw):
        kw = {**NOMARGIN, **kw}
        return dl.download(self.url(path), self.dest, size, sha, self.reports.append,
                           test=True, poll=.2, **kw)

    def final_absent(self): self.assertFalse(os.path.exists(self.dest))
    def part_absent(self): self.assertFalse(os.path.exists(self.dest + ".part"))


class T1Resume(Base):
    def test_resume_after_drop(self):
        r1 = self.get("/drop.bin")
        self.assertTrue(r1.startswith("curl-18"), r1)          # partial file
        part = os.path.getsize(self.dest + ".part")
        self.assertTrue(0 < part < SIZE, part); self.final_absent()
        self.assertEqual(self.get("/drop.bin"), "ok")
        self.assertEqual(dl.sha256_of(self.dest), SHA)
        ranges = [r for _, r in self.a.log if r]
        self.assertEqual(ranges, [f"bytes={part}-"], self.a.log)
        print(f"\n  T1 part after drop={part} bytes ({part*100//SIZE}%), log={self.a.log}")


class T2Guard(Base):
    def _refused(self, path, reason, **kw):
        self.assertTrue(self.get(path, **kw).startswith(reason))
        self.assertEqual(len(self.reports), 1)                  # reported once
        self.final_absent(); self.part_absent()

    def test_wrong_hash(self): self._refused("/bad.bin", "hash-mismatch")
    def test_truncated(self): self._refused("/short.bin", "size-mismatch")
    def test_larger_than_pinned(self): self._refused("/big.bin", "curl-63")

    def test_larger_than_pinned_without_curl_limit(self):
        self._refused("/big.bin", "size-mismatch", use_max_filesize=False)


class T3Atomic(Base):
    def test_final_appears_only_after_verification(self):
        seen, stop = [], threading.Event()
        def watch():
            while not stop.is_set():
                seen.append((os.path.exists(self.dest), os.path.exists(self.dest + ".part")))
                time.sleep(.05)
        t = threading.Thread(target=watch); t.start()
        self.assertEqual(self.get("/slow.bin"), "ok"); stop.set(); t.join()
        self.assertTrue(any(p for _, p in seen))                 # .part was visible
        self.assertFalse(any(f for f, p in seen if p))           # never both
        self.assertEqual(dl.sha256_of(self.dest), SHA); self.part_absent()
        # a corrupt download must not replace an existing good file either
        os.replace(self.dest, self.dest + ".old"); self.assertEqual(self.get("/bad.bin"), "hash-mismatch")
        self.final_absent()


class T4Redirect(Base):
    def test_redirect_allowed_across_ports(self):
        self.assertEqual(self.get("/hop/1"), "ok")
        self.assertEqual(self.b.log[-1][0], "/ok.bin")

    def test_redirect_count_bounded(self):
        r = self.get("/hop/3", max_redirs=2)
        self.assertTrue(r.startswith("curl-47"), r); self.final_absent()
        self.part_absent()

    def _curl(self, flags, path):
        return subprocess.run(["curl", "--disable", "--silent", "--show-error", "--fail",
                               "--location", *flags, "-o", os.devnull, self.url(path)],
                              capture_output=True, text=True)

    def test_flag_sets(self):
        prod = dl.build_args("https://x/f", "p", 1)
        test = dl.build_args("http://127.0.0.1/f", "p", 1, test=True)
        self.assertEqual(prod[prod.index("--proto") + 1], "=https")
        self.assertEqual(prod[prod.index("--proto-redir") + 1], "=https")
        self.assertNotEqual(prod, test)
        # production flags refuse an http URL outright
        r = self._curl(["--proto", "=https", "--proto-redir", "=https"], "/ok.bin")
        self.assertEqual(r.returncode, 1, r.stderr)
        # allow http for the first hop only: the redirect to http is refused
        r = self._curl(["--proto", "=http,https", "--proto-redir", "=https"], "/to-http")
        self.assertEqual(r.returncode, 1, r.stderr)
        print(f"\n  T4 redirect to http refused: rc={r.returncode} {r.stderr.strip()}")
        # test flags allow loopback http redirects
        r = self._curl(["--proto", "=http,https", "--proto-redir", "=http,https"], "/to-http")
        self.assertEqual(r.returncode, 0, r.stderr)

    def test_production_mode_refuses_http_url(self):
        self.assertEqual(dl.download(self.url("/ok.bin"), self.dest, SIZE, SHA,
                                     self.reports.append, **NOMARGIN), "not-https")
        self.assertEqual(self.a.log, [])


class T5Timeout(Base):
    def test_stall_is_aborted(self):
        t = time.time()
        r = self.get("/stall.bin", speed_limit=100, speed_time=2)
        self.assertTrue(r.startswith("curl-28"), r); dt = time.time() - t
        self.assertLess(dt, 10); print(f"\n  T5 stall aborted after {dt:.1f}s ({r})")
        self.assertTrue(os.path.exists(self.dest + ".part"))     # kept for resume

    def test_slow_steady_survives_what_max_time_kills(self):
        t = subprocess.run(["curl", "--silent", "--max-time", "2", "-o", os.devnull,
                            self.url("/slow.bin")])
        self.assertEqual(t.returncode, 28)                       # fixed total timeout kills it
        t0 = time.time()
        self.assertEqual(self.get("/slow.bin", speed_limit=1000, speed_time=2), "ok")
        print(f"\n  T5 steady download took {time.time()-t0:.1f}s, max-time 2 would kill it")


class T6Ceiling(Base):
    def test_content_length_larger_refused(self):
        r = self.get("/big.bin"); self.assertTrue(r.startswith("curl-63"), r)

    def test_curl_alone_stops_lying_server(self):
        r = self.get("/lie.bin"); print(f"\n  T6 --max-filesize vs no Content-Length: {r}")
        self.assertTrue(r.startswith("curl-63"), r); self.part_absent()

    def test_exact_size_without_content_length_is_accepted(self):
        r = self.get("/exact-nolen.bin"); print(f"\n  T6 exact size, no Content-Length: {r}")
        self.assertEqual(r, "ok")

    def test_part_size_watcher_stops_lying_server(self):
        r = self.get("/lie.bin", use_max_filesize=False)
        self.assertEqual(r, "oversize"); self.part_absent(); self.final_absent()


class T7Space(Base):
    def test_refuses_before_starting(self):
        fake = lambda d: type("U", (), {"free": 1_000_000})()
        r = dl.download(self.url("/ok.bin"), self.dest, 900_000, SHA, self.reports.append,
                        test=True, margin=256 * 1024, usage=fake)
        self.assertEqual(r, "no-space"); self.assertEqual(self.a.log, [])   # curl never ran
        self.assertTrue(dl.check_space(self.tmp.name, 10, margin=0))
        self.assertFalse(dl.check_space(self.tmp.name, 1 << 60, margin=0))  # real disk_usage
        self.assertTrue(dl.check_space(self.tmp.name, 900_000, 100_000, fake))


class T8Progress(Base):
    def test_poll_part_size_at_1hz(self):
        got = []
        r = dl.download(self.url("/slow.bin"), self.dest, SIZE, SHA, self.reports.append,
                        on_progress=lambda h, t: got.append(h * 100 // t), test=True,
                        poll=1.0, **NOMARGIN)
        self.assertEqual(r, "ok"); self.assertGreaterEqual(len(got), 3)
        self.assertEqual(got, sorted(got)); self.assertLessEqual(got[-1], 100)
        print(f"\n  T8 percent samples at 1 Hz: {got}")

    def test_curl_progress_bar_is_stderr_noise(self):
        p = subprocess.run(["curl", "--progress-bar", "-o", os.devnull, self.url("/slow.bin")],
                           capture_output=True, text=True)
        pcts = [x for x in p.stderr.replace("\r", "\n").split("\n") if "%" in x]
        print(f"\n  T8 --progress-bar lines={len(pcts)}, last={pcts[-1].strip()[-12:]!r}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
