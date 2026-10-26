#!/usr/bin/env python3
"""Draw the Kawaro mark from geometry and export every brand asset.

The hexagon is the same pointy-top cell the app draws (width:height = sqrt3:2).
The cactus is cut out of it as negative space. Colours come from
design/tokens.json; nothing here invents a colour.

Needs: python3, inkscape (text to path and PNG export).
Run from anywhere: python3 design/brand/build.py
"""

import json
import math
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
TOKENS = json.loads((ROOT / "design" / "tokens.json").read_text(encoding="utf-8"))
LIGHT = TOKENS["color"]["light"]
DARK = TOKENS["color"]["dark"]
SQRT3 = math.sqrt(3)
SIZES = [16, 32, 48, 64, 128, 256, 512]


def hexagon(cx, cy, r, corner):
    """Pointy-top hexagon path with rounded corners of radius `corner`."""
    pts = [(cx + r * math.cos(math.radians(a)), cy + r * math.sin(math.radians(a)))
           for a in (-90, -30, 30, 90, 150, 210)]
    d = []
    n = len(pts)
    for i in range(n):
        p0, p1, p2 = pts[i - 1], pts[i], pts[(i + 1) % n]
        # Pull back from the vertex along both edges, then curve through it.
        def toward(a, b, dist):
            length = math.dist(a, b)
            return (a[0] + (b[0] - a[0]) * dist / length, a[1] + (b[1] - a[1]) * dist / length)
        a = toward(p1, p0, corner)
        b = toward(p1, p2, corner)
        d.append(("M" if i == 0 else "L") + f"{a[0]:.2f},{a[1]:.2f}")
        d.append(f"Q{p1[0]:.2f},{p1[1]:.2f} {b[0]:.2f},{b[1]:.2f}")
    d.append("Z")
    return "".join(d)



def rounded_bar(x0, y0, x1, y1, top_round=True):
    """Vertical bar from y0 (top) to y1, fully rounded top when asked."""
    w = x1 - x0
    rr = w / 2
    if top_round:
        return (f"M{x0},{y1}V{y0 + rr}A{rr},{rr} 0 0 1 {x1},{y0 + rr}V{y1}Z")
    return f"M{x0},{y0}H{x1}V{y1}H{x0}Z"


def cactus():
    """Saguaro on a 512 grid as a union of simple shapes (no overlaps needed
    by renderers: the shapes are later merged by inkscape for the symbolic icon).

    Trunk 80 wide, arms 56 wide, gaps between arm and trunk 40 so they stay
    apart at 32 px and blur into one readable silhouette at 16 px.
    """
    parts = []
    # trunk: x 216..296, from y 96 to y 424
    parts.append(rounded_bar(216, 96, 296, 424))
    # left arm: upright x 120..176 from y 176 to elbow, elbow joins trunk
    parts.append(rounded_bar(120, 176, 176, 300))
    parts.append(f"M120,296V300A56,56 0 0 0 176,356H216V300H176V296Z")
    # right arm: upright x 336..392 from y 136 to elbow at y 300
    parts.append(rounded_bar(336, 136, 392, 260))
    parts.append(f"M392,256V260A56,56 0 0 1 336,316H296V260H336V256Z")
    return parts

HEX = hexagon(256, 256, 248, 28)


# The full logo: the cactus drawn as many hive cells on the honeycomb lattice
# (axial offset coordinates: column, row; odd rows sit half a cell right).
# Solid cells are the plant, ghost cells are empty cells in the plant's place,
# floats are loose cells beside it.
def _solid():
    c = [(5, 0)]
    for r in range(1, 11):
        c += [(4, r), (5, r)]
    c += [(2, 6), (3, 6), (1, 3), (2, 3), (1, 4), (2, 4), (1, 5), (2, 5), (2, 2)]
    c += [(6, 4), (7, 4), (7, 1), (8, 1), (7, 2), (8, 2), (7, 3), (8, 3), (8, 0)]
    return c


CELLS_GHOST = [(1, 4), (8, 2), (5, 9)]
CELLS_FLOATS = [(9.0, 6.0), (0.2, 8.8), (9.9, 3.2)]
CELLS_SOLID = [c for c in _solid() if c not in CELLS_GHOST]


def cell_points(cx, cy, r):
    return " ".join(f"{cx + r * math.cos(math.radians(a)):.2f},{cy + r * math.sin(math.radians(a)):.2f}"
                    for a in (-90, -30, 30, 90, 150, 210))


def cells_layout(r, extras=True):
    """Cell centres of the cactus for cell radius r, as (x, y, kind, row) with
    the origin at the top left of the bounding box."""
    s = r * 1.14
    pos = lambda c, row: (s * SQRT3 * (c + (0.5 if round(row) % 2 else 0) - 4.75), s * 1.5 * (row - 5))
    out = [(*pos(c, row), "solid", row) for c, row in CELLS_SOLID]
    if extras:
        out += [(*pos(c, row), "ghost", row) for c, row in CELLS_GHOST]
        out += [(*pos(c, row), "float", row) for c, row in CELLS_FLOATS]
    x0 = min(x for x, *_ in out) - r * SQRT3 / 2
    y0 = min(y for _, y, *_ in out) - r
    out = [(x - x0, y - y0, k, row) for x, y, k, row in out]
    w = max(x for x, *_ in out) + r * SQRT3 / 2
    h = max(y for _, y, *_ in out) + r
    return out, w, h


def cells_mark(scheme, r=20):
    cells, w, h = cells_layout(r)
    body = []
    for x, y, kind, _ in cells:
        pts = cell_points(x, y, r)
        if kind == "ghost":
            body.append(f'  <polygon fill="none" stroke="{scheme["border"]}" stroke-width="2" '
                        f'stroke-linejoin="round" points="{pts}"/>')
        else:
            body.append(f'  <polygon fill="{scheme["accent"]}" stroke="{scheme["accent"]}" stroke-width="2" '
                        f'stroke-linejoin="round" points="{pts}"/>')
    return svg("  <title>Kawaro</title>\n" + "\n".join(body), view=f"0 0 {w:.0f} {h:.0f}", w=round(w), h=round(h))


def svg(body, view="0 0 512 512", w=512, h=512, extra=""):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" '
            f'viewBox="{view}"{extra}>\n{body}\n</svg>\n')


def app_icon():
    body = (f'  <title>Kawaro</title>\n'
            f'  <path fill="{LIGHT["accent"]}" d="{HEX}"/>\n'
            + "".join(f'  <path fill="{LIGHT["accent-ink"]}" d="{p}"/>\n' for p in cactus()))
    return svg(body.rstrip("\n"))


def run(*args):
    subprocess.run(args, check=True, capture_output=True)


def merged_cutout(fill, out_path, w, h, extra=""):
    """Hexagon minus cactus as one path, via inkscape boolean ops."""
    with tempfile.TemporaryDirectory() as tmp:
        src = Path(tmp) / "src.svg"
        tiers = "".join(f'<path d="{p}"/>' for p in cactus())
        src.write_text(svg(f'<path id="hex" d="{HEX}"/><g id="cactus">{tiers}</g>'), encoding="utf-8")
        merged = Path(tmp) / "merged.svg"
        run("inkscape", str(src), "--actions",
            "select-by-id:cactus;selection-ungroup;path-union;"
            "select-all;path-difference;"
            f"export-plain-svg;export-filename:{merged};export-do")
        text = merged.read_text(encoding="utf-8")
    import re
    d = re.search(r'<path[^>]*\sd="([^"]+)"', text).group(1)
    body = (f'  <path fill="{fill}" d="{d}"{extra}/>')
    out_path.write_text(svg(body, w=w, h=h), encoding="utf-8")


def symbolic():
    # GTK convention: #bebebe in a -symbolic file is recoloured to the panel's
    # foreground. KDE reads the ColorScheme-Text class instead.
    style = ('  <style id="current-color-scheme" type="text/css">'
             f'.ColorScheme-Text {{ color: #bebebe; }}</style>\n')
    merged_cutout("currentColor", HERE / "kawaro-symbolic.svg", 16, 16,
                  extra=' class="ColorScheme-Text" style="fill:currentColor"')
    p = HERE / "kawaro-symbolic.svg"
    t = p.read_text(encoding="utf-8")
    t = t.replace(">\n", ">\n" + style, 1)
    p.write_text(t, encoding="utf-8")


def wordmark(name, scheme):
    """Mark, then Kawaro and كوارو on one line. Text becomes paths."""
    cells, w, h = cells_layout(20)
    k = 128 / h
    polys = "".join(
        (f'<polygon fill="none" stroke="{scheme["border"]}" stroke-width="2" stroke-linejoin="round" points="{cell_points(x, y, 20)}"/>'
         if kind == "ghost" else
         f'<polygon fill="{scheme["accent"]}" stroke="{scheme["accent"]}" stroke-width="2" stroke-linejoin="round" points="{cell_points(x, y, 20)}"/>')
        for x, y, kind, _ in cells)
    mark = f'<g transform="translate(0,8) scale({k:.4f})">{polys}</g>'
    font = 'font-family="IBM Plex Sans Arabic" font-weight="600"'
    text = (f'<text x="156" y="104" font-size="84" {font} fill="{scheme["text-primary"]}">Kawaro</text>'
            f'<text x="662" y="104" font-size="84" {font} fill="{scheme["text-secondary"]}" '
            f'direction="rtl" text-anchor="start" xml:lang="ar">كوارو</text>')
    with tempfile.TemporaryDirectory() as tmp:
        src = Path(tmp) / "w.svg"
        src.write_text(svg(mark + text, view="0 0 686 144", w=686, h=144), encoding="utf-8")
        out = HERE / name
        run("inkscape", str(src), "--export-text-to-path", "--export-plain-svg",
            f"--export-filename={out}")


def pngs():
    out = HERE / "png"
    out.mkdir(exist_ok=True)
    for s in SIZES:
        run("inkscape", str(HERE / "kawaro.svg"), "-w", str(s), "-h", str(s),
            f"--export-filename={out / f'kawaro-{s}.png'}")


def site_icons():
    site = ROOT / "site"
    (site / "favicon.svg").write_text(app_icon(), encoding="utf-8")
    (site / "kawaro-cells.svg").write_text(cells_mark(LIGHT), encoding="utf-8")
    for name, size in (("favicon-32.png", 32), ("apple-touch-icon.png", 180)):
        run("inkscape", str(HERE / "kawaro.svg"), "-w", str(size), "-h", str(size),
            f"--export-filename={site / name}")


def main():
    (HERE / "kawaro.svg").write_text(app_icon(), encoding="utf-8")
    (HERE / "kawaro-cells.svg").write_text(cells_mark(LIGHT), encoding="utf-8")
    (HERE / "kawaro-cells-dark.svg").write_text(cells_mark(DARK), encoding="utf-8")
    symbolic()
    wordmark("wordmark.svg", LIGHT)
    wordmark("wordmark-dark.svg", DARK)
    pngs()
    site_icons()
    print("brand assets written to", HERE.relative_to(ROOT))


if __name__ == "__main__":
    main()
