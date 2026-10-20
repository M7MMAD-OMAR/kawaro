#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Bounding box of the pixels that differ between two screenshots written by
shot.py (8 bit RGBA, filter 0 on every row). Standard library only.
Prints "X Y W H" of the box, or "none" when nothing differs by more than
TOLERANCE levels in any channel.
Usage: boxdiff.py BEFORE.png AFTER.png [TOLERANCE]"""
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


def main():
    a, b = sys.argv[1], sys.argv[2]
    tol = int(sys.argv[3]) if len(sys.argv) > 3 else 8
    wa, ha, ra = load(a)
    wb, hb, rb = load(b)
    if (wa, ha) != (wb, hb):
        raise SystemExit(f"sizes differ: {wa}x{ha} and {wb}x{hb}")
    x0, y0, x1, y1 = wa, ha, -1, -1
    for y in range(ha):
        p, q = ra[y], rb[y]
        if p == q:
            continue
        for x in range(wa):
            i = x * 4
            if any(abs(p[i + c] - q[i + c]) > tol for c in range(3)):
                x0, x1 = min(x0, x), max(x1, x)
                y0, y1 = min(y0, y), max(y1, y)
    if x1 < 0:
        print("none")
    else:
        print(x0, y0, x1 - x0 + 1, y1 - y0 + 1)


main()
