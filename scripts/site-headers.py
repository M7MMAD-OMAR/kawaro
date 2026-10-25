#!/usr/bin/env python3
"""Writes _headers and .well-known/security.txt into a publish folder (S-059).

Usage: site-headers.py OUT_DIR
The values follow ADR 0007 section 7. The CSP allows only same origin files
plus the one inline script, pinned by its hash. security.txt expires 180 days
after the build date, so a stale deploy shows.
"""
import base64
import datetime
import hashlib
import pathlib
import re
import sys

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
    "style-src 'self'",
    "style-src-attr 'unsafe-inline'",
    "img-src 'self'",
    "media-src 'self'",
    "font-src 'self'",
    "connect-src 'self'",
    "worker-src 'self'",
    "manifest-src 'self'",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'self'",
    "frame-ancestors 'none'",
    "upgrade-insecure-requests",
])
# autoplay stays allowed for this origin: the muted hero film needs it (S-067).
permissions = ", ".join(("autoplay=(self)" if p == "autoplay" else f"{p}=()") for p in (
    "accelerometer", "autoplay", "browsing-topics", "camera", "clipboard-read",
    "display-capture", "geolocation", "gyroscope", "magnetometer", "microphone",
    "midi", "payment", "usb",
))
immutable = "public, max-age=31536000, immutable"
(out / "_headers").write_text(f"""/*
  Content-Security-Policy: {csp}
  Strict-Transport-Security: max-age=86400; includeSubDomains
  X-Content-Type-Options: nosniff
  Referrer-Policy: no-referrer
  Permissions-Policy: {permissions}
  Cross-Origin-Opener-Policy: same-origin
  Cross-Origin-Resource-Policy: same-origin
  X-Frame-Options: DENY

/fonts/*
  Cache-Control: public, max-age=2592000

/media/*
  Cache-Control: public, max-age=2592000

/pagefind/fragment/*
  Cache-Control: {immutable}

/pagefind/index/*
  Cache-Control: {immutable}

/pagefind/*.pf_meta
  Cache-Control: {immutable}
""", encoding="utf-8")
expires = datetime.datetime.now(datetime.timezone.utc).date() + datetime.timedelta(days=180)
wk = out / ".well-known"
wk.mkdir(exist_ok=True)
(wk / "security.txt").write_text(f"""Contact: https://github.com/M7MMAD-OMAR/kawaro/security/advisories/new
Expires: {expires.isoformat()}T00:00:00Z
Preferred-Languages: en, ar
Canonical: https://kawaro.sbarah.com/.well-known/security.txt
Policy: https://github.com/M7MMAD-OMAR/kawaro/blob/main/SECURITY.md
""", encoding="utf-8")
