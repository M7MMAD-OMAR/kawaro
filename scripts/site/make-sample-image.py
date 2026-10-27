#!/usr/bin/env python3
"""Generate the one sample image the download page and the app share (S-088).

Writes tests/fixtures/images/kawaro-sample.png (the fixture S-087 embeds) and
the identical bytes to site/assets/kawaro-sample.png (what the "Try it on an
image" button downloads). One sample for both, so the app and the page always
show the same file; a test asserts the two files are identical.

Pure standard library, deterministic: a flat scene of honeycomb cells and the
cactus built from cells (design/brand/build.py), coloured from the site block
of design/tokens.json, the same source as the site. No third party image, so
the sample is licence clean (GPLv3 like the rest of the repository).

Run: python3 scripts/site/make-sample-image.py
"""
import importlib.util
import json
import math
import struct
import sys
import zlib
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
TOKENS = json.loads((REPO / "design" / "tokens.json").read_text(encoding="utf-8"))["site"]["color"]
W, H = 1280, 800

FIXTURE = REPO / "tests" / "fixtures" / "images" / "kawaro-sample.png"
SITE_ASSET = REPO / "site" / "assets" / "kawaro-sample.png"


def rgb(name):
    h = TOKENS[name].lstrip("#")
    return bytes(int(h[i:i + 2], 16) for i in (0, 2, 4))


def brand_layout():
    """cells_layout from design/brand/build.py, the same lattice as the logo."""
    spec = importlib.util.spec_from_file_location("kawaro_brand", REPO / "design" / "brand" / "build.py")
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load design/brand/build.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.cells_layout


def fill_hex(rows, cx, cy, r, colour):
    """Fill one pointy-top hexagon with a scanline pass (flat, no antialias)."""
    pts = [(cx + r * math.cos(math.radians(a)), cy + r * math.sin(math.radians(a)))
           for a in (-90, -30, 30, 90, 150, 210)]
    lo = max(0, int(math.floor(min(y for _, y in pts))))
    hi = min(H - 1, int(math.ceil(max(y for _, y in pts))))
    for y in range(lo, hi + 1):
        yc = y + 0.5
        xs = []
        for i in range(6):
            x1, y1 = pts[i]
            x2, y2 = pts[(i + 1) % 6]
            if (y1 <= yc < y2) or (y2 <= yc < y1):
                xs.append(x1 + (yc - y1) * (x2 - x1) / (y2 - y1))
        xs.sort()
        for i in range(0, len(xs) - 1, 2):
            a = max(0, int(math.ceil(xs[i] - 0.5)))
            b = min(W - 1, int(math.floor(xs[i + 1] - 0.5)))
            if b >= a:
                rows[y][a * 3:(b + 1) * 3] = colour * (b - a + 1)


def draw():
    rows = [bytearray(rgb("surface-0") * W) for _ in range(H)]
    # A ground band the cactus stands on.
    ground = rgb("surface-2")
    for y in range(int(H * 0.82), H):
        rows[y][:] = ground * W
    # A honeycomb field of flat cells over the paper.
    field = rgb("surface-1")
    r = 54
    pitch_x, pitch_y = r * math.sqrt(3), r * 1.5
    row = 0
    y = -r
    while y < H + r:
        x = -r + (pitch_x / 2 if row % 2 else 0)
        while x < W + r:
            fill_hex(rows, x, y, r - 2, field)
            x += pitch_x
        y += pitch_y
        row += 1
    # The cactus built from many hive cells (the brand mark), scaled to the scene.
    layout = brand_layout()
    cells, cw, ch = layout(100.0)
    scale = (H * 0.62) / ch
    r = 100.0 * scale
    ox = (W - cw * scale) / 2
    oy = H * 0.82 - ch * scale
    solid, ghost = rgb("accent"), rgb("surface-3")
    for x, y, kind, _ in cells:
        fill_hex(rows, ox + x * scale, oy + y * scale, r, ghost if kind == "ghost" else solid)
    return rows


def write_png(path, rows):
    raw = b"".join(b"\x00" + bytes(row) for row in rows)

    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    png = (b"\x89PNG\r\n\x1a\n"
           + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(raw, 9))
           + chunk(b"IEND", b""))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(png)


def main():
    rows = draw()
    write_png(FIXTURE, rows)
    write_png(SITE_ASSET, rows)
    print(f"{FIXTURE.relative_to(REPO)} and {SITE_ASSET.relative_to(REPO)}: {FIXTURE.stat().st_size} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
