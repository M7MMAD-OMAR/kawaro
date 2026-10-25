#!/usr/bin/env python3
"""Draw the Kawaro mark from geometry and export every brand asset.

The hexagon is the same pointy-top cell the app draws (width:height = sqrt3:2).
The beehive (kawwara) is cut out of it as negative space. Colours come from
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



def skep():
    """The traditional beehive (kawwara) on a 512 grid: a dome of radius 140
    centred at (256, 262), cut by three 14 unit gaps into three dome tiers and
    a wider base with an arched entrance. The upper half follows the circle,
    the lower part has straight sides. The tiers stay separate at 32 px and
    read as one striped dome with a door at 16 px.
    """
    cx, cy, r = 256, 262, 140

    def half(y):
        return math.sqrt(r * r - (y - cy) ** 2) if y < cy else r

    parts = []
    for y1, y2 in ((122, 174), (188, 236), (250, 294)):
        a = half(y1)
        top = f"M{cx - a:.2f},{y1}H{cx + a:.2f}"
        if y2 <= cy:
            b = half(y2)
            right = f"A{r},{r} 0 0 1 {cx + b:.2f},{y2}"
            bottom = f"H{cx - b:.2f}"
            left = f"A{r},{r} 0 0 1 {cx - a:.2f},{y1}"
        else:
            right = f"A{r},{r} 0 0 1 {cx + r},{cy}V{y2}"
            bottom = f"H{cx - r}"
            left = f"V{cy}A{r},{r} 0 0 1 {cx - a:.2f},{y1}"
        parts.append(top + right + bottom + left + "Z")
    # base: wider than the dome, with an arched entrance cut into it
    base_half, door, door_h = 154, 26, 56
    y1, y2 = 308, 388
    arch = y2 - door_h + door
    parts.append(f"M{cx - base_half},{y1}H{cx + base_half}V{y2}H{cx + door}V{arch}"
                 f"A{door},{door} 0 0 0 {cx - door},{arch}V{y2}H{cx - base_half}Z")
    return parts


HEX = hexagon(256, 256, 248, 28)


def svg(body, view="0 0 512 512", w=512, h=512, extra=""):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" '
            f'viewBox="{view}"{extra}>\n{body}\n</svg>\n')


def app_icon():
    body = (f'  <title>Kawaro</title>\n'
            f'  <path fill="{LIGHT["accent"]}" d="{HEX}"/>\n'
            + "".join(f'  <path fill="{LIGHT["accent-ink"]}" d="{p}"/>\n' for p in skep()))
    return svg(body.rstrip("\n"))


def run(*args):
    subprocess.run(args, check=True, capture_output=True)


def merged_cutout(fill, out_path, w, h, extra=""):
    """Hexagon minus beehive as one path, via inkscape boolean ops."""
    with tempfile.TemporaryDirectory() as tmp:
        src = Path(tmp) / "src.svg"
        tiers = "".join(f'<path d="{p}"/>' for p in skep())
        src.write_text(svg(f'<path id="hex" d="{HEX}"/><g id="skep">{tiers}</g>'), encoding="utf-8")
        merged = Path(tmp) / "merged.svg"
        run("inkscape", str(src), "--actions",
            "select-by-id:skep;selection-ungroup;path-union;"
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
    mark = (f'<g transform="translate(0,8) scale(0.25)">'
            f'<path fill="{scheme["accent"]}" d="{HEX}"/>'
            + "".join(f'<path fill="{scheme["accent-ink"]}" d="{p}"/>' for p in skep())
            + "</g>")
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
    for name, size in (("favicon-32.png", 32), ("apple-touch-icon.png", 180)):
        run("inkscape", str(HERE / "kawaro.svg"), "-w", str(size), "-h", str(size),
            f"--export-filename={site / name}")


def main():
    (HERE / "kawaro.svg").write_text(app_icon(), encoding="utf-8")
    symbolic()
    wordmark("wordmark.svg", LIGHT)
    wordmark("wordmark-dark.svg", DARK)
    pngs()
    site_icons()
    print("brand assets written to", HERE.relative_to(ROOT))


if __name__ == "__main__":
    main()
