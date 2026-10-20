#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Pixel tools for the drop mode screenshots (S-014c: AC9, AC10). Reads the
8 bit RGBA PNGs written by tests/overlay/shot.py (filter 0 on every row).
Standard library only.

  pixels.py crop SHOT X Y W H OUT
      writes the W x H box at X Y of SHOT to OUT (8 bit RGBA).
  pixels.py count SHOT X Y W H TOKENS SCHEME NAME [TOLERANCE]
      prints how many pixels of the box are within TOLERANCE levels (default
      8) per channel of color.SCHEME.NAME in design/tokens.json.
  pixels.py diff A B [TOLERANCE]
      prints the fraction of pixels that differ by more than TOLERANCE levels
      (default 8) in any channel; exits 1 when the sizes differ.
"""
import json
import struct
import sys
import zlib


def load(path):
    data = open(path, "rb").read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"{path}: not a PNG")
    pos, idat, w, h = 8, b"", 0, 0
    while pos < len(data):
        (n,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + n]
        if kind == b"IHDR":
            w, h, depth, colour = struct.unpack(">IIBB", body[:10])
            if depth != 8 or colour != 6:
                raise SystemExit(f"{path}: expected 8 bit RGBA")
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    raw = zlib.decompress(idat)
    stride = w * 4 + 1
    rows = []
    for y in range(h):
        row = raw[y * stride : (y + 1) * stride]
        if row[0] != 0:
            raise SystemExit(f"{path}: row filter {row[0]} not supported")
        rows.append(bytes(row[1:]))
    return w, h, rows


def box(rows, x, y, w, h):
    return [r[x * 4 : (x + w) * 4] for r in rows[y : y + h]]


def save(path, w, h, rows):
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(b"".join(b"\x00" + r for r in rows), 6)) + chunk(b"IEND", b"")
    open(path, "wb").write(png)


def rgb(hexstr):
    s = hexstr.lstrip("#")
    return tuple(int(s[i : i + 2], 16) for i in (0, 2, 4))


def main():
    cmd, a = sys.argv[1], sys.argv[2:]
    if cmd == "crop":
        _w, _h, rows = load(a[0])
        x, y, w, h = map(int, a[1:5])
        save(a[5], w, h, box(rows, x, y, w, h))
    elif cmd == "count":
        _w, _h, rows = load(a[0])
        x, y, w, h = map(int, a[1:5])
        want = rgb(json.load(open(a[5]))["color"][a[6]][a[7]])
        tol = int(a[8]) if len(a) > 8 else 8
        n = 0
        for r in box(rows, x, y, w, h):
            for i in range(0, len(r), 4):
                if all(abs(r[i + c] - want[c]) <= tol for c in range(3)):
                    n += 1
        print(n)
    elif cmd == "diff":
        wa, ha, ra = load(a[0])
        wb, hb, rb = load(a[1])
        if (wa, ha) != (wb, hb):
            print(f"size {wa}x{ha} against {wb}x{hb}")
            sys.exit(1)
        tol = int(a[2]) if len(a) > 2 else 8
        bad = 0
        for x, y in zip(ra, rb):
            for i in range(0, len(x), 4):
                if any(abs(x[i + c] - y[i + c]) > tol for c in range(4)):
                    bad += 1
        print(f"{bad / (wa * ha):.6f}")
    else:
        raise SystemExit(__doc__)


main()
