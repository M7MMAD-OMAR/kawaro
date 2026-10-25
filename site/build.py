#!/usr/bin/env python3
"""Build the SbarHive site: every page in Arabic (site root) and English (site/en).

Standard library only. Colours, lengths and durations never appear here: the
markup uses classes and custom properties from tokens.css. Geometry numbers for
the hive come from design/tokens.json, the same source as the app.

Run: python3 site/build.py
Pages: index, download, windows, privacy. Arabic and English are two originals
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
BASE = "https://hive.sbarah.com/"
REPO = "https://github.com/M7MMAD-OMAR/sbarhive"
RELEASES = REPO + "/releases"
VERSION = "0.4.0-beta.1"
# S-061: download links point at release assets only once the release exists.
# Set RELEASE_PUBLISHED=1 after the tag and its assets are on GitHub.
PUBLISHED = os.environ.get("RELEASE_PUBLISHED", "0") == "1"
DIRECTIONS = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)]

PAGES = ["index", "download", "windows", "privacy"]

# The tab icon is inlined: a browser fetches a linked icon after the page has
# loaded, and the try-it demo promises no network request after load.
FAVICON = "data:image/svg+xml," + urllib.parse.quote(
    (HERE / "favicon.svg").read_text(encoding="utf-8").strip(), safe="/:=,;'()")


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

def static_hive(lang):
    """First paint and no-JS fallback: the image hive, same geometry as app.js."""
    rtl = lang == "ar"
    kind = kinds(lang)[0]
    out = []
    for i, (q, r) in enumerate(spiral(RINGS)):
        cx, cy = cell_xy(q, r, rtl)
        if i == 0:
            out.append(f'<g class="cell cell-center" style="--i:0"><path d="{hex_path(cx, cy)}"/>'
                       f'<text x="{cx:.1f}" y="{cy:.1f}">{html.escape(kind["label"])}</text></g>')
            continue
        idx = i - 1
        if idx < len(kind["cells"]):
            c = kind["cells"][idx]
            cls = "cell cell-active" if idx == 0 else "cell"
            out.append(f'<g class="{cls}" style="--i:{i}"><path d="{hex_path(cx, cy)}"/>'
                       f'<text x="{cx:.1f}" y="{cy:.1f}">{html.escape(c["c"])}</text></g>')
        else:
            out.append(f'<g class="cell cell-empty" style="--i:{i}"><path d="{hex_path(cx, cy)}"/></g>')
    return "\n".join(out)


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


def privacy_svg(lang):
    ar = lang == "ar"
    rtl = ar
    w, h = 720, 290

    def mx(x):
        return w - x if rtl else x

    t_pc = "جهازك" if ar else "Your machine"
    t_net = "الإنترنت" if ar else "The internet"
    t_none = "لا يخرج أي ملف" if ar else "No file leaves"
    cx_pc, cx_net = mx(170), mx(570)
    out = [f'<svg class="privacy-map" viewBox="0 0 {w} {h}" role="img" aria-label="{html.escape(t_pc)}: {html.escape(t_none)}">']
    out.append(f'<path class="zone" d="{hex_path(cx_pc, 125, 118)}"/>')
    out.append(f'<text class="zone-label" x="{cx_pc}" y="272">{t_pc}</text>')
    out.append(f'<rect class="doc" x="{cx_pc - 70}" y="100" width="28" height="38"/>')
    out.append(f'<path class="hop" d="{hex_path(cx_pc, 120, 20)}"/>')
    out.append(f'<rect class="doc doc-new" x="{cx_pc + 42}" y="100" width="28" height="38"/>')
    out.append(f'<line class="wire-s" x1="{cx_pc - 40}" y1="119" x2="{cx_pc - 22}" y2="119"/>')
    out.append(f'<line class="wire-s" x1="{cx_pc + 22}" y1="119" x2="{cx_pc + 40}" y2="119"/>')
    # internet: dashed zone
    out.append(f'<path class="zone zone-off" d="{hex_path(cx_net, 125, 80)}"/>')
    out.append(f'<text class="zone-label" x="{cx_net}" y="272">{t_net}</text>')
    # blocked link
    a, b = sorted((cx_pc + (118 * 0.87 if not rtl else -118 * 0.87), cx_net + (-80 * 0.87 if not rtl else 80 * 0.87)))
    mid = (a + b) / 2
    out.append(f'<line class="wire-off" x1="{a:.0f}" y1="125" x2="{b:.0f}" y2="125"/>')
    out.append(f'<g class="cross"><line x1="{mid - 9:.0f}" y1="116" x2="{mid + 9:.0f}" y2="134"/>'
               f'<line x1="{mid - 9:.0f}" y1="134" x2="{mid + 9:.0f}" y2="116"/></g>')
    out.append(f'<text class="zone-label strong" x="{mid:.0f}" y="96">{t_none}</text>')
    out.append("</svg>")
    return "".join(out)


# ---------------------------------------------------------------------------
# Language packs. Two originals with the same structure. Short on purpose:
# big headlines, few words, the drawings explain the rest.
# ---------------------------------------------------------------------------

TAG = "v" + VERSION
PKGV = VERSION.replace("-", "%7E")
DL = f"{REPO}/releases/download/{TAG}/"
FILES = {
    "msi": DL + f"sbarhive-{VERSION}-windows-x64.msi",
    "zip": DL + f"sbarhive-{VERSION}-windows-x64.zip",
    "deb": DL + f"sbarhive_{PKGV}_amd64.deb",
    "rpm": DL + f"sbarhive-{PKGV}-1.x86_64.rpm",
}
SUPPORT = "https://sbarah.com/support"

AR = {
    "lang": "ar", "dir": "rtl", "name": "صبار هايف", "locale": "ar_AR", "alt": "en_US",
    "nav_skip": "انتقل إلى المحتوى",
    "nav": {"index": "الرئيسية", "docs": "التوثيق", "download": "التنزيل", "releases": "الإصدارات", "windows": "ويندوز", "privacy": "الخصوصية"},
    "nav_label": "التنقل الرئيسي",
    "other_lang": "English",
    "beta": "بيتا",
    "support": "ادعم صبارة",
    "titles": {
        "index": "صبار هايف: حوّل أي ملف بنقرة يمين",
        "download": "تنزيل صبار هايف " + VERSION + " للينكس وويندوز",
        "windows": "صبار هايف على ويندوز: تحذير SmartScreen والتحقق من الملف",
        "privacy": "الخصوصية في صبار هايف: ملفاتك تبقى عندك",
    },
    "descs": {
        "index": "محوّل ملفات مجاني ومفتوح المصدر للينكس وويندوز. حدّد الملف، اختر خلية، والناتج بجانب الأصل. ملفاتك تبقى على جهازك.",
        "download": "أول إصدار بيتا من صبار هايف للينكس وويندوز، مع خطوات التحقق بـ SHA256SUMS.",
        "windows": "نسخة ويندوز غير موقّعة بعد. شرح تحذير SmartScreen وطريقة التحقق من ملفك قبل تشغيله.",
        "privacy": "لا تتبع ولا حسابات ولا رفع للملفات. الاتصال الوحيد هو فحص التحديثات الموقّعة.",
    },
    "og_alt": "شعار صبار هايف بجانب الخلية السداسية، وعلى خلاياها صيغ تحويل الصور",
    "footer_license": "برنامج حر برخصة GPLv3.",
    "footer_beta": "بيتا <bdi dir=\"ltr\">" + VERSION + "</bdi>",
    "releases": "الإصدارات",
}

EN = {
    "lang": "en", "dir": "ltr", "name": "SbarHive", "locale": "en_US", "alt": "ar_AR",
    "nav_skip": "Skip to content",
    "nav": {"index": "Home", "docs": "Docs", "download": "Download", "releases": "Releases", "windows": "Windows", "privacy": "Privacy"},
    "nav_label": "Main navigation",
    "other_lang": "العربية",
    "beta": "beta",
    "support": "Support Sbarah",
    "titles": {
        "index": "SbarHive: convert any file with a right click",
        "download": "Download SbarHive " + VERSION + " for Linux and Windows",
        "windows": "SbarHive on Windows: the SmartScreen warning and checking your download",
        "privacy": "SbarHive privacy: your files stay with you",
    },
    "descs": {
        "index": "A free, open source file converter for Linux and Windows. Select the file, pick a cell, and the result lands next to the original. Your files stay on your machine.",
        "download": "The first SbarHive beta for Linux and Windows, with the SHA256SUMS steps to verify it.",
        "windows": "The Windows build is not signed yet. What the SmartScreen warning means and how to verify your file before running it.",
        "privacy": "No tracking, no accounts, no uploads. The only network use is the signed update check.",
    },
    "og_alt": "The SbarHive logo next to a hexagonal hive with image formats",
    "footer_license": "Free software under the GPLv3.",
    "footer_beta": "Beta " + VERSION,
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
            + ["css/pages.css"])
CSS_HOME = ["css/art.css", "css/hero.css", "css/scene.css", "css/sim.css"]
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
        "name": "SbarHive", "alternateName": "صبار هايف",
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


ART = {
    "hero": (1024, 768, "خلايا سداسية مسطحة متجاورة وفي وسطها خلية كهرمانية", "Flat hexagon cells joined as a honeycomb with one amber cell in the middle"),
    "privacy": (1024, 768, "شاشة حاسوب فيها خلايا وملفات وقفل، لا يخرج منها شيء", "A computer screen with hive cells, files and a padlock, nothing leaves it"),
    "formats": (1024, 368, "خمس خلايا لأنواع الملفات: صور وفيديو وصوت ومستندات وأرشيف", "Five hive cells for file types: images, video, audio, documents and archives"),
}


def art(lang, name, lazy=True):
    """Generated flat art (AC9): WebP in assets/, with width and height and a language matched alt.
    Below the fold images are small (under 12 KB) and load eagerly: the S-061 zero request after load test
    forbids a lazy fetch that fires while the demo scrolls."""
    w, h, alt_ar, alt_en = ART[name]
    alt = alt_ar if lang == "ar" else alt_en
    load = 'decoding="async"' if lazy else 'fetchpriority="high"'
    return (f'<img class="art art-{name}" src="{rel_root(lang, "index")}assets/art-{name}.webp" '
            f'width="{w}" height="{h}" alt="{alt}" {load}>')


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
    for key in ("docs", "download", "releases"):
        cur = ' aria-current="page"' if (key == page or page.startswith(key + "/")) else ""
        links.append(f'<a class="nav-link" href="{page_href(lang, page, lang, key)}"{cur}>{p["nav"][key]}</a>')
    sw = (f'<a class="nav-link lang-switch" href="{page_href(lang, page, other, page)}" '
          f'hreflang="{other}" lang="{other}">{p["other_lang"]}</a>')
    return f'''<header class="site-header">
  <div class="wrap">
    <a class="brand" href="{page_href(lang, page, lang, "index")}">
      <img src="{root}favicon.svg" alt="" width="48" height="48">
      <span>{p["name"]}</span><span class="badge">{p["beta"]}</span>
    </a>
    <nav class="site-nav" aria-label="{p["nav_label"]}">
      {"".join(links)}
      {support_link(lang)}
      {sw}
    </nav>
  </div>
</header>
<main id="main">
'''


def footer(lang, page, root):
    p = lang_pack(lang)
    other = "en" if lang == "ar" else "ar"
    product = [
        f'<a href="{REPO}">GitHub</a>',
        f'<a href="{page_href(lang, page, lang, "windows")}">{p["nav"]["windows"]}</a>',
        f'<a href="{page_href(lang, page, lang, "privacy")}">{p["nav"]["privacy"]}</a>',
        f'<a href="{RELEASES}">GitHub {p["releases"]}</a>',
        support_a("", p["support"]),
    ]
    items = "".join(f"<li>{x}</li>" for x in product)
    lang_sw = (f'<a href="{page_href(lang, page, other, page)}" hreflang="{other}" lang="{other}">'
               f'{p["other_lang"]}</a>')
    scripts = f'<script src="{root}app.js" defer></script>'
    if page == "index":
        scripts += f'\n<script src="{root}js/scene.js" defer></script>\n<script src="{root}js/sim.js" defer></script>'
    return f'''</main>
<footer class="site-footer">
  <div class="wrap">
    <p>{p["footer_license"]} <span class="muted">{p["footer_beta"]}</span></p>
    <ul class="footer-product" role="list">{items}</ul>
    <ul class="footer-lang" role="list"><li>{lang_sw}</li></ul>
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


SOON = {"ar": "قريبا", "en": "Coming soon"}
LATIN = re.compile(r"\b(?:MSI|DEB|RPM|ZIP|EXE)\b|\d+\.\d+\.\d+(?:-[A-Za-z]+\.\d+)?")


def bidi(text, lang):
    """Arabic text: isolate Latin file codes and versions so they keep their order."""
    if lang != "ar":
        return text
    parts = re.split(r"(<[^>]*>)", text)
    return "".join(x if x.startswith("<") else LATIN.sub(lambda m: f"<bdi>{m.group(0)}</bdi>", x) for x in parts)


def dl_link(lang, kind, label, button="plain", size="md"):
    """A download button. Until the release exists it opens the releases page."""
    href = FILES[kind] if PUBLISHED else RELEASES
    soon = "" if PUBLISHED else f' <span class="soon">{SOON[lang]}</span>'
    return hex_button(href, f"{label}{soon}", button, size, f' data-dl="{kind}"')


def dl_buttons(lang, which):
    """Direct download buttons. Names follow scripts/release/dry-run.sh and make-msi.sh."""
    ar = lang == "ar"
    word = "تنزيل" if ar else "Download"
    if which == "win":
        return (dl_link(lang, "msi", bidi(f"{word} MSI", lang), "primary", "lg")
                + dl_link(lang, "zip", bidi("ZIP محمول" if ar else "Portable ZIP", lang)))
    return (dl_link(lang, "deb", bidi(f"{word} DEB", lang), "primary", "lg")
            + dl_link(lang, "rpm", bidi(f"{word} RPM", lang), "primary", "lg"))


def download_slab(lang, which):
    """One operating system: a cut corner slab with its icon, its note and its buttons."""
    ar = lang == "ar"
    if which == "win":
        title, sub, icon, card = ("ويندوز" if ar else "Windows"), ("10 و11، 64 بت" if ar else "10 and 11, 64 bit"), WIN_ICO, "win"
    else:
        title, sub, icon, card = ("لينكس" if ar else "Linux"), ("فيدورا وأوبونتو وديبيان، 64 بت" if ar else "Fedora, Ubuntu, Debian, 64 bit"), LIN_ICO, "linux"
    return (f'<div class="download-slab" data-card="{card}">{icon}<h3>{title}</h3>'
            f'<p class="download-slab-sub">{sub}</p>'
            f'<div class="actions">{dl_buttons(lang, which)}</div></div>')


def dl_cards(lang):
    return f'<div class="dl-grid">{download_slab(lang, "win")}{download_slab(lang, "linux")}</div>'


def pills(lang, verify_href, win_href):
    ar = lang == "ar"
    return (f'<ul class="facts" role="list">'
            f'<li><span class="pill pill-hot"><bdi>{VERSION}</bdi></span></li>'
            f'<li><a href="{win_href}">{"ويندوز غير موقّع" if ar else "Windows is unsigned"}</a></li>'
            f'<li><a href="{verify_href}">{"تحقق من الملف" if ar else "Verify the file"}</a></li>'
            f'</ul>')


def stage(lang):
    """The hero hive: a quiet picture of the idea. The try-it demo is the #sim section."""
    return f'''<div class="stage" aria-hidden="true">
  <svg class="hive" viewBox="{VIEW}" focusable="false">
{static_hive(lang)}
  </svg>
</div>'''


# ---------------------------------------------------------------------------
# Home
# ---------------------------------------------------------------------------

T_HOME = {
    "ar": {
        "h1": "حوّل أي ملف بنقرة <span class=\"hl\">يمين</span>",
        "sub": "حدّد الملف. اختر خلية. الناتج بجانب الأصل.",
        "dl": "تنزيل",
        "all": "كل التنزيلات",
        "win_link": "ويندوز (غير موقّع)",
        "note": "بيتا للينكس و{win}",
        "note_soon": "البيتا قريبا للينكس و{win}",
        "scene_h2": "شاهدها تعمل",
        "scene_steps": ["هذا مجلدك، وفيه ملفاتك", "حدّد الملف الذي تريده", "تنفتح الخلية حلقة بعد حلقة",
                        "اختر الخلية المناسبة", "الناتج بجانب الأصل، والأصل لم يتغير"],
        "scene_folder": "الصور",
        "scene_kept": "الأصل",
        "scene_new": "جديد",
        "try": "جرّبها بنفسك",
        "how": "ثلاث خطوات",
        "steps": ["حدّد", "اختر", "خذ الناتج"],
        "step_text": ["حدّد ملفا أو أكثر في مدير الملفات.",
                      "انقر بزر الفأرة الأيمن، فتعرض الخلية ما يناسب الملف فقط.",
                      "يظهر الملف الجديد بجانب الأصل."],
        "privacy": "ملفاتك تبقى عندك",
        "ticks": ["بلا رفع", "بلا تتبع", "الأصل لا يتغير"],
        "formats": "صور وفيديو وصوت ومستندات",
        "tiles": ["صور", "فيديو", "صوت", "مستندات", "PDF"],
        "get": "نزّل صبار هايف",
        "open": "مجاني ومفتوح",
        "open_p": "مفتوح المصدر برخصة GPLv3. اقرأه وعدّله وشاركه.",
        "gh": "الشيفرة على GitHub",
        "support_h2": "ادعم العمل",
        "support_p": "صبار هايف مجاني. إن وفّر عليك وقتا فيمكنك دعم صبارة.",
    },
    "en": {
        "h1": "Convert any file with a <span class=\"hl\">right</span> click",
        "sub": "Select the file. Pick a cell. The result lands next to the original.",
        "dl": "Download",
        "all": "All downloads",
        "win_link": "Windows (unsigned)",
        "note": "Beta for Linux and {win}",
        "note_soon": "Beta coming soon for Linux and {win}",
        "scene_h2": "Watch it work",
        "scene_steps": ["This is your folder, full of files", "Select the file you want", "The hive opens, ring by ring",
                        "Pick the cell that fits", "The result lands next to the untouched original"],
        "scene_folder": "Pictures",
        "scene_kept": "Original",
        "scene_new": "New",
        "try": "Try it yourself",
        "how": "Three steps",
        "steps": ["Select", "Pick", "Take it"],
        "step_text": ["Select one file or many in your file manager.",
                      "Right click. The hive shows only what fits the file.",
                      "The new file lands next to the original."],
        "privacy": "Your files stay with you",
        "ticks": ["No uploads", "No tracking", "Original untouched"],
        "formats": "Images, video, audio, documents",
        "tiles": ["Images", "Video", "Audio", "Documents", "PDF"],
        "get": "Get SbarHive",
        "open": "Free and open",
        "open_p": "Open source under GPLv3. Read it, change it, share it.",
        "gh": "Source on GitHub",
        "support_h2": "Support the work",
        "support_p": "SbarHive is free. If it saves you time, you can support Sbarah.",
    },
}

T_SIM = {
    "ar": {
        "h2": "جرّبها هنا",
        "intro": "اسحب ملفا إلى المجلد، أو اختر ملفا واضغط Enter. لا يُرفع شيء: هذا عرض توضيحي فقط.",
        "samples": "ملفات تجريبية",
        "zone": "المجلد. أفلت ملفا تجريبيا هنا.",
        "folder": "ملفات المجلد",
        "hint": "أفلت ملفا هنا",
        "idle": "تظهر الخلية هنا",
        "nojs": "صورة ثابتة للخلية، فعّل JavaScript لتجربتها.",
        "sample": "{name}، {kind}. اضغط Enter لفتح الخلية.",
        "hive": "الخلية: إجراءات تناسب الملف",
    },
    "en": {
        "h2": "Try it here",
        "intro": "Drag a file onto the folder, or pick one and press Enter. Nothing is uploaded: this is only a demo.",
        "samples": "Sample files",
        "zone": "Folder. Drop a sample file here.",
        "folder": "Folder files",
        "hint": "Drop a file here",
        "idle": "The hive opens here",
        "nojs": "A still picture of the hive, turn on JavaScript to try it.",
        "sample": "{name}, {kind}. Press Enter to open the hive.",
        "hive": "The hive: actions that fit the file",
    },
}

DOC_ICO = ('<svg class="ico" viewBox="0 0 24 24" aria-hidden="true" focusable="false">'
           '<path d="M6 3h8l4 4v14H6zM14 3v4h4"/></svg>')


def sim_section(lang):
    """The try-it demo (S-061 AC7, AC8). Behaviour lives in js/sim.js between the sim markers."""
    t = T_SIM[lang]
    items = []
    for k in kinds(lang):
        names = ", ".join(k["files"])
        label = html.escape(t["sample"].format(name=names, kind=k["label"]))
        items.append(f'<li><button type="button" class="sim-file" draggable="true" data-kind="{k["key"]}" '
                     f'data-name="{names}" aria-label="{label}">{DOC_ICO}<span class="name">{names}</span></button></li>')
    pic = (f'<svg class="sim-pic" viewBox="0 0 160 140" aria-hidden="true" focusable="false">'
           f'{mini_cluster(80, 70, 22, lang == "ar", {0: "mc mc-hot"})}</svg>')
    inner = f'''
{section_head("sim-title", t["h2"], t["intro"])}
<div class="sim-board">
  <div class="sim-left">
    <ul class="sim-samples" role="list" aria-label="{t["samples"]}">{"".join(items)}</ul>
    <div class="sim-zone" id="sim-zone" tabindex="0" role="group" aria-label="{t["zone"]}">
      <p class="sim-hint">{t["hint"]}</p>
      <ul class="sim-files" id="sim-files" aria-label="{t["folder"]}"></ul>
    </div>
  </div>
  <div class="sim-stage">
    <p class="sim-idle">{t["idle"]}</p>
    <div class="sim-hive" id="sim-hive" role="group" aria-label="{t["hive"]}" hidden></div>
  </div>
</div>
<div class="sim-static">{pic}<p class="sim-nojs">{t["nojs"]}</p></div>
<p class="sr-only" id="stage-live" role="status" aria-live="polite"></p>
'''
    return sec(inner, cls="s-sim", ident="sim", labelledby="sim-title")


def support_section(lang):
    """The owner's badge in the Sbarah theme: the only link in the section."""
    t, p, r = T_HOME[lang], lang_pack(lang), rel_root(lang, "index")
    pic = f'<img src="{r}assets/support-sbarah-light.svg" alt="{p["support"]}" width="520" height="140">'
    inner = f'''
{section_head("support-title", t["support_h2"], t["support_p"])}
{support_a("support-badge", pic)}
'''
    return sec(inner, cls="s-support", ident="support", labelledby="support-title")


def scene_cell(q, r, ring, label, picked=False):
    pick = " data-pick" if picked else ""
    on = f'<span class="scene-hit-label">{html.escape(label)}</span>' if picked else ""
    return (f'<div class="scene-cell" data-ring="{ring}"{pick} style="--q:{q};--r:{r};--k:{ring}">'
            f'<span class="scene-cell-hit">{on}</span><span class="scene-cell-label">{html.escape(label)}</span></div>')


def scene_section(lang):
    """AC6: one pinned scene. Scroll progress drives it (js/scene.js writes --p and data-state).
    Ghost cells draw the empty hive from the first frame; the real cells fly in over them."""
    t = T_HOME[lang]
    image = kinds(lang)[0]
    slots = spiral(2)
    ghosts = "".join(f'<i class="scene-ghost" data-ring="{1 if 0 < i <= 6 else 2 if i else 0}" '
                     f'style="--q:{q};--r:{r}"></i>' for i, (q, r) in enumerate(slots))
    cells = [scene_cell(0, 0, 0, image["label"])]
    for i, c in enumerate(image["cells"][:12], 1):
        q, r = slots[i]
        cells.append(scene_cell(q, r, 1 if i <= 6 else 2, c["c"], picked=(i == 1)))

    def tile(cls, ext, name, tag, extra=""):
        return (f'<li class="{cls}">{extra}<span class="scene-ext">{ext}</span>'
                f'<span class="scene-name">{name}</span><span class="scene-tag">{tag}</span></li>')

    files = "".join([
        tile("scene-file scene-original", "PNG", "photo.png", t["scene_kept"], '<i class="scene-sel"></i><i class="scene-cursor"></i>'),
        tile("scene-result", "WEBP", "photo.webp", t["scene_new"], '<i class="scene-halo"></i>'),
        tile("scene-file", "MOV", "clip.mov", ""),
        tile("scene-file scene-more", "DOCX", "report.docx", ""),
    ])
    steps = "".join(f'<li><span class="num" aria-hidden="true">{i}</span><span class="scene-step-text">{s}</span></li>'
                    for i, s in enumerate(t["scene_steps"], 1))
    pips = "".join(f'<i class="scene-pip"><s></s><b>{i}</b></i>' for i in range(1, 6))
    inner = f'''<div class="scene-stage">
  <div class="scene-top">
    {section_head("scene-title", t["scene_h2"])}
    <div class="scene-progress" aria-hidden="true">{pips}</div>
  </div>
  <div class="scene-visual" aria-hidden="true">
    <div class="scene-folder">
      <div class="scene-bar"><span class="scene-bar-name">{t["scene_folder"]}</span></div>
      <ul class="scene-files" role="list">{files}</ul>
    </div>
    <div class="scene-hive">{ghosts}{"".join(cells)}</div>
  </div>
  <ol class="scene-steps">{steps}</ol>
</div>'''
    return f'<section id="scene" class="scene" aria-labelledby="scene-title">\n{inner}\n</section>\n'


def try_bridge(lang):
    """The keyboard operable version of the demo: a control after the scene that jumps to #sim."""
    t = T_HOME[lang]
    btn = hex_button("#sim", t["try"], "primary", "lg", extra="try-it")
    return f'<div class="try-bridge"><div class="wrap">{btn}</div></div>\n'


def home(lang):
    t = T_HOME[lang]
    win_href = page_href(lang, "index", lang, "windows")
    dl_href = page_href(lang, "index", lang, "download")
    win_a = f'<a href="{win_href}">{t["win_link"]}</a>'
    note = (t["note"] if PUBLISHED else t["note_soon"]).format(win=win_a)
    rel = ' data-rel="1"' if PUBLISHED else ""
    btn = hex_button(dl_href, t["dl"], "primary", "lg", f' data-os-default="linux"{rel}')
    hero = f'''<section class="hero sec" aria-labelledby="hero-title">
<div class="wrap hero-grid">
<div class="hero-copy">
<h1 id="hero-title">{t["h1"]}</h1>
<p class="lead">{t["sub"]}</p>
<div class="actions">{btn}<a class="link-all" href="{dl_href}">{t["all"]}</a></div>
<p class="hero-note">{note}</p>
</div>
<div class="stage">{art(lang, "hero", lazy=False)}</div>
</div>
</section>
'''
    cells = "".join(step_cell(i, t["steps"][i - 1], t["step_text"][i - 1]) for i in (1, 2, 3))
    how = sec(f'''
{section_head("how-title", t["how"])}
<ol class="step-row" role="list">{cells}</ol>
''', cls="s-how alt", ident="how", labelledby="how-title")
    plates = "".join(hex_plate(x, "li", "accent" if i == 0 else "") for i, x in enumerate(t["ticks"]))
    privacy = sec(f'''
{section_head("privacy-title", t["privacy"])}
<div class="split">
  <ul class="plate-list" role="list">{plates}</ul>
  <div class="privacy-art">{art(lang, "privacy")}</div>
</div>
''', cls="s-privacy", ident="privacy", labelledby="privacy-title")
    fam = [
        (t["tiles"][0], "PNG JPG WEBP AVIF"),
        (t["tiles"][1], "MP4 WEBM MKV GIF"),
        (t["tiles"][2], "MP3 OGG FLAC WAV"),
        (t["tiles"][3], "DOCX ODT XLSX"),
        (t["tiles"][4], "PDF"),
    ]
    formats = sec(f'''
{section_head("formats-title", t["formats"])}
{art(lang, "formats")}
<ul class="format-grid" role="list">{"".join(format_cell(a, b) for a, b in fam)}</ul>
''', cls="s-formats alt", ident="formats", labelledby="formats-title")
    get = sec(f'''
{section_head("get-title", t["get"])}
{dl_cards(lang)}
{pills(lang, dl_href + "#verify", win_href)}
''', cls="s-get", ident="download", labelledby="get-title")
    open_ = sec(f'''
{section_head("open-title", t["open"], t["open_p"])}
<div class="actions">{hex_button(REPO, t["gh"], "plain", "lg")}</div>
''', cls="s-open", ident="open", labelledby="open-title")
    return (hero + scene_section(lang) + try_bridge(lang) + sim_section(lang) + how + privacy + formats + get
            + open_ + support_section(lang))



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


def download(lang):
    ar = lang == "ar"
    win_href = page_href(lang, "download", lang, "windows")
    if ar:
        h = page_head("نزّل صبار هايف", f"أول إصدار بيتا <code>{VERSION}</code>" + ("" if PUBLISHED else "، قريبا"))
        top = sec(f'{dl_cards(lang)}<ul class="facts" role="list"><li><span class="pill pill-hot"><bdi>{VERSION}</bdi></span></li><li><a href="{win_href}">ويندوز غير موقّع</a></li><li><a href="{RELEASES}">كل الإصدارات</a></li></ul>',
                  cls="s-dl", ident="choose", edge=False)
        ver = sec(f'''<h2 id="v-title">تحقق من الملف</h2>
<p class="section-intro">نزّل <code>SHA256SUMS</code> في المجلد نفسه الذي فيه الملفات، ويجب أن يظهر OK أمام كل ملف.</p>
<div class="split"><div><h3>لينكس</h3>{code("sha256sum -c SHA256SUMS --ignore-missing")}</div>
<div><h3>ويندوز</h3>{code(f"certutil -hashfile sbarhive-{VERSION}-windows-x64.msi SHA256")}</div></div>
<p class="section-intro">وإن اختلفت القيمة فلا تشغّل الملف.</p>''', cls="alt s-verify", ident="verify", labelledby="v-title", side="start")
        src = sec(f'''<h2 id="b-title">البناء من الشيفرة على لينكس</h2>
{code(f"git clone {REPO}" + chr(10) + "cd sbarhive" + chr(10) + "./scripts/install.sh")}
<p class="section-intro">يحتاج Rust 1.85 أو أحدث. للإزالة: <code>./scripts/uninstall.sh</code></p>''', cls="s-build", ident="build", labelledby="b-title", side="end")
    else:
        h = page_head("Download SbarHive", f"First beta, <code>{VERSION}</code>" + ("" if PUBLISHED else ", coming soon"))
        top = sec(f'{dl_cards(lang)}<ul class="facts" role="list"><li><span class="pill pill-hot"><bdi>{VERSION}</bdi></span></li><li><a href="{win_href}">Windows is unsigned</a></li><li><a href="{RELEASES}">All releases</a></li></ul>',
                  cls="s-dl", ident="choose", edge=False)
        ver = sec(f'''<h2 id="v-title">Verify the file</h2>
<p class="section-intro">Put <code>SHA256SUMS</code> in the packages folder; every file must say OK.</p>
<div class="split"><div><h3>Linux</h3>{code("sha256sum -c SHA256SUMS --ignore-missing")}</div>
<div><h3>Windows</h3>{code(f"certutil -hashfile sbarhive-{VERSION}-windows-x64.msi SHA256")}</div></div>
<p class="section-intro">If a value differs, do not run the file.</p>''', cls="alt s-verify", ident="verify", labelledby="v-title", side="start")
        src = sec(f'''<h2 id="b-title">From source on Linux</h2>
{code(f"git clone {REPO}" + chr(10) + "cd sbarhive" + chr(10) + "./scripts/install.sh")}
<p class="section-intro">Needs Rust 1.85 or newer. To remove: <code>./scripts/uninstall.sh</code></p>''', cls="s-build", ident="build", labelledby="b-title", side="end")
    return h + top + ver + src


def windows(lang):
    ar = lang == "ar"
    dl = page_href(lang, "windows", lang, "download")
    if ar:
        h = page_head("صبار هايف على ويندوز", "غير موقّع بعد، فيظهر تحذير SmartScreen")
        a = sec(f'''<h2 id="w1">ثلاث خطوات</h2>
<ol class="step-row" role="list">{step_cell(1, "تحقق من الملف", f'<a href="{dl}#verify">SHA256SUMS</a>')}{step_cell(2, "More info")}{step_cell(3, "Run anyway")}</ol>
<p class="section-intro">ليست لدينا شهادة توقيع بعد، ولا نعد بمتجر أو بموعد.</p>''', cls="s-w1", ident="why", labelledby="w1", edge=False, side="start")
        b = sec(f'''<h2 id="w2">لم يكتمل بعد</h2>
<div class="grid grid-3">{card("لم يُجرَّب على جهاز ويندوز حقيقي", "", ("tag-warn", "تنبيه"))}{card("بلا خلية الإفلات بعد", "", ("tag-warn", "غير جاهز"))}{card("بلا إشعارات بعد", "التقدم في تلميح الأيقونة", ("tag-warn", "غير جاهز"))}</div>''',
                cls="alt s-w2", ident="gaps", labelledby="w2", side="end")
    else:
        h = page_head("SbarHive on Windows", "Not signed yet, so SmartScreen warns you")
        a = sec(f'''<h2 id="w1">Three steps</h2>
<ol class="step-row" role="list">{step_cell(1, "Verify the file", f'<a href="{dl}#verify">SHA256SUMS</a>')}{step_cell(2, "More info")}{step_cell(3, "Run anyway")}</ol>
<p class="section-intro">We have no signing certificate yet, and promise no store or date.</p>''', cls="s-w1", ident="why", labelledby="w1", edge=False, side="start")
        b = sec(f'''<h2 id="w2">Not finished yet</h2>
<div class="grid grid-3">{card("Not run on a real Windows machine", "", ("tag-warn", "Note"))}{card("No drop hive", "", ("tag-warn", "Missing"))}{card("No notifications", "Progress shows in the tray tooltip", ("tag-warn", "Missing"))}</div>''',
                cls="alt s-w2", ident="gaps", labelledby="w2", side="end")
    return h + a + b


def privacy(lang):
    ar = lang == "ar"
    if ar:
        h = page_head("ملفاتك تبقى عندك")
        a = sec(f'''<div class="split"><div><ul class="plate-list" role="list">{hex_plate("بلا رفع", "li", "accent")}{hex_plate("بلا تتبع", "li")}{hex_plate("بلا حسابات", "li")}</ul></div>
<div>{privacy_svg(lang)}</div></div>''', cls="s-p1", ident="never", edge=False, side="start")
        b = sec(f'''<h2 id="p2">الاتصال الوحيد هو فحص التحديثات</h2>
<div class="grid grid-2">{card("مرة في اليوم", "يقرأ البرنامج ملف تحديث موقّعا من صفحة إصدارات GitHub، فيرى GitHub عنوان IP الخاص بك كما في أي تنزيل.")}{card("تحقق قبل التنزيل", "التوقيع، ثم الحجم، ثم SHA-256.")}{card("قناتا التحديث", "<code>beta</code> و<code>stable</code>.")}{card("إيقاف الفحص", "<code>update_check = false</code> في <code>config.toml</code>.")}</div>''',
                cls="alt s-p2", ident="updates", labelledby="p2", side="end")
        c = sec(f'''<h2 id="p3">الأصل لا يُمسّ</h2>
<p class="section-intro">يُكتب الناتج أولا في ملف مؤقت، ثم يُعاد تسميته. الشيفرة مفتوحة على <a href="{REPO}">GitHub</a>.</p>''', cls="s-p3", ident="originals", labelledby="p3", side="start")
    else:
        h = page_head("Your files stay with you")
        a = sec(f'''<div class="split"><div><ul class="plate-list" role="list">{hex_plate("No uploads", "li", "accent")}{hex_plate("No tracking", "li")}{hex_plate("No accounts", "li")}</ul></div>
<div>{privacy_svg(lang)}</div></div>''', cls="s-p1", ident="never", edge=False, side="start")
        b = sec(f'''<h2 id="p2">The only connection: updates</h2>
<div class="grid grid-2">{card("Once a day", "The app reads a signed manifest from GitHub releases. GitHub sees your address, as with any download.")}{card("Verified first", "Signature, then size, then SHA-256.")}{card("Two channels", "<code>beta</code> and <code>stable</code>.")}{card("Turn it off", "<code>update_check = false</code> in <code>config.toml</code>.")}</div>''',
                cls="alt s-p2", ident="updates", labelledby="p2", side="end")
        c = sec(f'''<h2 id="p3">The original is never touched</h2>
<p class="section-intro">The result is written to a temporary file, then renamed. The code is open on <a href="{REPO}">GitHub</a>.</p>''', cls="s-p3", ident="originals", labelledby="p3", side="start")
    return h + a + b + c


def render(lang, page):
    root = rel_root(lang, page)
    body = home(lang) if page == "index" else {"download": download, "windows": windows, "privacy": privacy}[page](lang)
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
           "url": abs_url(lang, page), "author": {"@type": "Organization", "name": "SbarHive"}}]
    edit = ""
    if slug != "formats":
        fn = f"{slug}.ar.md" if ar else f"{slug}.md"
        edit = (f'<p class="edit"><a href="{EDIT_BASE}{fn}">{"عدّل هذه الصفحة على GitHub" if ar else "Edit this page on GitHub"}</a> · '
                f'<a href="{REPO}/issues">{"بلّغ عن مشكلة" if ar else "Report an issue"}</a> · '
                f'<a href="{slug}.md">Markdown</a></p>')
    else:
        edit = (f'<p class="edit"><a href="{REPO}/tree/main/actions">{"ملفات الإجراءات على GitHub" if ar else "Action files on GitHub"}</a> · '
                f'<a href="{slug}.md">Markdown</a></p>')
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
        pill = ("بيتا، لم يصدر بعد" if ar else "Beta, not tagged yet") if unreleased else ("مستقر" if ar else "Stable")
        cls = "pill pill-hot" if unreleased else "pill"
        items = "".join(f"<li>{bidi(html.escape((arx or en) if ar else en), lang)}</li>" for en, arx in r["items"][:6])
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
    desc = "كل إصدارات صبار هايف وما تغيّر فيها." if ar else "Every SbarHive release and what changed."
    latest = (f'<div class="latest"><span class="pill pill-hot"><bdi>{VERSION}</bdi></span>'
              f'<div class="actions">{dl_buttons(lang, "win")}{dl_buttons(lang, "linux")}</div>'
              f'<p class="fine"><a href="{page_href(lang, page, lang, "download")}#verify">{"تحقق من الملف" if ar else "Verify the file"}</a> · '
              f'<a href="{page_href(lang, page, lang, "windows")}">{"ويندوز غير موقّع" if ar else "Windows is unsigned"}</a> · '
              f'<a href="feed.xml">{"موجز الإصدارات" if ar else "Feed"}</a></p></div>')
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
        entries.append(f"<entry><title>SbarHive {ver}</title><id>{RELEASES}/tag/v{ver}</id>"
                       f'<link href="{RELEASES}/tag/v{ver}"/><updated>{upd}</updated><summary>{summary}</summary></entry>')
    return (f'<?xml version="1.0" encoding="utf-8"?>\n<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="{lang}">'
            f'<title>{"إصدارات صبار هايف" if ar else "SbarHive releases"}</title><id>{abs_url(lang, "releases")}</id>'
            f'<link href="{abs_url(lang, "releases")}"/><updated>{now}</updated>{"".join(entries)}</feed>\n')


ALL_PAGES = (["index", "download", "windows", "privacy", "releases", "docs/index"]
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
    idx = ["# SbarHive", "",
           "> A free, open source file converter for Linux and Windows. Select files, right click, pick a cell in a hexagonal hive; the result is saved next to the original. Everything runs on your machine: no uploads, no telemetry. First beta " + VERSION + ".",
           "", "## Docs", ""]
    for slug in DOC_ORDER:
        idx.append(f"- [{doc_title(slug, 'en')}]({BASE}en/docs/{slug}.md)")
    idx += ["", "## Project", "",
            f"- [Download]({abs_url('en', 'download')}): Linux DEB and RPM, Windows MSI and ZIP, with SHA256SUMS steps",
            f"- [Windows and SmartScreen]({abs_url('en', 'windows')}): the Windows build is unsigned",
            f"- [Privacy]({abs_url('en', 'privacy')}): no tracking, signed updates only",
            f"- [Releases]({BASE}en/releases.md)",
            f"- [Source]({REPO}): GPL-3.0-or-later",
            "", "## Arabic", "",
            f"- [التوثيق بالعربية]({abs_url('ar', 'docs/index')})", ""]
    full = ["# SbarHive documentation (English)", ""]
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
    build_extras()


if __name__ == "__main__":
    main()
