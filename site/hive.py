#!/usr/bin/env python3
"""Draw the hero hive into both landing pages.

Same geometry as spikes/hive/src/geometry.rs: pointy-top cells in axial
coordinates, spacing r + gap / sqrt3, centre first then each ring in spiral
order, x mirrored for right-to-left. Colours are CSS classes that read
site/tokens.css, so the markup holds no raw colour.

The markup goes between <!-- hive:start --> and <!-- hive:end --> in each page.
Run: python3 site/hive.py
"""

import json
import math
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOKENS = json.loads((HERE.parent / "design" / "tokens.json").read_text(encoding="utf-8"))
SQRT3 = math.sqrt(3)
R = TOKENS["hex"]["size"]["md"] / 2
GAP = TOKENS["hex"]["gap"]
PAD = 4

DIRECTIONS = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)]

# What the hive offers for one selected image in v0.1 (README, "What it does").
# (code, arabic label, english label); None marks a cell the selection leaves empty.
CELLS = [
    ("WEBP", "WEBP", "WEBP"), ("PNG", "PNG", "PNG"), ("JPG", "JPG", "JPG"),
    ("AVIF", "AVIF", "AVIF"), ("PDF", "PDF", "PDF"), ("GIF", "GIF", "GIF"),
    ("BMP", "BMP", "BMP"), ("TIFF", "TIFF", "TIFF"), ("ICO", "ICO", "ICO"),
    ("resize", "تحجيم", "RESIZE"), ("rotate", "تدوير", "ROTATE"), ("flip", "قلب", "FLIP"),
    ("gray", "رمادي", "GRAY"), ("compress", "ضغط", "SHRINK"), ("strip", "تنظيف", "STRIP"),
    None, None, None,
]
ACTIVE = 0  # index into CELLS of the highlighted cell

PAGES = {
    "ar": {
        "path": HERE / "index.html",
        "rtl": True,
        "center": "إلى WebP",
        "title": "الخلية عند تحديد صورة واحدة: صيغ التحويل في الحلقة الأولى، وإجراءات الصورة في الحلقة الثانية",
    },
    "en": {
        "path": HERE / "en" / "index.html",
        "rtl": False,
        "center": "To WebP",
        "title": "The hive for one image: target formats in the first ring, image actions in the second",
    },
}


def spiral(rings):
    out = [(0, 0)]
    for k in range(1, rings + 1):
        q, r = DIRECTIONS[4][0] * k, DIRECTIONS[4][1] * k
        for dq, dr in DIRECTIONS:
            for _ in range(k):
                out.append((q, r))
                q, r = q + dq, r + dr
    return out


def hex_path(cx, cy):
    pts = [(cx + R * math.cos(math.radians(a)), cy + R * math.sin(math.radians(a)))
           for a in (-90, -30, 30, 90, 150, 210)]
    return "M" + "L".join(f"{x:.1f},{y:.1f}" for x, y in pts) + "Z"


def build(lang):
    page = PAGES[lang]
    s = R + GAP / SQRT3
    half_w = s * SQRT3 * 2 + R * SQRT3 / 2 + PAD
    half_h = s * 1.5 * 2 + R + PAD
    w, h = 2 * half_w, 2 * half_h
    lines = [
        f'<svg class="hive" viewBox="0 0 {w:.0f} {h:.0f}" role="img" aria-labelledby="hive-title">',
        f'  <title id="hive-title">{page["title"]}</title>',
        '  <g aria-hidden="true">',
    ]
    for i, (q, r) in enumerate(spiral(2)):
        x = s * (SQRT3 * q + SQRT3 / 2 * r)
        if page["rtl"]:
            x = -x
        cx, cy = half_w + x, half_h + s * 1.5 * r
        if i == 0:
            cls, label = "cell cell-center", page["center"]
        else:
            cell = CELLS[i - 1]
            if cell is None:
                cls, label = "cell cell-empty", ""
            else:
                cls = "cell cell-active" if i - 1 == ACTIVE else "cell"
                label = cell[1] if lang == "ar" else cell[2]
        lines.append(f'    <g class="{cls}" style="--i:{i}">')
        lines.append(f'      <path d="{hex_path(cx, cy)}"/>')
        if label:
            lines.append(f'      <text x="{cx:.1f}" y="{cy:.1f}">{label}</text>')
        lines.append("    </g>")
    lines += ["  </g>", "</svg>"]
    return "\n".join(lines)


def main():
    for lang, page in PAGES.items():
        html = page["path"].read_text(encoding="utf-8")
        new, n = re.subn(r"(<!-- hive:start -->).*?(<!-- hive:end -->)",
                         lambda m: m.group(1) + "\n" + build(lang) + "\n" + m.group(2),
                         html, flags=re.S)
        if n != 1:
            raise SystemExit(f"{page['path']}: hive markers not found")
        page["path"].write_text(new, encoding="utf-8")
        print("hive written to", page["path"].relative_to(HERE.parent))


if __name__ == "__main__":
    main()
