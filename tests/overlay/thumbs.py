#!/usr/bin/env python3
"""Writes a freedesktop thumbnail for a file into a private thumbnail cache,
as a thumbnailer would (S-018 AC24 to AC27). Standard library only.

    thumbs.py FILE CACHE SIZE WIDTH HEIGHT [photo|noise]

FILE must exist; its path must need no URI escaping (ASCII letters, digits,
"/", ".", "_", "-"), so file:// plus the path is the GLib URI. CACHE is the
XDG_CACHE_HOME of a sandbox, never the owner's. SIZE is the cache folder
(normal, large, x-large, xx-large). The PNG is 8-bit RGBA with the tEXt
chunks Thumb::URI, Thumb::MTime (seconds) and Thumb::Size, at
CACHE/thumbnails/SIZE/MD5(URI).png. Pictures are deterministic:
  photo  flat sky, sun and ground bands (the snapshot picture)
  noise  a busy pattern that compresses poorly (decoding is not understated)
Prints the thumbnail path.
"""
import hashlib
import os
import re
import struct
import sys
import zlib

SKY = (70, 130, 200, 255)
SUN = (250, 200, 40, 255)
GROUND = (60, 150, 70, 255)


def chunk(kind, data):
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def pixel(pattern, x, y, w, h):
    if pattern == "noise":
        v = (x * 2654435761 ^ y * 40503 ^ (x * y * 97)) & 0xFFFFFF
        return (v & 255, (v >> 8) & 255, (v >> 16) & 255, 255)
    if y >= h * 0.62:
        return GROUND
    cx, cy, r = w * 0.68, h * 0.30, min(w, h) * 0.16
    if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
        return SUN
    return SKY


def main(argv):
    if len(argv) not in (6, 7):
        print(__doc__, file=sys.stderr)
        return 2
    path, cache, size = os.path.abspath(argv[1]), argv[2], argv[3]
    w, h = int(argv[4]), int(argv[5])
    pattern = argv[6] if len(argv) == 7 else "photo"
    if size not in ("normal", "large", "x-large", "xx-large") or pattern not in ("photo", "noise"):
        print("bad size or pattern", file=sys.stderr)
        return 2
    if not re.fullmatch(r"[A-Za-z0-9/._-]+", path):
        print(f"path needs URI escaping: {path}", file=sys.stderr)
        return 2
    st = os.stat(path)
    uri = "file://" + path
    out_dir = os.path.join(cache, "thumbnails", size)
    os.makedirs(out_dir, exist_ok=True)
    out = os.path.join(out_dir, hashlib.md5(uri.encode()).hexdigest() + ".png")

    rows = bytearray()
    for y in range(h):
        rows.append(0)
        for x in range(w):
            rows.extend(pixel(pattern, x, y, w, h))
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
    for key, value in (("Thumb::URI", uri), ("Thumb::MTime", str(int(st.st_mtime))),
                       ("Thumb::Size", str(st.st_size))):
        png += chunk(b"tEXt", key.encode("latin-1") + b"\0" + value.encode("latin-1"))
    png += chunk(b"IDAT", zlib.compress(bytes(rows), 6))
    png += chunk(b"IEND", b"")
    with open(out, "wb") as f:
        f.write(png)
    print(out)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
