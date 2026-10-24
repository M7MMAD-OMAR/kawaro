#!/usr/bin/env python3
"""Writes _headers and .well-known/security.txt into a publish folder.

Usage: site-headers.py OUT_DIR
The CSP allows only same origin files plus the one inline script, pinned by hash.
"""
import base64, hashlib, pathlib, re, sys

out = pathlib.Path(sys.argv[1])
hashes = set()
for f in out.rglob("*.html"):
    for m in re.finditer(r"<script(?![^>]*\bsrc=)([^>]*)>(.*?)</script>", f.read_text(encoding="utf-8"), re.S):
        if "json" not in m.group(1):
            d = hashlib.sha256(m.group(2).encode()).digest()
            hashes.add("'sha256-" + base64.b64encode(d).decode() + "'")
csp = "; ".join([
    "default-src 'none'",
    "script-src 'self' 'wasm-unsafe-eval' " + " ".join(sorted(hashes)),
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data:",
    "font-src 'self'",
    "connect-src 'self'",
    "manifest-src 'self'",
    "base-uri 'none'",
    "form-action 'self'",
    "frame-ancestors 'none'",
])
(out / "_headers").write_text(f"""/*
  Content-Security-Policy: {csp}
  Strict-Transport-Security: max-age=31536000; includeSubDomains
  X-Content-Type-Options: nosniff
  Referrer-Policy: no-referrer
  Permissions-Policy: accelerometer=(), camera=(), geolocation=(), gyroscope=(), microphone=(), payment=(), usb=()
  Cross-Origin-Opener-Policy: same-origin
  Cross-Origin-Resource-Policy: same-origin
""", encoding="utf-8")
wk = out / ".well-known"
wk.mkdir(exist_ok=True)
(wk / "security.txt").write_text("""Contact: https://github.com/M7MMAD-OMAR/sbarhive/security/advisories/new
Expires: 2027-10-25T00:00:00.000Z
Preferred-Languages: ar, en
Canonical: https://hive.sbarah.com/.well-known/security.txt
""", encoding="utf-8")
