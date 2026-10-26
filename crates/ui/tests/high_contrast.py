#!/usr/bin/env python3
"""S-062 rendered checks for one hive snapshot (AC6, AC7, AC9, AC10).

    high_contrast.py SNAP.png LANG SCHEME TOKENS.json CENTRES.txt FONT.otf

SNAP.png is a hive_snapshot render (8-bit RGBA, highlight 0, the sample image
actions). Exit 0 only if every check holds; every failure prints one line.
Standard library only, except PIL for the AC10 width (skipped, with a note,
when PIL is missing). The PNG decoder is the one of tests/overlay/centre.py.
"""
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location(
    "centre", os.path.join(HERE, "..", "..", "..", "tests", "overlay", "centre.py"))
centre = importlib.util.module_from_spec(spec)
spec.loader.exec_module(centre)

EN_CODES = ["WEBP", "PNG", "JPG", "RESIZE", "ROTATE", "GRAY", "PDF", "STRIP"]
TOL = 12


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def near(px, c, tol=TOL):
    return all(abs(px[i] - c[i]) <= tol for i in range(3))


def components(img, colour, min_size=300):
    """4-connected components of exactly `colour`: (size, x0, y0, x1, y1)."""
    w, h, bpp, rows = img
    seen = [bytearray(w) for _ in range(h)]
    found = []
    for y in range(h):
        for x in range(w):
            if seen[y][x] or not near(rows[y][x * bpp:x * bpp + 3], colour, 0):
                continue
            if bpp == 4 and rows[y][x * bpp + 3] != 255:
                continue
            stack, n = [(x, y)], 0
            seen[y][x] = 1
            x0 = x1 = x
            y0 = y1 = y
            while stack:
                cx, cy = stack.pop()
                n += 1
                x0, x1, y0, y1 = min(x0, cx), max(x1, cx), min(y0, cy), max(y1, cy)
                for nx, ny in ((cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)):
                    if 0 <= nx < w and 0 <= ny < h and not seen[ny][nx]:
                        p = rows[ny][nx * bpp:nx * bpp + 3]
                        if near(p, colour, 0) and (bpp == 3 or rows[ny][nx * bpp + 3] == 255):
                            seen[ny][nx] = 1
                            stack.append((nx, ny))
            if n >= min_size:
                found.append((n, x0, y0, x1, y1))
    return found


def main(argv):
    if len(argv) != 7:
        print(__doc__, file=sys.stderr)
        return 2
    snap, lang, scheme, tokens_path, centres_path, font_path = argv[1:]
    tokens = json.load(open(tokens_path))
    col = tokens["color"][scheme]
    fails = []
    need = ["hive-plate", "hive-edge", "hex-fill", "hex-fill-centre", "hex-ink", "accent", "accent-ink"]
    missing = [k for k in need if k not in col]
    if missing:
        print(f"FAIL {snap}: tokens missing in color.{scheme}: {', '.join(missing)}")
        return 1
    plate, edge = rgb(col["hive-plate"]), rgb(col["hive-edge"])
    fill, fill_c = rgb(col["hex-fill"]), rgb(col["hex-fill-centre"])
    ink, acc, acc_ink = rgb(col["hex-ink"]), rgb(col["accent"]), rgb(col["accent-ink"])
    border = tokens["border-width"].get("hex", 2)
    space2 = tokens["space"]["2"]

    img = centre.read_png(snap)
    w, h, bpp, rows = img
    if bpp != 4:
        print(f"FAIL {snap}: expected RGBA")
        return 1

    def px(x, y):
        return tuple(rows[y][x * bpp:x * bpp + 4])

    # AC9: cell centres (doubled pixel coordinates) are the committed ones.
    want = []
    for line in open(centres_path):
        parts = line.split()
        if parts and parts[0] == lang:
            want.append((int(parts[1]), int(parts[2])))
    want.sort()
    cells = []  # (colour, size, x0, y0, x1, y1)
    for c in (fill, fill_c, acc):
        for comp in components(img, c):
            cells.append((c,) + comp)
    got = sorted((x0 + x1, y0 + y1) for (_, _, x0, y0, x1, y1) in cells)
    if got != want:
        fails.append(f"AC9 cell centres {got} != committed {want}")

    # AC6 (S-074 D3, follows AC1): no plate. Every pixel farther than a
    # cell's circumradius + 4 px from every cell centre is the compositor
    # background of the harness (fully transparent), not the plate colour.
    pts = [(x // 2, y // 2) for x, y in got]
    rad = max((c[5] - c[3]) / 2 for c in cells) + 4 if cells else 0
    outside = [(x, y) for y in range(0, h, 3) for x in range(0, w, 3)
               if all((x - a[0]) ** 2 + (y - a[1]) ** 2 > rad ** 2 for a in pts)]
    bad = [(p, px(*p)) for p in outside if px(*p) != (0, 0, 0, 0)]
    if len(outside) < 20:
        fails.append(f"AC6 only {len(outside)} points outside the cells, want 20 or more")
    if bad:
        fails.append(f"AC6 {len(bad)} of {len(outside)} points outside every cell are not transparent background: {bad[:4]}")
    if any(px(*p)[:3] == plate and px(*p)[3] == 255 for p in outside):
        fails.append(f"AC6 plate colour {plate} found outside the cells")

    # AC7: the highlighted cell is the accent component; its edge is
    # `border` px of hive-edge, outside the accent on the mid row.
    hi = [c for c in cells if c[0] == acc]
    if len(hi) != 1:
        fails.append(f"AC7 {len(hi)} accent cells, want 1")
    else:
        _, _, x0, y0, x1, y1 = hi[0]
        y = (y0 + y1) // 2
        x = x0 - 1
        run = 0
        while x >= 0 and near(px(x, y), edge):
            run += 1
            x -= 1
        if run != border:
            fails.append(f"AC7 highlighted edge band is {run} px of hive-edge {edge} on row {y}, want {border}")

    # AC10: the code text of every ring cell is complete: inside the cell and
    # as wide as its font says (not elided).
    try:
        from PIL import ImageFont
        font = ImageFont.truetype(font_path, tokens["font"]["size"].get("cell", 14))
    except Exception as e:  # PIL or the font is missing
        font = None
        print(f"note: AC10 width comparison skipped ({e})")
    widths = []
    for (c, _, x0, y0, x1, y1) in cells:
        if c == fill_c:
            continue
        cell_ink = acc_ink if c == acc else ink
        top, bottom = y0 + (y1 - y0) // 4, y1 - (y1 - y0) // 4
        dist = sum(abs(cell_ink[i] - c[i]) for i in range(3))
        rows_hit = []
        for yy in range(top, bottom + 1):
            xs = [xx for xx in range(x0, x1 + 1)
                  if sum(abs(px(xx, yy)[i] - c[i]) for i in range(3)) * 2 > dist]
            rows_hit.append(xs)
        groups, cur = [], []
        for xs in rows_hit:
            if xs:
                cur.append(xs)
            elif cur:
                groups.append(cur)
                cur = []
        if cur:
            groups.append(cur)
        if not groups:
            fails.append(f"AC10 no text in the cell at {(x0 + x1) // 2},{(y0 + y1) // 2}")
            continue
        last = groups[-1]
        tw = max(max(g) for g in last) - min(min(g) for g in last) + 1
        inner = (x1 - x0 + 1) + 2 * border - 2 * space2
        widths.append(tw)
        if tw > inner:
            fails.append(f"AC10 text {tw} px wide in a cell with {inner} px inner width")
    if font is not None and lang == "en":
        want_w = sorted(round(font.getlength(t)) - 2 for t in EN_CODES)
        if len(widths) != len(want_w) or any(abs(a - b) > 2 for a, b in zip(sorted(widths), want_w)):
            fails.append(f"AC10 rendered code widths {sorted(widths)} vs font widths {want_w} (an elided label is narrower)")

    for f in fails:
        print(f"FAIL {os.path.basename(snap)}: {f}")
    if not fails:
        print(f"ok {os.path.basename(snap)}: AC6 AC7 AC9 AC10")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
