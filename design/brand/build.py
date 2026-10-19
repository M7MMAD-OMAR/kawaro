#!/usr/bin/env python3
"""Draw the SbarHive mark from geometry and export every brand asset.

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


def svg(body, view="0 0 512 512", w=512, h=512, extra=""):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" '
            f'viewBox="{view}"{extra}>\n{body}\n</svg>\n')


def app_icon():
    body = (f'  <title>SbarHive</title>\n'
            f'  <path fill="{LIGHT["accent"]}" d="{HEX}"/>\n'
            + "".join(f'  <path fill="{LIGHT["accent-ink"]}" d="{p}"/>\n' for p in cactus()))
    return svg(body.rstrip("\n"))


def run(*args):
    subprocess.run(args, check=True, capture_output=True)


def merged_cutout(fill, out_path, w, h, extra=""):
    """Hexagon minus cactus as one path, via inkscape boolean ops."""
    with tempfile.TemporaryDirectory() as tmp:
        src = Path(tmp) / "src.svg"
        cact = "".join(f'<path d="{p}"/>' for p in cactus())
        src.write_text(svg(f'<path id="hex" d="{HEX}"/><g id="cactus">{cact}</g>'), encoding="utf-8")
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
    merged_cutout("currentColor", HERE / "sbarhive-symbolic.svg", 16, 16,
                  extra=' class="ColorScheme-Text" style="fill:currentColor"')
    p = HERE / "sbarhive-symbolic.svg"
    t = p.read_text(encoding="utf-8")
    t = t.replace(">\n", ">\n" + style, 1)
    p.write_text(t, encoding="utf-8")


def wordmark(name, scheme):
    """Mark, then SbarHive and صبار هايف on one line. Text becomes paths."""
    mark = (f'<g transform="translate(0,8) scale(0.25)">'
            f'<path fill="{scheme["accent"]}" d="{HEX}"/>'
            + "".join(f'<path fill="{scheme["accent-ink"]}" d="{p}"/>' for p in cactus())
            + "</g>")
    font = 'font-family="IBM Plex Sans Arabic" font-weight="600"'
    text = (f'<text x="156" y="104" font-size="84" {font} fill="{scheme["text-primary"]}">SbarHive</text>'
            f'<text x="904" y="104" font-size="84" {font} fill="{scheme["text-secondary"]}" '
            f'direction="rtl" text-anchor="start" xml:lang="ar">صبار هايف</text>')
    with tempfile.TemporaryDirectory() as tmp:
        src = Path(tmp) / "w.svg"
        src.write_text(svg(mark + text, view="0 0 928 144", w=928, h=144), encoding="utf-8")
        out = HERE / name
        run("inkscape", str(src), "--export-text-to-path", "--export-plain-svg",
            f"--export-filename={out}")


def pngs():
    out = HERE / "png"
    out.mkdir(exist_ok=True)
    for s in SIZES:
        run("inkscape", str(HERE / "sbarhive.svg"), "-w", str(s), "-h", str(s),
            f"--export-filename={out / f'sbarhive-{s}.png'}")


def site_icons():
    site = ROOT / "site"
    (site / "favicon.svg").write_text(app_icon(), encoding="utf-8")
    for name, size in (("favicon-32.png", 32), ("apple-touch-icon.png", 180)):
        run("inkscape", str(HERE / "sbarhive.svg"), "-w", str(size), "-h", str(size),
            f"--export-filename={site / name}")


def main():
    (HERE / "sbarhive.svg").write_text(app_icon(), encoding="utf-8")
    symbolic()
    wordmark("wordmark.svg", LIGHT)
    wordmark("wordmark-dark.svg", DARK)
    pngs()
    site_icons()
    print("brand assets written to", HERE.relative_to(ROOT))


if __name__ == "__main__":
    main()
