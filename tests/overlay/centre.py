#!/usr/bin/env python3
"""Checks that a hive snapshot shows given colours in its centre cell
(S-018 AC24, AC26). Standard library only.

    centre.py SNAP.png R,G,B [R,G,B ...]

Reads an 8-bit RGBA or RGB PNG (non-interlaced), looks at the 40 x 40 box
around the image centre (the centre hexagon's inscribed square holds it), and
exits 0 only if every colour has at least 20 pixels there within 24 levels
per channel. Prints the counts.
"""
import struct
import sys
import zlib

BOX = 40
TOL = 24
MIN = 20


def read_png(path):
    data = open(path, "rb").read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos, idat, w = 8, b"", None
    while pos < len(data):
        (n,) = struct.unpack(">I", data[pos:pos + 4])
        kind, body = data[pos + 4:pos + 8], data[pos + 8:pos + 8 + n]
        pos += 12 + n
        if kind == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
            if depth != 8 or ctype not in (2, 6) or interlace:
                raise ValueError("expected 8-bit RGB or RGBA, not interlaced")
            bpp = 4 if ctype == 6 else 3
        elif kind == b"IDAT":
            idat += body
    raw, stride = zlib.decompress(idat), w * bpp
    rows, prev, i = [], bytearray(stride), 0
    for _ in range(h):
        f, line = raw[i], bytearray(raw[i + 1:i + 1 + stride])
        i += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + b) & 255
            elif f == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(line)
        prev = line
    return w, h, bpp, rows


def main(argv):
    if len(argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    w, h, bpp, rows = read_png(argv[1])
    want = [tuple(int(v) for v in c.split(",")) for c in argv[2:]]
    counts = [0] * len(want)
    for y in range(h // 2 - BOX // 2, h // 2 + BOX // 2):
        for x in range(w // 2 - BOX // 2, w // 2 + BOX // 2):
            px = rows[y][x * bpp:x * bpp + 3]
            for k, c in enumerate(want):
                if all(abs(px[j] - c[j]) <= TOL for j in range(3)):
                    counts[k] += 1
    ok = all(n >= MIN for n in counts)
    print("centre colours:", ", ".join(f"{c} {n}" for c, n in zip(argv[2:], counts)),
          "ok" if ok else f"(want >= {MIN} each)")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
