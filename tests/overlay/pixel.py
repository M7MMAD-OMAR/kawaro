#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Which surface-3 a screenshot written by shot.py shows at a point (S-018
AC15). Reads the 3 x 3 patch centred on X Y and compares every pixel with
color.dark.surface-3 and color.light.surface-3 from design/tokens.json.
Prints "dark", "light" or "other", then the patch's centre pixel as #RRGGBB.
A pixel matches when no channel differs by more than TOLERANCE levels (the
two colours differ by about 170 in each channel). Standard library only.
Usage: pixel.py SHOT.png X Y TOKENS.json [TOLERANCE]"""
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
        rows.append(row[1:])
    return w, h, rows


def rgb(hexstr):
    s = hexstr.lstrip("#")
    return tuple(int(s[i : i + 2], 16) for i in (0, 2, 4))


def main():
    shot, x, y, tokens = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    tol = int(sys.argv[5]) if len(sys.argv) > 5 else 3
    colours = json.load(open(tokens))["color"]
    want = {"dark": rgb(colours["dark"]["surface-3"]), "light": rgb(colours["light"]["surface-3"])}
    _w, _h, rows = load(shot)
    patch = [tuple(rows[y + dy][(x + dx) * 4 : (x + dx) * 4 + 3]) for dy in (-1, 0, 1) for dx in (-1, 0, 1)]
    verdict = "other"
    for name, c in want.items():
        if all(all(abs(p[i] - c[i]) <= tol for i in range(3)) for p in patch):
            verdict = name
    print(verdict, "#%02X%02X%02X" % patch[4])


main()
