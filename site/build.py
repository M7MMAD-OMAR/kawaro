#!/usr/bin/env python3
"""Build the Kawaro site: every page in Arabic (site root) and English (site/en).

Standard library only. Colours, lengths and durations never appear here: the
markup uses classes and custom properties from tokens.css. Geometry numbers for
the hive come from design/tokens.json, the same source as the app.

Run: python3 site/build.py
Pages: index, windows. Arabic and English are two originals
that share a structure, not one text translated from the other.
"""

import html
import json
import math
import os
import re
import subprocess
import sys
import urllib.parse
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOKENS = json.loads((HERE.parent / "design" / "tokens.json").read_text(encoding="utf-8"))
SQRT3 = math.sqrt(3)
R = TOKENS["hex"]["size"]["md"] / 2
GAP = TOKENS["hex"]["gap"]
PAD = 4
BASE = "https://kawaro.sbarah.com/"
REPO = "https://github.com/M7MMAD-OMAR/kawaro"
RELEASES = REPO + "/releases"
VERSION = "0.4.6"
# S-061: download links point at release assets only once the release exists.
# Set RELEASE_PUBLISHED=1 after the tag and its assets are on GitHub.
PUBLISHED = os.environ.get("RELEASE_PUBLISHED", "0") == "1"
DIRECTIONS = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)]

PAGES = ["index", "windows"]

# The tab icon is a same origin file: the CSP has no data: source (S-059).
FAVICON = "/favicon.svg"


def spiral(rings):
    out = [(0, 0)]
    for k in range(1, rings + 1):
        q, r = DIRECTIONS[4][0] * k, DIRECTIONS[4][1] * k
        for dq, dr in DIRECTIONS:
            for _ in range(k):
                out.append((q, r))
                q, r = q + dq, r + dr
    return out


def hex_path(cx, cy, radius=R):
    pts = [(cx + radius * math.cos(math.radians(a)), cy + radius * math.sin(math.radians(a)))
           for a in (-90, -30, 30, 90, 150, 210)]
    return "M" + "L".join(f"{x:.1f},{y:.1f}" for x, y in pts) + "Z"


S = R + GAP / SQRT3
RINGS = 3
HALF_W = S * SQRT3 * RINGS + R * SQRT3 / 2 + PAD
HALF_H = S * 1.5 * RINGS + R + PAD
VIEW = f"0 0 {2 * HALF_W:.0f} {2 * HALF_H:.0f}"


def cell_xy(q, r, rtl):
    x = S * (SQRT3 * q + SQRT3 / 2 * r)
    if rtl:
        x = -x
    return HALF_W + x, HALF_H + S * 1.5 * r


# ---------------------------------------------------------------------------
# Hive demo data. Cells follow actions/*.toml: only what fits the selection.
# Each cell: (label shown in the cell, full name, output file name).
# ---------------------------------------------------------------------------

def kinds(lang):
    ar = lang == "ar"
    img = [
        ("WEBP", "تحويل إلى WebP" if ar else "Convert to WebP", "photo.webp"),
        ("PNG", "تحويل إلى PNG" if ar else "Convert to PNG", "photo.png"),
        ("JPG", "تحويل إلى JPG" if ar else "Convert to JPG", "photo.jpg"),
        ("AVIF", "تحويل إلى AVIF" if ar else "Convert to AVIF", "photo.avif"),
        ("PDF", "تحويل إلى PDF" if ar else "Convert to PDF", "photo.pdf"),
        ("GIF", "تحويل إلى GIF" if ar else "Convert to GIF", "photo.gif"),
        ("تحجيم" if ar else "RESIZE", "تحجيم الصورة" if ar else "Resize image", "photo-resized.png"),
        ("تدوير" if ar else "ROTATE", "تدوير الصورة" if ar else "Rotate", "photo-rotated.png"),
        ("قلب" if ar else "FLIP", "قلب الصورة" if ar else "Flip image", "photo-flipped.png"),
        ("رمادي" if ar else "GRAY", "تحويل الصورة إلى رمادي" if ar else "Gray: make it gray", "photo-gray.png"),
        ("ضغط" if ar else "SHRINK", "ضغط الصورة" if ar else "Shrink image", "photo-compressed.png"),
        ("تنظيف" if ar else "STRIP", "تنظيف: حذف البيانات الوصفية" if ar else "Strip metadata", "photo-clean.png"),
    ]
    vid = [
        ("MP4", "تحويل إلى MP4" if ar else "Convert to MP4", "clip.mp4"),
        ("WEBM", "تحويل إلى WebM" if ar else "Convert to WebM", "clip.webm"),
        ("GIF", "تحويل إلى GIF متحركة" if ar else "Convert to animated GIF", "clip.gif"),
        ("MKV", "تحويل إلى MKV" if ar else "Convert to MKV", "clip.mkv"),
        ("MP3", "استخراج الصوت بصيغة MP3" if ar else "Extract audio as MP3", "clip.mp3"),
        ("OGG", "استخراج الصوت بصيغة OGG" if ar else "Extract audio as OGG", "clip.ogg"),
        ("ضغط" if ar else "SHRINK", "ضغط الفيديو" if ar else "Shrink video", "clip-compressed.mov"),
        ("M4A", "استخراج الصوت بصيغة M4A" if ar else "Extract audio as M4A", "clip.m4a"),
        ("FLAC", "استخراج الصوت بصيغة FLAC" if ar else "Extract audio as FLAC", "clip.flac"),
        ("WAV", "استخراج الصوت بصيغة WAV" if ar else "Extract audio as WAV", "clip.wav"),
    ]
    doc = [
        ("PDF", "تحويل إلى PDF" if ar else "Convert to PDF", "report.pdf"),
        ("ODT", "تحويل إلى ODT" if ar else "Convert to ODT", "report.odt"),
    ]
    aud = [
        ("MP3", "تحويل إلى MP3" if ar else "Convert to MP3", "song.mp3"),
        ("M4A", "تحويل إلى M4A" if ar else "Convert to M4A", "song.m4a"),
        ("OGG", "تحويل إلى OGG" if ar else "Convert to OGG", "song.ogg"),
        ("FLAC", "تحويل إلى FLAC" if ar else "Convert to FLAC", "song.flac"),
    ]
    pdf = [
        ("دمج" if ar else "MERGE", "دمج الملفات في ملف واحد" if ar else "Merge into one PDF", "a-merged.pdf"),
        ("ضغط" if ar else "SHRINK", "ضغط ملفات PDF" if ar else "Shrink PDFs", "a-compressed.pdf"),
        ("تدوير" if ar else "ROTATE", "تدوير الصفحات 90 درجة" if ar else "Rotate pages 90 degrees", "a-rotated.pdf"),
        ("صور" if ar else "PNGS", "صور: تحويل الصفحات إلى صور PNG" if ar else "PNGs: pages to PNG images", "a-pages.zip"),
        ("تقسيم" if ar else "SPLIT", "تقسيم: ملف PDF لكل صفحة" if ar else "Split: one PDF per page", "a-split.zip"),
    ]

    def k(key, label, sel, files, cells):
        exts = [o.rsplit(".", 1)[1] for _, _, o in cells]
        # id: the output extension, or the whole name when two cells share one
        return {"key": key, "label": label, "selection": sel, "files": files,
                "cells": [{"c": c, "n": n, "o": o,
                           "id": ext if exts.count(ext) == 1 else o.replace(".", "-")}
                          for (c, n, o), ext in zip(cells, exts)]}

    return [
        k("image", "صورة" if ar else "Image", "صورة واحدة" if ar else "One image", ["photo.png"], img),
        k("video", "فيديو" if ar else "Video", "فيديو واحد" if ar else "One video", ["clip.mov"], vid),
        k("document", "مستند" if ar else "Document", "مستند واحد" if ar else "One document", ["report.docx"], doc),
        k("audio", "صوت" if ar else "Audio", "ملف صوتي" if ar else "One audio file", ["song.wav"], aud),
        k("pdfs", "ملفا PDF" if ar else "Two PDFs", "ملفان معا" if ar else "Two files", ["a.pdf", "b.pdf"], pdf),
    ]


def hive_config(lang):
    ar = lang == "ar"
    return {
        "rtl": ar,
        "r": R, "gap": GAP, "pad": PAD,
        "kinds": kinds(lang),
        "ui": {
            "done": "تم" if ar else "Done",
            "actions": "{n} إجراءات" if ar else "{n} actions",
            "original": "الأصل لا يتغير" if ar else "Original, untouched",
            "fresh": "جديد" if ar else "New",
            "rel": "تنزيل ل{os}" if ar else "Download for {os}",
            "win": "ويندوز" if ar else "Windows",
            "linux": "لينكس" if ar else "Linux",
        },
    }


# ---------------------------------------------------------------------------
# Static pieces
# ---------------------------------------------------------------------------

def mini_cluster(cx, cy, radius, rtl=False, classes=None):
    """Seven small cells (centre plus ring 1) around (cx, cy)."""
    s = radius + 2 / SQRT3
    parts = []
    for i, (q, r) in enumerate(spiral(1)):
        x = s * (SQRT3 * q + SQRT3 / 2 * r)
        if rtl:
            x = -x
        cls = (classes or {}).get(i, "mc")
        parts.append(f'<path class="{cls}" d="{hex_path(cx + x, cy + s * 1.5 * r, radius)}"/>')
    return "".join(parts)


# ---------------------------------------------------------------------------
# Language packs. Two originals with the same structure. Short on purpose:
# big headlines, few words, the drawings explain the rest.
# ---------------------------------------------------------------------------

TAG = "v" + VERSION
PKGV = VERSION.replace("-", ".")  # GitHub stores "~" as "." (S-073)
DL = f"{REPO}/releases/download/{TAG}/"
FILES = {
    "msi": DL + f"kawaro-{VERSION}-windows-x64.msi",
    "deb": DL + f"kawaro_{PKGV}_amd64.deb",
    "rpm": DL + f"kawaro-{PKGV}-1.x86_64.rpm",
}
SUPPORT = "https://sbarah.com/support"

AR = {
    "lang": "ar", "dir": "rtl", "name": "كوارو", "locale": "ar_AR", "alt": "en_US",
    "nav_skip": "انتقل إلى المحتوى",
    "nav": {"index": "الرئيسية", "docs": "التوثيق", "releases": "الإصدارات", "windows": "ويندوز"},
    "nav_label": "التنقل الرئيسي",
    "other_lang": "English",
    "beta": "بيتا",
    "support": "ادعم صبارة",
    "titles": {
        "index": "كوارو: حوّل ملفاتك بأمان على جهازك",
        "windows": "كوارو على ويندوز: تحذير SmartScreen والتحقق من الملف",
    },
    "descs": {
        "index": "محوّل ملفات مجاني ومفتوح المصدر للينكس وويندوز. حدّد الملف، اختر خلية، والناتج بجانب الأصل. ملفاتك تبقى على جهازك.",
        "windows": "بناء ويندوز موقَّع بشهادة اختبار ذاتية، لا بناشر موثَّق، فيظهر تحذير SmartScreen. تحقق من الملف بـ SHA256SUMS ثم More info وRun anyway.",
    },
    "og_alt": "شعار كوارو بجانب الخلية السداسية، وعلى خلاياها صيغ تحويل الصور",
    "footer_license": "برنامج حر برخصة <bdi dir=\"ltr\">GPLv3</bdi>.",
    "footer_beta": "<bdi dir=\"ltr\">" + VERSION + "</bdi>",
    "releases": "الإصدارات",
}

EN = {
    "lang": "en", "dir": "ltr", "name": "Kawaro", "locale": "en_US", "alt": "ar_AR",
    "nav_skip": "Skip to content",
    "nav": {"index": "Home", "docs": "Docs", "releases": "Releases", "windows": "Windows"},
    "nav_label": "Main navigation",
    "other_lang": "العربية",
    "beta": "beta",
    "support": "Support Sbarah",
    "titles": {
        "index": "Kawaro: convert files safely, on your machine",
        "windows": "Kawaro on Windows: the SmartScreen warning and checking your download",
    },
    "descs": {
        "index": "A free, open source file converter for Linux and Windows. Select the file, pick a cell, and the result lands next to the original. Your files stay on your machine.",
        "windows": "The Windows build is test-signed with a self-signed certificate, not a verified publisher, so SmartScreen warns on first run. Verify against SHA256SUMS, then click More info and Run anyway.",
    },
    "og_alt": "The Kawaro logo next to a hexagonal hive with image formats",
    "footer_license": "Free software under the GPLv3.",
    "footer_beta": VERSION,
    "releases": "Releases",
}


def lang_pack(lang):
    return AR if lang == "ar" else EN


# ---------------------------------------------------------------------------
# Page shell
# ---------------------------------------------------------------------------

def _parts(page):
    p = page.split("/")
    if p[-1] == "index":
        p = p[:-1]
    return p


def rel_root(lang, page):
    depth = (1 if lang == "en" else 0) + len(_parts(page))
    return "../" * depth


def dl_anchor(lang, page):
    """The download section lives on the home page: one anchor, no separate page."""
    return page_href(lang, page, lang, "index") + "#download"


def verify_href(lang, page):
    """The checksum steps are the Verify section of the install guide."""
    return page_href(lang, page, lang, "docs/install") + "#" + ("التحقق" if lang == "ar" else "verify")


def page_href(from_lang, from_page, to_lang, to_page):
    here = (["en"] if from_lang == "en" else []) + _parts(from_page)
    there = (["en"] if to_lang == "en" else []) + _parts(to_page)
    common = 0
    while common < min(len(here), len(there)) and here[common] == there[common]:
        common += 1
    up = "../" * (len(here) - common)
    down = "/".join(there[common:])
    target = up + down
    if down:
        target += "/"
    return target or "./"


def abs_url(lang, page):
    parts = (["en"] if lang == "en" else []) + _parts(page)
    return BASE + "/".join(parts) + ("/" if parts else "")


COMPONENT_STEMS = ("hex-button", "section-head", "hex-plate", "step-cell", "format-cell", "download-slab")
CSS_BASE = (["style.css", "css/header.css"] + [f"css/components/{s}.css" for s in COMPONENT_STEMS]
            + ["css/pages.css", "css/film.css"])
CSS_HOME = ["css/hero.css", "css/demo.css", "css/marquee.css", "css/bento.css",
            "css/formats.css", "css/closing.css", "css/motion.css"]
CSS_DOCS = ["css/docs.css"]


def css_links(page, root):
    files = list(CSS_BASE)
    if page == "index":
        files += CSS_HOME
    if page == "releases" or page.startswith("docs"):
        files += CSS_DOCS
    return "\n".join(f'<link rel="stylesheet" href="{root}{f}">' for f in files)


def head(lang, page, root, title=None, desc=None, extra_ld=()):
    p = lang_pack(lang)
    title = html.escape(title or p["titles"][page])
    desc = html.escape(desc or p["descs"][page])
    other = "en" if lang == "ar" else "ar"
    ld = {
        "@context": "https://schema.org", "@type": "SoftwareApplication",
        "name": "Kawaro", "alternateName": "كوارو",
        "applicationCategory": "UtilitiesApplication", "operatingSystem": "Linux, Windows",
        "softwareVersion": VERSION, "isAccessibleForFree": True,
        "license": "https://www.gnu.org/licenses/gpl-3.0.html",
        "url": BASE, "codeRepository": REPO,
        "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"},
    }
    return f'''<!doctype html>
<html lang="{lang}" dir="{p["dir"]}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="{desc}">
<link rel="canonical" href="{abs_url(lang, page)}">
<link rel="alternate" hreflang="{lang}" href="{abs_url(lang, page)}">
<link rel="alternate" hreflang="{other}" href="{abs_url(other, page)}">
<link rel="alternate" hreflang="x-default" href="{abs_url("ar", page)}">
<meta name="color-scheme" content="light">
<link rel="icon" href="{FAVICON}" type="image/svg+xml">
<link rel="icon" href="{root}favicon-32.png" sizes="32x32" type="image/png">
<link rel="apple-touch-icon" href="{root}apple-touch-icon.png">
<meta property="og:type" content="website">
<meta property="og:site_name" content="{p["name"]}">
<meta property="og:locale" content="{p["locale"]}">
<meta property="og:locale:alternate" content="{p["alt"]}">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{desc}">
<meta property="og:url" content="{abs_url(lang, page)}">
<meta property="og:image" content="{BASE}og.png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="{html.escape(p["og_alt"])}">
<meta name="twitter:card" content="summary_large_image">
<link rel="preload" href="{root}fonts/IBMPlexSansArabic-SemiBold.woff2" as="font" type="font/woff2" crossorigin>
<link rel="stylesheet" href="{root}tokens.css">
{css_links(page, root)}
<script type="application/ld+json">
{json.dumps(ld, ensure_ascii=False)}
</script>
{"".join('<script type="application/ld+json">' + json.dumps(x, ensure_ascii=False) + '</script>' + chr(10) for x in extra_ld)}
<script>document.documentElement.classList.add("js")</script>
</head>
<body class="{"home" if page == "index" else "inner"}">
<a class="skip" href="#main">{p["nav_skip"]}</a>
'''


HEART = ('<svg class="ico" viewBox="0 0 24 24" aria-hidden="true" focusable="false">'
         '<path d="M12 21L4 12.5V7.5L8 4L12 7L16 4L20 7.5V12.5Z"/></svg>')


# ---------------------------------------------------------------------------
# The six section helpers (S-063 AC5). Each has its own file in site/css/components/.
# ---------------------------------------------------------------------------

def hex_button(href, label, kind="plain", size="md", attrs="", extra=""):
    """A flat-top hexagon button: edge and fill are two clipped layers."""
    size_cls = "" if size == "md" else f" hex-button-{size}"
    more = f" {extra}" if extra else ""
    return f'<a class="hex-button button-{kind}{size_cls}{more}" href="{href}"{attrs}>{label}</a>'


def section_head(ident, title, lede=""):
    """A heading block: a small hexagon mark, the section's one h2 and an optional lede."""
    p = f'<p class="section-head-lede">{lede}</p>' if lede else ""
    return (f'<div class="section-head"><span class="section-head-mark" aria-hidden="true"></span>'
            f'<h2 id="{ident}">{title}</h2>{p}</div>')


def hex_plate(inner, tag="div", tone=""):
    """A wide plate with pointed ends, the web twin of a hive plate."""
    cls = "hex-plate" + (f" hex-plate-{tone}" if tone else "")
    return f'<{tag} class="{cls}">{inner}</{tag}>'


def step_cell(n, title, text=""):
    """One step: a pointy-top hexagon with its number, a title and one sentence."""
    p = f"<p>{text}</p>" if text else ""
    return f'<li class="step-cell"><span class="step-cell-num">{n}</span><h3>{title}</h3>{p}</li>'


def format_cell(title, codes):
    """A pointy-top hexagon holding a family of formats."""
    return f'<li class="format-cell"><h3>{title}</h3><p class="format-cell-codes">{codes}</p></li>'


def support_a(cls, inner, extra=""):
    """The one support link shape: exact URL, new tab, no opener."""
    return (f'<a class="{cls}" href="{SUPPORT}" target="_blank" rel="noopener noreferrer"{extra}>'
            f'{inner}</a>')


def support_link(lang, cls="hex-button button-support hex-button-sm"):
    text = lang_pack(lang)["support"]
    return support_a(cls, f'{HEART}<span>{text}</span>', f' aria-label="{text}"')


def header(lang, page, root):
    p = lang_pack(lang)
    other = "en" if lang == "ar" else "ar"
    links = []
    for key in ("docs", "releases"):
        cur = ' aria-current="page"' if (key == page or page.startswith(key + "/")) else ""
        links.append(f'<a class="nav-link" href="{page_href(lang, page, lang, key)}"{cur}>{p["nav"][key]}</a>')
    sw = (f'<a class="nav-link lang-switch" href="{page_href(lang, page, other, page)}" '
          f'hreflang="{other}" lang="{other}">{p["other_lang"]}</a>')
    sup = support_link(lang, "nav-link nav-support")
    cta = hex_button(dl_anchor(lang, page), "تنزيل" if lang == "ar" else "Download", "primary", "sm",
                     "", extra="nav-cta")
    meter = '<div class="scroll-meter" aria-hidden="true"></div>' if page == "index" else ""
    return f'''<header class="site-header">
  <div class="wrap">
    <a class="brand" href="{page_href(lang, page, lang, "index")}">
      <img src="{root}favicon.svg" alt="" width="48" height="48">
      <span>{p["name"]}</span>
    </a>
    <nav class="site-nav" aria-label="{p["nav_label"]}">
      {"".join(links)}
      {sup}
      {sw}
      {cta}
    </nav>
  </div>
</header>
<main id="main">
{meter}
'''


def footer_cells():
    """The foot of every page: a full honeycomb lattice whose rows fade in steps
    from the name upward, so the field reads as a gradation, not as clumps.
    Cells grow in as the footer scrolls into view, bottom row first."""
    r = 13
    pitch = r * math.sqrt(3)
    rows, cols = 5, 48
    h = r * (1.5 * rows + 0.5)
    fade = (0.1, 0.18, 0.28, 0.4, 0.52)  # top row first, all translucent
    polys = []
    for row in range(rows):
        y = h - r * (1.5 * (rows - 1 - row) + 1)
        for col in range(cols + 1):
            x = pitch * (col + (0.5 if row % 2 else 0)) + pitch / 2
            pts = " ".join(f"{x + r * math.cos(math.radians(a)):.1f},{y + r * math.sin(math.radians(a)):.1f}"
                           for a in (-90, -30, 30, 90, 150, 210))
            polys.append(f'<polygon class="fc" style="--i:{rows - 1 - row}" opacity="{fade[row]}" '
                         f'points="{pts}"/>')
    total = pitch * cols + pitch / 2
    return (f'<svg class="footer-cells" viewBox="0 0 {total:.0f} {h:.0f}"  '
            f'focusable="false">{"".join(polys)}</svg>')


def footer(lang, page, root):
    p = lang_pack(lang)
    other = "en" if lang == "ar" else "ar"
    product = [
        f'<a href="{REPO}">GitHub</a>',
        f'<a href="{page_href(lang, page, lang, "windows")}">{p["nav"]["windows"]}</a>',
        f'<a href="{RELEASES}">GitHub {p["releases"]}</a>',
        support_a("", p["support"]),
    ]
    items = "".join(f"<li>{x}</li>" for x in product)
    lang_sw = (f'<a href="{page_href(lang, page, other, page)}" hreflang="{other}" lang="{other}">'
               f'{p["other_lang"]}</a>')
    scripts = f'<script src="{root}app.js" defer></script>'
    # Arabic is stretched with kashida (tatweel), the calligraphic way, never with letter spacing.
    mark_text = "ك" + "\u0640" * 18 + "وارو" if lang == "ar" else p["name"]
    cells = footer_cells()


    return f'''</main>
<footer class="site-footer">
  <div class="wrap">
    <div class="footer-row">
      <p>{p["footer_license"]} <span class="muted">{p["footer_beta"]}</span></p>
      <ul class="footer-product" role="list">{items}</ul>
      <ul class="footer-lang" role="list"><li>{lang_sw}</li></ul>
    </div>
  </div>
  <div class="footer-band" aria-hidden="true">
    <div class="footer-cells">{cells}</div>
    <p class="footer-mark">{mark_text}</p>
  </div>
</footer>
<script id="hive-config" type="application/json">{json.dumps(hive_config(lang), ensure_ascii=False)}</script>
{scripts}
</body>
</html>
'''


def sec(inner, *, cls="", ident=None, labelledby=None, **_):
    attrs = f' id="{ident}"' if ident else ""
    if labelledby:
        attrs += f' aria-labelledby="{labelledby}"'
    classes = ("sec " + cls).strip()
    return f'<section class="{classes}"{attrs}>\n<div class="wrap">\n{inner}\n</div>\n</section>\n'


def code(text):
    return f'<pre><code>{html.escape(text)}</code></pre>'


# ---------------------------------------------------------------------------
# Shared blocks
# ---------------------------------------------------------------------------

WIN_ICO = ('<svg class="os-ico" viewBox="0 0 48 48" aria-hidden="true" focusable="false">'
           '<path d="M6 6H23V23H6ZM25 6H42V23H25ZM6 25H23V42H6ZM25 25H42V42H25Z"/></svg>')
LIN_ICO = ('<svg class="os-ico" viewBox="0 0 48 48" aria-hidden="true" focusable="false">'
           '<path class="line" d="M5 9H43V39H5Z"/>'
           '<path class="line" d="M14 20L22 26L14 32M26 33H35"/></svg>')


LATIN = re.compile(r"\b(?:MSI|DEB|RPM|EXE)\b|\d+\.\d+\.\d+(?:-[A-Za-z]+\.\d+)?")


def bidi(text, lang):
    """Arabic text: isolate Latin file codes and versions so they keep their order."""
    if lang != "ar":
        return text
    parts = re.split(r"(<[^>]*>)", text)
    return "".join(x if x.startswith("<") else LATIN.sub(lambda m: f"<bdi>{m.group(0)}</bdi>", x) for x in parts)


def dl_link(lang, kind, label, button="plain", size="md"):
    """A download button. Until the release exists it opens the releases page."""
    href = FILES[kind] if PUBLISHED else RELEASES
    return hex_button(href, label, button, size, f' data-dl="{kind}"')


def dl_buttons(lang, which):
    """Direct download buttons. Names follow scripts/release/dry-run.sh and make-msi.sh."""
    ar = lang == "ar"
    word = "تنزيل" if ar else "Download"
    if which == "win":
        return dl_link(lang, "msi", bidi(f"{word} MSI", lang), "primary", "lg")
    return (dl_link(lang, "deb", bidi(f"{word} DEB", lang), "primary", "lg")
            + dl_link(lang, "rpm", bidi(f"{word} RPM", lang), "primary", "lg"))


def download_slab(lang, which, note=False):
    """One operating system: a cut corner slab with its icon, its note and its buttons."""
    ar = lang == "ar"
    if which == "win":
        title, sub, icon, card = ("ويندوز" if ar else "Windows"), ("10 و11، 64 بت" if ar else "10 and 11, 64 bit"), WIN_ICO, "win"
    else:
        title, sub, icon, card = ("لينكس" if ar else "Linux"), ("فيدورا وأوبونتو وديبيان، 64 بت" if ar else "Fedora, Ubuntu, Debian, 64 bit"), LIN_ICO, "linux"
    n = ""
    if which == "win" and note:
        # One plain sentence (S-083 owns the honesty rule): it never claims the
        # build is signed, never says the warning is gone, never says disable.
        guide = REPO + "/blob/main/docs/windows-smartscreen" + (".ar.md" if ar else ".md")
        label = "لماذا؟" if ar else "Why?"
        text = ("إذا ظهر لك تحذير أزرق من ويندوز: اضغط «مزيد من المعلومات» ثم «تشغيل على أي حال». "
                "ويندوز لا يعرف الناشر بعد. "
                if ar else
                "If Windows shows a blue warning: click More info, then Run anyway. "
                "Windows does not know this publisher yet. ")
        n = f'<p class="download-note">{text}<a href="{guide}">{label}</a></p>'
    return (f'<div class="download-slab" data-card="{card}">{icon}<h3>{title}</h3>'
            f'<p class="download-slab-sub">{sub}</p>'
            f'<div class="actions">{dl_buttons(lang, which)}</div>{n}</div>')


def dl_cards(lang, note=False):
    return f'<div class="dl-grid">{download_slab(lang, "win", note)}{download_slab(lang, "linux")}</div>'


def pills(lang, verify_href, win_href):
    ar = lang == "ar"
    return (f'<ul class="facts" role="list">'
            f'<li><span class="pill pill-hot"><bdi>{VERSION}</bdi></span></li>'
            f'<li><a href="{win_href}">{"ويندوز بتوقيع اختباري" if ar else "Windows is test-signed"}</a></li>'
            f'<li><a href="{verify_href}">{"تحقق من الملف" if ar else "Verify the file"}</a></li>'
            f'</ul>')


# ---------------------------------------------------------------------------
# Home (S-064). One idea per section, in this order: hero with a live demo,
# a band of conversions, four facts in a bento grid, the try-it demo, the
# formats from actions/*.toml, the download, then open source and support.
# The words are short on purpose; the drawings and the motion carry the rest.
# ---------------------------------------------------------------------------

T_HOME = {
    "ar": {
        "h1a": "حوّل ملفاتك",
        "h1b": "بأمان على <span class=\"hl\">جهازك</span>",
        "sub": "حدّد الملف وانقر بالزر الأيمن، فتظهر الخلية بما يناسبه. التحويل كله يجري في جهازك: لا يُرفع أي ملف، ويُحفظ الناتج بجانب الأصل.",
        "dl": "تنزيل مجاني",
        "win_link": "ويندوز",
        "note": "مفتوح المصدر، للينكس و{win}.",
        "demo_label": "عرض متحرك: تحدّد صورة في مجلد، وتفتح الخلية وتختار WEBP، فيظهر ملف جديد بجانب الأصل",
        "demo_folder": "الصور",
        "demo_centre": "صورة",
        "demo_new": "جديد",
        "demo_steps": ["حدّد الملف", "اختر الإجراء", "الناتج بجانبه"],
        "bento_h2": "تحوّل الخلية ملفك، ثم تختفي",
        "privacy_k": "الخصوصية",
        "privacy_h": "ملفاتك لا تغادر جهازك",
        "privacy_p": "كل تحويل يجري في جهازك: لا رفع، ولا حساب، ولا تتبع.",
        "orig_h": "الأصل لا يتغير",
        "orig_p": "يُحفظ الناتج في ملف جديد بجانبه، ويبقى ملفك كما هو.",
        "orig_kept": "الأصل",
        "fit_h": "لا ترى إلا ما يناسب ملفك",
        "fit_p": "إن حدّدت صورة رأيت صيغ الصور، وإن حدّدت ملفي PDF رأيت الدمج والتقسيم.",
        "lang_h": "واجهة عربية كاملة",
        "lang_p": "تعمل بالعربية والإنجليزية، والعربية فيها من اليمين إلى اليسار.",
        "formats_h2": "{n} إجراء في خلية واحدة",
        "formats_p": "لا يظهر الإجراء في الخلية إن لم تكن أداته مثبتة في جهازك.",
        "formats_all": "كل الصيغ والإجراءات",
        "families": {"images": "صور", "video": "فيديو", "audio": "صوت", "documents": "مستندات", "pdf": "PDF"},
        "get": "نزّل كوارو",
        "get_p": "مجاني وبلا حساب، ويحوّل ملفاتك دون اتصال بالإنترنت.",
        "open": "مجاني ومفتوح المصدر",
        "open_p": "برخصة <bdi dir=\"ltr\">GPLv3</bdi>: اقرأ الشيفرة، وعدّلها، وشاركها.",
        "gh": "الشيفرة على GitHub",
        "support_h": "ادعم صبارة",
        "support_p": "كوارو مجاني. إن وفّر عليك وقتا فيمكنك دعم صبارة.",
    },
    "en": {
        "h1a": "Convert files",
        "h1b": "safely, <span class=\"hl\">locally</span>",
        "sub": "Select a file and right click: the hive shows what fits it. Everything runs on your machine; nothing is uploaded, and the result is saved next to the original.",
        "dl": "Free download",
        "win_link": "Windows",
        "note": "Open source, for Linux and {win}.",
        "demo_label": "Animation: a picture is selected in a folder, the hive opens, WEBP is picked, and a new file appears next to the original",
        "demo_folder": "Pictures",
        "demo_centre": "Image",
        "demo_new": "New",
        "demo_steps": ["Select the file", "Pick an action", "Saved beside it"],
        "bento_h2": "The hive converts your file, then gets out of the way",
        "privacy_k": "Privacy",
        "privacy_h": "Your files never leave your machine",
        "privacy_p": "Every conversion runs on your machine: no uploads, no account, no tracking.",
        "orig_h": "The original stays as it is",
        "orig_p": "The result goes to a new file beside it. Your file is never modified.",
        "orig_kept": "Original",
        "fit_h": "You only see what fits your file",
        "fit_p": "Select an image and you see image formats; select two PDFs and you see merge and split.",
        "lang_h": "A full Arabic interface",
        "lang_p": "It works in Arabic and English, with Arabic right to left.",
        "formats_h2": "{n} actions in one hive",
        "formats_p": "An action only shows in the hive when its tool is installed on your machine.",
        "formats_all": "All formats and actions",
        "families": {"images": "Images", "video": "Video", "audio": "Audio", "documents": "Documents", "pdf": "PDF"},
        "get": "Get Kawaro",
        "get_p": "Free, no account, and it converts your files offline.",
        "open": "Free and open source",
        "open_p": "Under the GPLv3: read the code, change it, share it.",
        "gh": "Source on GitHub",
        "support_h": "Support Sbarah",
        "support_p": "Kawaro is free. If it saves you time, you can support Sbarah.",
    },
}

# The hero demo: a folder window, a cursor, the hive and the result. Every
# position is in container units (cqw) of the window, so the picture scales as
# one piece; x is mirrored for right to left in CSS through --dir.
DEMO_FILES = [("PNG", "photo.png", "pick"), ("", "", "slot"), ("MOV", "clip.mov", ""),
              ("WAV", "song.wav", ""), ("DOCX", "report.docx", ""), ("PDF", "notes.pdf", "")]
DEMO_RING = ["WEBP", "JPG", "AVIF", "PDF", "GIF", "ICO"]
# Ring one around the centre, pointy-top cells: (x, y) in cqw from the hive centre.
DEMO_STEP_X, DEMO_STEP_Y = 17.0, 14.7
DEMO_SLOTS = [(DEMO_STEP_X, 0), (DEMO_STEP_X / 2, -DEMO_STEP_Y), (-DEMO_STEP_X / 2, -DEMO_STEP_Y),
              (-DEMO_STEP_X, 0), (-DEMO_STEP_X / 2, DEMO_STEP_Y), (DEMO_STEP_X / 2, DEMO_STEP_Y)]

CURSOR = ('<svg class="demo-cursor-ico" viewBox="0 0 24 24" aria-hidden="true" focusable="false">'
          '<path d="M3 2L20 12L12.5 13.5L16.5 21L13.5 22.5L9.5 15L3 20Z"/></svg>')


FILM_LABEL = "فيديو قصير بلا صوت: يسحب المستخدم ملفاً، ويفتح الخلية، ويفلته على الصيغة، فيظهر ملف جديد بجانب الأصل"


FILM_LABEL_EN = ("Short silent video: the user drags a file, opens the hive cell with a shortcut "
                 "and drops the file on the format, and a new file appears next to the original")


def hero_film(lang, root=""):
    """The hero picture: the product film (S-068 Arabic, S-070 English), silent,
    looping, one file per format in media/. The poster is the demo scene, so a
    visitor who prefers reduced motion, or whose browser blocks autoplay, still
    sees the product."""
    en = lang == "en"
    label = html.escape(FILM_LABEL_EN if en else FILM_LABEL)
    stem = "hero-en" if en else "hero"
    return (f'<figure class="film"><video class="film-video" width="1280" height="720" '
            f'poster="{root}media/{stem}-poster.webp" autoplay muted loop playsinline preload="auto" aria-label="{label}">'
            f'<source src="{root}media/{stem}.webm" type="video/webm"><source src="{root}media/{stem}.mp4" type="video/mp4">'
            f'</video></figure>')


def hero_demo(lang):
    """The hero picture: a looping, CSS only story of one conversion (S-064 AC3).
    Without motion it shows the end of the story: the result beside the original."""
    t = T_HOME[lang]
    tiles = []
    for code_, name, role in DEMO_FILES:
        if role == "slot":
            tiles.append(f'<li class="demo-file demo-slot"><span class="demo-result"><span class="demo-ext">WEBP</span>'
                         f'<span class="demo-name">photo.webp</span><span class="demo-tag">{t["demo_new"]}</span></span></li>')
            continue
        cls = "demo-file" + (" demo-pick" if role == "pick" else "")
        tiles.append(f'<li class="{cls}"><span class="demo-ext">{code_}</span><span class="demo-name">{name}</span></li>')
    cells = [f'<span class="demo-cell demo-centre" style="--x:0;--y:0;--i:0">{t["demo_centre"]}</span>']
    for i, (code_, (x, y)) in enumerate(zip(DEMO_RING, DEMO_SLOTS), 1):
        hot = " demo-hot" if i == 1 else ""
        cells.append(f'<span class="demo-cell{hot}" style="--x:{x:g};--y:{y:g};--i:{i}">{code_}</span>')
    steps = "".join(f'<li class="demo-step" style="--s:{i}"><b>{i}</b>{s}</li>' for i, s in enumerate(t["demo_steps"], 1))
    return f'''<figure class="demo" role="img" aria-label="{html.escape(t["demo_label"])}">
  <div class="demo-window" aria-hidden="true">
    <div class="demo-bar"><i></i><i></i><i></i><span>{t["demo_folder"]}</span></div>
    <ul class="demo-files" role="list">{"".join(tiles)}</ul>
    <div class="demo-hive">{"".join(cells)}</div>
    <div class="demo-pointer"><span class="demo-cursor">{CURSOR}</span></div>
  </div>
  <ol class="demo-steps" aria-hidden="true">{steps}</ol>
</figure>'''


MARQUEE = [("PNG", "WEBP"), ("MOV", "MP4"), ("WAV", "MP3"), ("DOCX", "PDF"), ("JPG", "AVIF"),
           ("MP4", "GIF"), ("FLAC", "OGG"), ("ODT", "DOCX"), ("PNG", "PDF"), ("MKV", "WEBM")]
ARROW = ('<svg class="mq-arrow" viewBox="0 0 24 24" aria-hidden="true" focusable="false">'
         '<path d="M3 10.5H15.5L11 6L13 4L21 12L13 20L11 18L15.5 13.5H3Z"/></svg>')


def marquee(lang):
    """A band of conversions that drifts across the page, twice for a seamless loop.
    Decoration only: the formats section below says the same thing in words."""
    rows = []
    for row, pairs in enumerate((MARQUEE, MARQUEE[5:] + MARQUEE[:5])):
        one = "".join(f'<li><span>{a}</span>{ARROW}<span>{b}</span></li>' for a, b in pairs)
        rows.append(f'<div class="mq-row mq-row-{row + 1}"><ul class="mq-track" role="list">{one}{one}</ul></div>')
    return f'<div class="mq" aria-hidden="true">{"".join(rows)}</div>\n'

def bento_tile(cls, inner, tag="li"):
    """One tile of the facts grid: a slab with one cut corner, its own surface tone."""
    return f'<{tag} class="bento-tile {cls}">{inner}</{tag}>'


LOCK = ('<svg class="bento-lock" viewBox="0 0 120 138" aria-hidden="true" focusable="false">'
        '<path class="bl-hex" d="M60 3L117 36V102L60 135L3 102V36Z"/>'
        '<path class="bl-body" d="M36 62H84V104H36Z"/><path class="bl-loop" d="M44 62V48A16 16 0 0 1 76 48V62H69V48A9 9 0 0 0 51 48V62Z"/>'
        '<path class="bl-key" d="M56 76H64V92H56Z"/></svg>')


def bento_section(lang):
    t = T_HOME[lang]
    fit_cells = "".join(f'<i style="--q:{q};--r:{r}"{" class=on" if i in (0, 1, 4) else ""}></i>'
                        for i, (q, r) in enumerate(spiral(1)))
    tiles = [
        bento_tile("bento-privacy", f'''{LOCK}
<p class="bento-kicker">{t["privacy_k"]}</p>
<h3>{t["privacy_h"]}</h3>
<p>{t["privacy_p"]}</p>'''),
        bento_tile("bento-orig", f'''<div class="bento-files" aria-hidden="true">
<span class="bf bf-old"><b>PNG</b><small>{t["orig_kept"]}</small></span>
<span class="bf bf-new"><b>WEBP</b><small>{t["demo_new"]}</small></span></div>
<h3>{t["orig_h"]}</h3><p>{t["orig_p"]}</p>'''),
        bento_tile("bento-fit", f'<div class="bento-hive" aria-hidden="true">{fit_cells}</div>'
                                f'<h3>{t["fit_h"]}</h3><p>{bidi(t["fit_p"], lang)}</p>'),
        bento_tile("bento-lang", f'<div class="bento-glyphs" aria-hidden="true"><span lang="ar">ع</span>'
                                 f'<span lang="en">A</span></div><h3>{t["lang_h"]}</h3><p>{t["lang_p"]}</p>'),
    ]
    inner = f'''{section_head("bento-title", t["bento_h2"])}
<ul class="bento" role="list">{"".join(tiles)}</ul>'''
    return sec(inner, cls="s-bento", ident="why", labelledby="bento-title")


def families(lang):
    """Action families from actions/*.toml: (key, name, codes in ring order)."""
    ar = lang == "ar"
    acts = {}
    for f in sorted((ROOT / "actions").glob("*.toml")):
        for a in tomllib.loads(f.read_text(encoding="utf-8"))["action"]:
            acts.setdefault(a["accepts"][0], []).append(a)
    out = []
    for key, kind, _, _ in GROUPS:
        group = sorted(acts.get(kind, []), key=lambda a: (a.get("ring", 1), a.get("order", 0)))
        codes = []
        for a in group:
            c = a["code"]["ar" if ar else "en"]
            if c not in codes:
                codes.append(c)
        out.append((key, len(group), codes))
    return out


def count_label(lang, n):
    """Action count with the Arabic number agreement: 3 to 10 take the plural."""
    if lang != "ar":
        return f"{n} action" + ("" if n == 1 else "s")
    return f"{n} " + ("إجراءات" if 3 <= n <= 10 else "إجراء")


def formats_section(lang):
    t = T_HOME[lang]
    fams = families(lang)
    total = sum(n for _, n, _ in fams)
    rows = []
    for i, (key, n, codes) in enumerate(fams):
        chips = "".join(f'<li><bdi>{html.escape(c)}</bdi></li>' for c in codes)
        rows.append(f'''<li class="fam" style="--i:{i}">
<h3 class="fam-name">{t["families"][key]}<sup aria-label="{count_label(lang, n)}"><bdi>{n}</bdi></sup></h3>
<ul class="fam-codes" role="list" aria-label="{count_label(lang, n)}">{chips}</ul></li>''')
    href = page_href(lang, "index", lang, "docs/formats")
    inner = f'''{section_head("formats-title", t["formats_h2"].format(n=f'<bdi>{total}</bdi>'), t["formats_p"])}
<ol class="fams" role="list">{"".join(rows)}</ol>
<div class="actions">{hex_button(href, t["formats_all"], "plain", "md")}</div>'''
    return sec(inner, cls="s-formats", ident="formats", labelledby="formats-title")


def get_section(lang):
    t = T_HOME[lang]
    win_href = page_href(lang, "index", lang, "windows")
    inner = f'''{section_head("get-title", t["get"], t["get_p"])}
{dl_cards(lang, note=True)}
{pills(lang, verify_href(lang, "index"), win_href)}'''
    return sec(inner, cls="s-get", ident="download", labelledby="get-title")


def closing_section(lang):
    """Open source and support, side by side. The support tile keeps the Sbarah theme."""
    t, p, r = T_HOME[lang], lang_pack(lang), rel_root(lang, "index")
    pic = f'<img src="{r}assets/support-sbarah-light.svg" alt="{p["support"]}" width="520" height="140">'
    inner = f'''<div class="closing">
<div class="closing-open">
<h2 id="open-title">{t["open"]}</h2>
<p>{t["open_p"]}</p>
<div class="actions">{hex_button(REPO, t["gh"], "primary", "lg")}</div>
</div>
<div class="closing-support" id="support">
<h3>{t["support_h"]}</h3>
<p>{t["support_p"]}</p>
{support_a("support-badge", pic)}
</div>
</div>'''
    return sec(inner, cls="s-closing", ident="open", labelledby="open-title")


def home(lang):
    t = T_HOME[lang]
    note = t["note"].format(win=t["win_link"])
    btn = hex_button("#download", t["dl"], "primary", "lg")
    hero = f'''<section class="hero" aria-labelledby="hero-title">
<div class="wrap hero-grid">
<h1 id="hero-title"><span class="h1-line"><span>{t["h1a"]}</span></span>
<span class="h1-line"><span>{t["h1b"]}</span></span></h1>
<div class="hero-side">
<p class="lead">{t["sub"]}</p>
<div class="actions">{btn}</div>
<p class="hero-note">{note}</p>
</div>
<div class="hero-stage">{hero_film(lang, rel_root(lang, "index"))}</div>
</div>
</section>
'''
    return (hero + marquee(lang) + bento_section(lang) + formats_section(lang)
            + get_section(lang) + closing_section(lang))


# ---------------------------------------------------------------------------
# Inner pages
# ---------------------------------------------------------------------------

def page_head(title, lead=""):
    lead_html = f'<p class="lead">{lead}</p>' if lead else ""
    return (f'<section class="page-head sec"><div class="wrap">'
            f'<h1>{title}</h1>{lead_html}</div></section>\n')


def card(title, body="", tag=""):
    tg = f'<span class="tag {tag[0]}">{tag[1]}</span>' if tag else ""
    bd = f"<p>{body}</p>" if body else ""
    return f'<div class="card">{tg}<h3>{title}</h3>{bd}</div>'


def windows(lang):
    ar = lang == "ar"
    dl = verify_href(lang, "windows")
    if ar:
        h = page_head("كوارو على ويندوز", "موقَّع بشهادة اختبار ذاتية، لا بناشر موثَّق، فيظهر تحذير SmartScreen")
        a = sec(f'''<h2 id="w1">ثلاث خطوات</h2>
<ol class="step-row" role="list">{step_cell(1, "تحقق من الملف", f'<a href="{dl}">SHA256SUMS</a>')}{step_cell(2, "More info")}{step_cell(3, "Run anyway")}</ol>
<p class="section-intro">البناء موقَّع بشهادة اختبار ذاتية، لا بشهادة من سلطة موثوقة، فلا يظهر الناشر موثَّقاً. شهادة موثوقة مطلوبة للثقة الكاملة، ولا نعد بموعد.</p>''', cls="s-w1", ident="why", labelledby="w1", edge=False, side="start")
        b = sec(f'''<h2 id="w2">لم يكتمل بعد</h2>
<div class="grid grid-3">{card("لم يُجرَّب على جهاز ويندوز حقيقي", "", ("tag-warn", "تنبيه"))}{card("بلا خلية الإفلات بعد", "", ("tag-warn", "غير جاهز"))}{card("بلا إشعارات بعد", "التقدم في تلميح الأيقونة", ("tag-warn", "غير جاهز"))}</div>''',
                cls="alt s-w2", ident="gaps", labelledby="w2", side="end")
    else:
        h = page_head("Kawaro on Windows", "Test-signed with a self-signed certificate, not a verified publisher, so SmartScreen warns on first run")
        a = sec(f'''<h2 id="w1">Three steps</h2>
<ol class="step-row" role="list">{step_cell(1, "Verify the file", f'<a href="{dl}">SHA256SUMS</a>')}{step_cell(2, "More info")}{step_cell(3, "Run anyway")}</ol>
<p class="section-intro">The build is test-signed with a self-signed certificate, not one from a trusted authority, so the publisher is not verified. A trusted certificate is still needed for full trust, and we promise no date.</p>''', cls="s-w1", ident="why", labelledby="w1", edge=False, side="start")
        b = sec(f'''<h2 id="w2">Not finished yet</h2>
<div class="grid grid-3">{card("Not run on a real Windows machine", "", ("tag-warn", "Note"))}{card("No drop hive", "", ("tag-warn", "Missing"))}{card("No notifications", "Progress shows in the tray tooltip", ("tag-warn", "Missing"))}</div>''',
                cls="alt s-w2", ident="gaps", labelledby="w2", side="end")
    return h + a + b


def render(lang, page):
    root = rel_root(lang, page)
    body = home(lang) if page == "index" else windows(lang)
    return head(lang, page, root) + header(lang, page, root) + body + footer(lang, page, root)


# ---------------------------------------------------------------------------
# Docs, formats, releases, search, and the files agents and crawlers read.
# Sources: docs/guide/NAME.md and NAME.ar.md (English main, Arabic copy),
# actions/*.toml (formats) and CHANGELOG.md (releases). Nothing is written twice.
# ---------------------------------------------------------------------------

import tomllib

ROOT = HERE.parent
GUIDE = ROOT / "docs" / "guide"
GUIDE_SLUGS = ["start", "install", "use", "how", "shortcuts", "settings", "troubleshoot", "contribute"]
DOC_ORDER = ["start", "install", "use", "how", "formats", "shortcuts", "settings", "troubleshoot", "contribute"]
EDIT_BASE = REPO + "/blob/main/docs/guide/"
TOOLS = {"ffmpeg": "FFmpeg", "ghostscript": "Ghostscript", "office": "LibreOffice"}
GROUPS = [("images", "image", "الصور", "Images"), ("video", "video", "الفيديو", "Video"),
          ("audio", "audio", "الصوت", "Audio"), ("documents", "document", "المستندات", "Documents"),
          ("pdf", "pdf", "PDF", "PDF")]


def inline(text, lang, page):
    """Escape, then code spans, bold and links. Guide links become site links."""
    parts = re.split(r"(`[^`]+`)", text)
    out = []
    for i, part in enumerate(parts):
        if i % 2:
            out.append(f"<code>{html.escape(part[1:-1])}</code>")
            continue
        part = html.escape(part, quote=False)
        part = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", part)

        def link(m):
            label, target = m.group(1), m.group(2)
            if target.startswith("site:"):
                name, _, frag = target[5:].partition("#")
                href = page_href(lang, page, lang, name) + ("#" + frag if frag else "")
            elif re.fullmatch(r"[a-z]+(\.ar)?\.md", target):
                slug = target.replace(".ar.md", "").replace(".md", "")
                href = page_href(lang, page, lang, "docs/" + slug)
            else:
                href = target
            return f'<a href="{href}">{label}</a>'

        part = re.sub(r"\[([^\]]+)\]\(([^)\s]+)\)", link, part)
        out.append(bidi(part, lang))
    return "".join(out)


def md_to_html(text, lang, page):
    """A small Markdown subset: headings, paragraphs, lists, tables, code fences."""
    lines = text.split("\n")
    out, i, title = [], 0, ""
    while i < len(lines):
        ln = lines[i]
        if ln.startswith("```"):
            buf = []
            i += 1
            while i < len(lines) and not lines[i].startswith("```"):
                buf.append(lines[i])
                i += 1
            out.append(code("\n".join(buf)))
        elif ln.startswith("![") and "(diagram:" in ln:
            alt, name = re.match(r"!\[([^\]]*)\]\(diagram:([a-z]+)\)", ln).groups()
            r_ = rel_root(lang, page)
            out.append(f'<figure class="diagram">'
                       f'<img src="{r_}diagrams/{name}.{lang}.light.svg" alt="{html.escape(alt)}" loading="lazy"></figure>')
        elif ln.startswith("# "):
            title = ln[2:].strip()
            out.append(f"<h1>{inline(title, lang, page)}</h1>")
        elif ln.startswith("## "):
            h = ln[3:].strip()
            slug = re.sub(r"[^\w]+", "-", h.lower(), flags=re.U).strip("-")
            out.append(f'<h2 id="{slug}">{inline(h, lang, page)}</h2>')
        elif ln.startswith("|"):
            rows = []
            while i < len(lines) and lines[i].startswith("|"):
                rows.append([c.strip() for c in lines[i].strip().strip("|").split("|")])
                i += 1
            i -= 1
            head_, body = rows[0], rows[2:]
            th = "".join(f"<th>{inline(c, lang, page)}</th>" for c in head_)
            tb = "".join("<tr>" + "".join(f"<td>{inline(c, lang, page)}</td>" for c in r) + "</tr>" for r in body)
            out.append(f'<div class="table-wrap"><table><thead><tr>{th}</tr></thead><tbody>{tb}</tbody></table></div>')
        elif re.match(r"(\d+\.|-) ", ln):
            ordered = ln[0].isdigit()
            items = []
            while i < len(lines) and re.match(r"(\d+\.|-) ", lines[i]):
                items.append(re.sub(r"^(\d+\.|-) ", "", lines[i]))
                i += 1
            i -= 1
            tag = "ol" if ordered else "ul"
            out.append(f"<{tag}>" + "".join(f"<li>{inline(x, lang, page)}</li>" for x in items) + f"</{tag}>")
        elif ln.strip():
            para = [ln]
            while i + 1 < len(lines) and lines[i + 1].strip() and not re.match(r"(#|```|\||\d+\. |- )", lines[i + 1]):
                i += 1
                para.append(lines[i])
            out.append(f"<p>{inline(' '.join(para), lang, page)}</p>")
        i += 1
    return title, "\n".join(out)


def read_guide(slug, lang):
    path = GUIDE / (f"{slug}.ar.md" if lang == "ar" else f"{slug}.md")
    text = path.read_text(encoding="utf-8")
    return text.split("\n", 2)[2] if text.startswith("[English]") else text


def formats_md(lang):
    """Formats page source, generated from the action files so it never drifts."""
    ar = lang == "ar"
    allowed = {}
    for f in sorted((ROOT / "actions").glob("*.toml")):
        for a in tomllib.loads(f.read_text(encoding="utf-8"))["action"]:
            allowed.setdefault(a["accepts"][0], []).append(a)
    lines = ["# " + ("الصيغ والإجراءات" if ar else "Formats and actions"), "",
             "ما تفعله الخلية لكل نوع ملف. الإجراء الذي تغيب أداته لا يظهر." if ar
             else "What the hive does for each kind of file. An action whose tool is missing does not appear.", ""]
    for _, kind, ar_name, en_name in GROUPS:
        acts = sorted(allowed.get(kind, []), key=lambda a: (a.get("ring", 1), a.get("order", 0)))
        if not acts:
            continue
        lines += [f"## {ar_name if ar else en_name}", "",
                  "| " + ("الإجراء | الأداة | الشرط" if ar else "Action | Tool | Needs") + " |", "| --- | --- | --- |"]
        for a in acts:
            tool = TOOLS.get(a["engine"], "مدمجة" if ar else "built in")
            lab = a["label"]["ar" if ar else "en"]
            need = ("ملفان فأكثر" if ar else "2 or more files") if a.get("min_files", 1) > 1 else ""
            lines.append(f"| {lab} | {tool} | {need} |")
        lines.append("")
    return "\n".join(lines)


def parse_changelog():
    """Releases from CHANGELOG.md: [{version, date, name, items: [(en, ar)]}]."""
    text = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    rels, cur = [], None
    for ln in text.split("\n"):
        m = re.match(r"## \[([^\]]+)\](?: - (\d{4}-\d{2}-\d{2}))?(?: «([^»]+)»)?", ln)
        if m:
            cur = {"version": m.group(1), "date": m.group(2) or "", "name": m.group(3) or "", "items": []}
            rels.append(cur)
        elif cur is not None and ln.startswith("- "):
            body = ln[2:]
            arm = re.search(r"«([^»]+)»\s*$", body)
            ar_text = arm.group(1) if arm else ""
            en = body[: arm.start()].strip() if arm else body
            title = en.split(":")[0].strip() if ":" in en[:140] else en[:90]
            cur["items"].append((title, ar_text))
    return rels


def releases_md(lang):
    ar = lang == "ar"
    lines = ["# " + ("الإصدارات" if ar else "Releases"), ""]
    for r in parse_changelog():
        name = VERSION if r["version"] == "Unreleased" else r["version"]
        lines.append(f"## {name}" + (f" ({r['date']})" if r["date"] else ""))
        for en, arx in r["items"][:6]:
            lines.append("- " + ((arx or en) if ar else en))
        lines.append("")
    return "\n".join(lines)


SEARCH = {
    "ar": ("ابحث في التوثيق", "لا نتائج", "بحث"),
    "en": ("Search the docs", "No results", "Search"),
}


def search_box(lang, root):
    ph, none, lab = SEARCH[lang]
    return (f'<form class="search" role="search" data-pagefind-ignore data-js="{root}pagefind/pagefind.js" data-none="{none}">'
            f'<label class="sr-only" for="q">{lab}</label>'
            f'<input id="q" type="search" placeholder="{ph}" autocomplete="off">'
            f'<ul class="results" id="results" aria-live="polite"></ul></form>')


def docs_nav(lang, page, current):
    ar = lang == "ar"
    items = []
    for slug in DOC_ORDER:
        title = doc_title(slug, lang)
        cur = ' aria-current="page"' if slug == current else ""
        items.append(f'<li><a href="{page_href(lang, page, lang, "docs/" + slug)}"{cur}>{bidi(html.escape(title), lang)}</a></li>')
    return f'<nav class="docs-nav" aria-label="{"التوثيق" if ar else "Documentation"}"><ul role="list">{"".join(items)}</ul></nav>'


_titles = {}


def doc_title(slug, lang):
    key = (slug, lang)
    if key not in _titles:
        if slug == "formats":
            _titles[key] = "الصيغ" if lang == "ar" else "Formats"
        else:
            t, _ = md_to_html(read_guide(slug, lang), lang, "docs/" + slug)
            _titles[key] = t
    return _titles[key]


def breadcrumb_ld(lang, crumbs):
    return {"@context": "https://schema.org", "@type": "BreadcrumbList",
            "itemListElement": [{"@type": "ListItem", "position": i + 1, "name": n, "item": abs_url(lang, p)}
                                for i, (n, p) in enumerate(crumbs)]}


def docs_page(lang, slug):
    ar = lang == "ar"
    page = "docs/" + slug
    root = rel_root(lang, page)
    src = formats_md(lang) if slug == "formats" else read_guide(slug, lang)
    title, body = md_to_html(src, lang, page)
    p = lang_pack(lang)
    desc = re.sub(r"<[^>]+>", "", re.search(r"<p>(.*?)</p>", body, re.S).group(1)) if "<p>" in body else title
    desc = html.unescape(desc)[:160]
    full_title = f"{title} | {p['name']}"
    crumbs = [(p["name"], "index"), (p["nav"]["docs"], "docs/index"), (title, page)]
    ld = [breadcrumb_ld(lang, [(n, "index" if q == "docs/index" and False else q) for n, q in crumbs]),
          {"@context": "https://schema.org", "@type": "TechArticle", "headline": title, "inLanguage": lang,
           "url": abs_url(lang, page), "author": {"@type": "Organization", "name": "Kawaro"}}]
    edit = ""
    if slug != "formats":
        fn = f"{slug}.ar.md" if ar else f"{slug}.md"
        edit = (f'<p class="edit"><a href="{EDIT_BASE}{fn}">{"عدّل هذه الصفحة على GitHub" if ar else "Edit this page on GitHub"}</a> · '
                f'<a href="{REPO}/issues">{"بلّغ عن مشكلة" if ar else "Report an issue"}</a> · '
                f'<a href="../{slug}.md">Markdown</a></p>')
    else:
        edit = (f'<p class="edit"><a href="{REPO}/tree/main/actions">{"ملفات الإجراءات على GitHub" if ar else "Action files on GitHub"}</a> · '
                f'<a href="../{slug}.md">Markdown</a></p>')
    inner = (f'<div class="docs-layout"><aside>{search_box(lang, root)}{docs_nav(lang, page, slug)}</aside>'
             f'<article class="prose" data-pagefind-body data-pagefind-meta="title:{html.escape(title)}">{body}{edit}</article></div>')
    html_out = (head(lang, page, root, full_title, desc, ld) + header(lang, page, root)
                + sec(inner, cls="s-docs", edge=False, side="end") + footer(lang, page, root))
    return html_out, src


def docs_index(lang):
    ar = lang == "ar"
    page = "docs/index"
    root = rel_root(lang, page)
    p = lang_pack(lang)
    title = "التوثيق" if ar else "Documentation"
    desc = "ابدأ بالتثبيت، ثم الخلية، ثم الصيغ والإعدادات." if ar else "Start with install, then the hive, then formats and settings."
    tiles = []
    for n, slug in enumerate(DOC_ORDER, 1):
        tiles.append(f'<a class="doc-tile" href="{page_href(lang, page, lang, "docs/" + slug)}">'
                     f'<span class="n">{n}</span><span class="t">{bidi(html.escape(doc_title(slug, lang)), lang)}</span></a>')
    inner = (f'<h1>{title}</h1>{search_box(lang, root)}<div class="doc-tiles">{"".join(tiles)}</div>')
    ld = [breadcrumb_ld(lang, [(p["name"], "index"), (title, page)])]
    return (head(lang, page, root, f"{title} | {p['name']}", desc, ld) + header(lang, page, root)
            + sec(f'<div data-pagefind-body data-pagefind-meta="title:{title}">{inner}</div>', cls="s-docs-index", edge=False, side="start")
            + footer(lang, page, root))


def ar_more(n):
    if n == 1:
        return "وتغيير واحد آخر في"
    if n == 2:
        return "وتغييران آخران في"
    if 3 <= n <= 10:
        return f"و{n} تغييرات أخرى في"
    return f"و{n} تغييرا آخر في"


def release_blocks(lang):
    ar = lang == "ar"
    out = []
    for r in parse_changelog():
        unreleased = r["version"] == "Unreleased"
        ver = VERSION if unreleased else r["version"]
        pill = ("لم يصدر بعد" if ar else "Not released yet") if unreleased else ("مستقر" if ar else "Stable")
        cls = "pill pill-hot" if unreleased else "pill"
        # An English line inside the Arabic page is its own LTR run, so its punctuation keeps its place.
        items = "".join((f"<li>{bidi(html.escape(arx), lang)}</li>" if ar and arx else
                         f'<li dir="ltr" lang="en">{html.escape(en)}</li>' if ar else f"<li>{html.escape(en)}</li>")
                        for en, arx in r["items"][:6])
        more = len(r["items"]) - 6
        more_html = ""
        if more > 0:
            more_html = (f'<p class="fine">{ar_more(more) if ar else "and " + str(more) + " more in the"} '
                         f'<a href="{REPO}/blob/main/CHANGELOG.md">CHANGELOG</a></p>')
        tag_link = f"{RELEASES}/tag/v{ver}"
        date = f'<time>{r["date"]}</time>' if r["date"] else ""
        name = f' <span class="muted">{html.escape(r["name"])}</span>' if r["name"] and ar else ""
        out.append(f'<article class="release"><h2 id="v{ver.replace(".", "-")}"><span dir="ltr">{ver}</span>{name}</h2>'
                   f'<p class="meta"><span class="{cls}">{pill}</span> {date} '
                   f'<a href="{tag_link}">GitHub</a></p><ul>{items}</ul>{more_html}</article>')
    return "".join(out)


def releases_page(lang):
    ar = lang == "ar"
    page = "releases"
    root = rel_root(lang, page)
    p = lang_pack(lang)
    title = "الإصدارات" if ar else "Releases"
    desc = "كل إصدارات كوارو وما تغيّر فيها." if ar else "Every Kawaro release and what changed."
    latest = (f'<div class="latest"><span class="pill pill-hot"><bdi>{VERSION}</bdi></span>'
              f'<div class="actions">{dl_buttons(lang, "win")}{dl_buttons(lang, "linux")}</div>'
              f'<ul class="fine fine-links" role="list">'
              f'<li><a href="{verify_href(lang, page)}">{"تحقق من الملف" if ar else "Verify the file"}</a></li>'
              f'<li><a href="{page_href(lang, page, lang, "windows")}">{"ويندوز بتوقيع اختباري" if ar else "Windows is test-signed"}</a></li>'
              f'<li><a href="feed.xml">{"موجز الإصدارات" if ar else "Feed"}</a></li></ul></div>')
    inner = f'<h1>{title}</h1>{latest}<div class="releases" data-pagefind-body data-pagefind-meta="title:{title}">{release_blocks(lang)}</div>'
    ld = [breadcrumb_ld(lang, [(p["name"], "index"), (title, page)])]
    return (head(lang, page, root, f"{title} | {p['name']}", desc, ld) + header(lang, page, root)
            + sec(inner, cls="s-releases", edge=False, side="end") + footer(lang, page, root))


def newest_date():
    """The newest dated release in CHANGELOG.md. Pages never carry the build time."""
    return max([r["date"] for r in parse_changelog() if r["date"]] or ["1970-01-01"])


def feed_xml(lang):
    ar = lang == "ar"
    now = newest_date() + "T00:00:00Z"
    entries = []
    for r in parse_changelog():
        ver = VERSION if r["version"] == "Unreleased" else r["version"]
        upd = (r["date"] + "T00:00:00Z") if r["date"] else now
        summary = "; ".join(html.escape((arx or en) if ar else en) for en, arx in r["items"][:6])
        entries.append(f"<entry><title>Kawaro {ver}</title><id>{RELEASES}/tag/v{ver}</id>"
                       f'<link href="{RELEASES}/tag/v{ver}"/><updated>{upd}</updated><summary>{summary}</summary></entry>')
    return (f'<?xml version="1.0" encoding="utf-8"?>\n<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="{lang}">'
            f'<title>{"إصدارات كوارو" if ar else "Kawaro releases"}</title><id>{abs_url(lang, "releases")}</id>'
            f'<link href="{abs_url(lang, "releases")}"/><updated>{now}</updated>{"".join(entries)}</feed>\n')


ALL_PAGES = (["index", "windows", "releases", "docs/index"]
             + ["docs/" + s for s in DOC_ORDER])


def sitemap():
    today = newest_date()
    urls = []
    for page in ALL_PAGES:
        for lang in ("ar", "en"):
            other = "en" if lang == "ar" else "ar"
            urls.append(f"<url><loc>{abs_url(lang, page)}</loc><lastmod>{today}</lastmod>"
                        f'<xhtml:link rel="alternate" hreflang="{lang}" href="{abs_url(lang, page)}"/>'
                        f'<xhtml:link rel="alternate" hreflang="{other}" href="{abs_url(other, page)}"/></url>')
    return ('<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" '
            'xmlns:xhtml="http://www.w3.org/1999/xhtml">' + "".join(urls) + "</urlset>\n")


def llms_files(sources):
    """llms.txt (index) and llms-full.txt (all English docs), for agents."""
    idx = ["# Kawaro", "",
           "> A free, open source file converter for Linux and Windows. Select files, right click, pick a cell in a hexagonal hive; the result is saved next to the original. Everything runs on your machine: no uploads, no telemetry. Version " + VERSION + ".",
           "", "## Docs", ""]
    for slug in DOC_ORDER:
        idx.append(f"- [{doc_title(slug, 'en')}]({BASE}en/docs/{slug}.md)")
    idx += ["", "## Project", "",
            f"- [Download]({abs_url('en', 'index')}#download): Linux DEB and RPM, Windows MSI; SHA256SUMS steps in the install guide",
            f"- [Windows and SmartScreen]({abs_url('en', 'windows')}): the Windows build is test-signed with a self-signed certificate, so SmartScreen warns",
            f"- [Releases]({BASE}en/releases.md)",
            f"- [Source]({REPO}): GPL-3.0-or-later",
            "", "## Arabic", "",
            f"- [التوثيق بالعربية]({abs_url('ar', 'docs/index')})", ""]
    full = ["# Kawaro documentation (English)", ""]
    for slug in DOC_ORDER:
        full += [sources[("en", slug)].strip(), "", "---", ""]
    return "\n".join(idx), "\n".join(full)


ROBOTS = f"User-agent: *\nAllow: /\n\nSitemap: {BASE}sitemap.xml\n"


def build_extras():
    """Docs pages, releases, feeds, markdown twins, sitemap, robots, llms files."""
    sources = {}
    for lang in ("ar", "en"):
        base = HERE if lang == "ar" else HERE / "en"
        for slug in DOC_ORDER:
            out, src = docs_page(lang, slug)
            d = base / "docs" / slug
            d.mkdir(parents=True, exist_ok=True)
            (d / "index.html").write_text(out, encoding="utf-8")
            twin = re.sub(r"\(site:([a-z]+)(#[^)]*)?\)", lambda m: f"({abs_url(lang, m.group(1))}{m.group(2) or ''})", src)
            twin = twin.replace(".ar.md)", ".md)")
            twin = re.sub(r"\(diagram:([a-z]+)\)", lambda m: f"({BASE}diagrams/{m.group(1)}.{lang}.light.svg)", twin)
            (base / "docs" / f"{slug}.md").write_text(twin.strip() + "\n", encoding="utf-8")
            sources[(lang, slug)] = src
        d = base / "docs"
        d.mkdir(parents=True, exist_ok=True)
        (d / "index.html").write_text(docs_index(lang), encoding="utf-8")
        r = base / "releases"
        r.mkdir(parents=True, exist_ok=True)
        (r / "index.html").write_text(releases_page(lang), encoding="utf-8")
        (r / "feed.xml").write_text(feed_xml(lang), encoding="utf-8")
        (base / "releases.md").write_text(releases_md(lang), encoding="utf-8")
    (HERE / "404.html").write_text(not_found(), encoding="utf-8")
    (HERE / "sitemap.xml").write_text(sitemap(), encoding="utf-8")
    (HERE / "robots.txt").write_text(ROBOTS, encoding="utf-8")
    llms, full = llms_files(sources)
    (HERE / "llms.txt").write_text(llms, encoding="utf-8")
    (HERE / "llms-full.txt").write_text(full, encoding="utf-8")
    (HERE / ".nojekyll").write_text("", encoding="utf-8")
    import shutil
    (HERE / "diagrams").mkdir(exist_ok=True)
    for f in (ROOT / "docs" / "diagrams").glob("*.light.svg"):
        shutil.copy(f, HERE / "diagrams" / f.name)
    (HERE / "assets").mkdir(exist_ok=True)
    for f in (ROOT / "docs" / "assets").glob("support-sbarah-*.svg"):
        shutil.copy(f, HERE / "assets" / f.name)
    print("wrote docs, releases, feeds, sitemap, robots, llms files")


def not_found():
    """404.html: the page Cloudflare serves for any unknown path (wrangler not_found_handling 404-page).
    It is served at any depth, so every URL is made root absolute."""
    def absolutize(s):
        return re.sub(r'(href|src)="(?!https?:|#|/|mailto:)([^"]*)"',
                      lambda m: f'{m.group(1)}="/{m.group(2).lstrip("./") if m.group(2) not in ("./", ".") else ""}"', s)
    p = AR
    out = head("ar", "index", "/", title="الصفحة غير موجودة | كوارو",
               desc="الصفحة التي طلبتها غير موجودة. Page not found.")
    out = re.sub(r'<link rel="(canonical|alternate)"[^>]*>\n', "", out)
    out = out.replace('<meta name="color-scheme"', '<meta name="robots" content="noindex">\n<meta name="color-scheme"', 1)
    out = out.replace('<meta property="og:url" content="https://kawaro.sbarah.com/">\n', "")
    body = ('<section class="page-head sec"><div class="wrap">'
            '<h1>الصفحة غير موجودة</h1>'
            '<p class="lead">لم نجد هذه الصفحة. ربما تغير الرابط.</p>'
            f'<p>{hex_button("/", "الصفحة الرئيسية", "primary")} '
            f'{hex_button("/en/", "Home (English)", "plain", attrs=' lang="en"')}</p>'
            '<p lang="en" dir="ltr">Page not found. The link may have changed.</p>'
            '</div></section>\n')
    return (out + absolutize(header("ar", "index", "")) + body + absolutize(footer("ar", "index", "")))


def check_release_tag():
    """With RELEASE_PUBLISHED=1 the asset links are real only if the tag exists."""
    out = subprocess.run(["git", "tag", "-l", TAG], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    if not out:
        print(f"RELEASE_PUBLISHED=1 but the tag {TAG} does not exist: create and push it first", file=sys.stderr)
        sys.exit(1)


def main():
    if PUBLISHED:
        check_release_tag()
    for lang in ("ar", "en"):
        for page in PAGES:
            parts = (["en"] if lang == "en" else []) + ([] if page == "index" else [page])
            out = HERE.joinpath(*parts, "index.html")
            out.parent.mkdir(parents=True, exist_ok=True)
            out.write_text(render(lang, page), encoding="utf-8")
            print("wrote", out.relative_to(HERE.parent))
    write_download_stubs()
    build_extras()


def write_download_stubs():
    """/download/ and /en/download/ are gone as pages; old links land on the home
    page section. Plain HTML redirect: no script, so the strict CSP is untouched."""
    for lang, parts, home_path, text in (("ar", ["download"], "/", "انتقل إلى قسم التنزيل"),
                                         ("en", ["en", "download"], "/en/", "Go to the download section")):
        target = f"{home_path}#download"
        out = HERE.joinpath(*parts, "index.html")
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(
            f'<!doctype html>\n<html lang="{lang}" dir="{"rtl" if lang == "ar" else "ltr"}">\n<head>\n'
            f'<meta charset="utf-8">\n<title>{"كوارو" if lang == "ar" else "Kawaro"}</title>\n'
            f'<meta name="viewport" content="width=device-width, initial-scale=1">\n'
            f'<meta name="color-scheme" content="light">\n<meta name="robots" content="noindex">\n'
            f'<meta http-equiv="refresh" content="0; url={target}">\n'
            f'<link rel="canonical" href="{BASE.rstrip("/")}{target}">\n</head>\n'
            f'<body><p><a href="{target}">{text}</a></p></body>\n</html>\n', encoding="utf-8")
        print("wrote", out.relative_to(HERE.parent))


if __name__ == "__main__":
    main()
