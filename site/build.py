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
            "pause": "إيقاف الحركة" if ar else "Pause motion",
            "play": "تشغيل الحركة" if ar else "Play motion",
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
                       f'<text x="{cx:.1f}" y="{cy:.1f}">{html.escape(kind["selection"])}</text></g>')
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


def journey_svg(lang):
    """Three nodes: files, the hive, the result next to the original."""
    rtl = lang == "ar"
    w, h = 720, 190
    xs = [110, 360, 610]
    if rtl:
        xs = [w - x for x in xs]
    big = 70
    out = [f'<svg class="journey" viewBox="0 0 {w} {h}" aria-hidden="true" focusable="false">']
    for a, b in ((xs[0], xs[1]), (xs[1], xs[2])):
        lo, hi = sorted((a, b))
        out.append(f'<line class="wire" x1="{lo + big * 0.9:.0f}" y1="95" x2="{hi - big * 0.9:.0f}" y2="95" pathLength="1"/>')
    for x in xs:
        out.append(f'<path class="node" d="{hex_path(x, 95, big)}"/>')
    # 1: two stacked files
    x = xs[0]
    out.append(f'<rect class="doc" x="{x - 30}" y="66" width="38" height="50" rx="6"/>')
    out.append(f'<rect class="doc doc-front" x="{x - 12}" y="76" width="38" height="50" rx="6"/>')
    # 2: the hive
    out.append(mini_cluster(xs[1], 95, 15, rtl, {0: "mc mc-hot"}))
    # 3: original and result
    x = xs[2]
    out.append(f'<rect class="doc" x="{x - 44}" y="70" width="36" height="48" rx="6"/>')
    out.append(f'<rect class="doc doc-new" x="{x + 8}" y="70" width="36" height="48" rx="6"/>')
    out.append("</svg>")
    return "".join(out)


def privacy_svg(lang):
    ar = lang == "ar"
    rtl = ar
    w, h = 640, 250

    def mx(x):
        return w - x if rtl else x

    t_pc = "جهازك" if ar else "Your machine"
    t_net = "الإنترنت" if ar else "The internet"
    t_none = "لا يخرج أي ملف" if ar else "No file leaves"
    cx_pc, cx_net = mx(190), mx(520)
    out = [f'<svg class="privacy-map" viewBox="0 0 {w} {h}" role="img" aria-label="{html.escape(t_pc)}: {html.escape(t_none)}">']
    out.append(f'<path class="zone" d="{hex_path(cx_pc, 125, 112)}"/>')
    out.append(f'<text class="zone-label" x="{cx_pc}" y="22">{t_pc}</text>')
    out.append(f'<rect class="doc" x="{cx_pc - 70}" y="100" width="28" height="38" rx="5"/>')
    out.append(f'<path class="hop" d="{hex_path(cx_pc, 120, 20)}"/>')
    out.append(f'<rect class="doc doc-new" x="{cx_pc + 42}" y="100" width="28" height="38" rx="5"/>')
    out.append(f'<line class="wire-s" x1="{cx_pc - 40}" y1="119" x2="{cx_pc - 22}" y2="119"/>')
    out.append(f'<line class="wire-s" x1="{cx_pc + 22}" y1="119" x2="{cx_pc + 40}" y2="119"/>')
    # internet: dashed zone
    out.append(f'<path class="zone zone-off" d="{hex_path(cx_net, 125, 80)}"/>')
    out.append(f'<text class="zone-label" x="{cx_net}" y="125">{t_net}</text>')
    # blocked link
    a, b = sorted((cx_pc + (112 * 0.87 if not rtl else -112 * 0.87), cx_net + (-80 * 0.87 if not rtl else 80 * 0.87)))
    mid = (a + b) / 2
    out.append(f'<line class="wire-off" x1="{a:.0f}" y1="125" x2="{b:.0f}" y2="125"/>')
    out.append(f'<g class="cross"><line x1="{mid - 9:.0f}" y1="116" x2="{mid + 9:.0f}" y2="134"/>'
               f'<line x1="{mid - 9:.0f}" y1="134" x2="{mid + 9:.0f}" y2="116"/></g>')
    out.append(f'<text class="zone-label strong" x="{mid:.0f}" y="165">{t_none}</text>')
    out.append("</svg>")
    return "".join(out)


def clip_frame(kind, caption):
    """A hexagonal frame with a small looping illustration. Illustrations only."""
    if kind == "resize":
        art = ('<rect class="a-img" x="30" y="30" width="60" height="60" rx="8"/>'
               '<path class="a-corner" d="M30 52V30H52M90 68V90H68"/>')
    elif kind == "audio":
        bars = "".join(f'<rect class="a-bar" style="--i:{i}" x="{20 + i * 12}" y="35" width="7" height="50" rx="3.5"/>'
                       for i in range(8))
        art = bars
    elif kind == "film":
        art = ('<rect class="a-film" x="20" y="34" width="80" height="52" rx="8"/>'
               '<path class="a-play" d="M52 49L72 60L52 71Z"/>')
    elif kind == "docs":
        art = ('<rect class="a-sheet" x="22" y="30" width="38" height="52" rx="6"/>'
               '<rect class="a-sheet a-docnew" x="60" y="38" width="38" height="52" rx="6"/>'
               '<path class="a-arrow" d="M44 92H76M68 84L76 92L68 100"/>')
    else:
        art = ('<rect class="a-sheet a-s1" x="26" y="34" width="38" height="52" rx="6"/>'
               '<rect class="a-sheet a-s2" x="56" y="34" width="38" height="52" rx="6"/>'
               '<rect class="a-sheet a-merged" x="41" y="30" width="38" height="60" rx="6"/>')
    return (f'<figure class="clip clip-{kind}"><div class="hexframe"><div class="hexframe-in">'
            f'<svg viewBox="0 0 120 120" aria-hidden="true" focusable="false">{art}</svg></div></div>'
            f'<figcaption>{caption}</figcaption></figure>')


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
<meta name="color-scheme" content="light dark">
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
<link rel="stylesheet" href="{root}style.css">
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
         '<path d="M12 21C5 15.5 3 12 3 8.7 3 6 5 4 7.6 4c1.8 0 3.3 1 4.4 2.6C13.100 5 14.600 4 16.400 4 19 4 21 6 21 8.700 21 12 19 15.500 12 21Z"/></svg>')


def support_a(cls, inner, extra=""):
    """The one support link shape: exact URL, new tab, no opener."""
    return (f'<a class="{cls}" href="{SUPPORT}" target="_blank" rel="noopener noreferrer"{extra}>'
            f'{inner}</a>')


def support_link(lang, cls="button button-support"):
    text = lang_pack(lang)["support"]
    return support_a(cls, f'{HEART}<span>{text}</span>', f' aria-label="{text}"')


def header(lang, page, root):
    p = lang_pack(lang)
    other = "en" if lang == "ar" else "ar"
    links = []
    for key in ("docs", "download", "releases"):
        cur = ' aria-current="page"' if (key == page or page.startswith(key + "/")) else ""
        links.append(f'<a href="{page_href(lang, page, lang, key)}"{cur}>{p["nav"][key]}</a>')
    sw = (f'<a class="lang-switch" href="{page_href(lang, page, other, page)}" '
          f'hreflang="{other}" lang="{other}">{p["other_lang"]}</a>')
    return f'''<header class="site-header">
  <div class="wrap">
    <a class="brand" href="{page_href(lang, page, lang, "index")}">
      <img src="{root}favicon.svg" alt="" width="32" height="32">
      <span>{p["name"]}</span><span class="badge">{p["beta"]}</span>
    </a>
    <nav class="site-nav" aria-label="{p["nav_label"]}">
      {"".join(links)}
      {support_link(lang, "button button-support button-sm")}
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
    ui = hive_config(lang)["ui"]
    lang_sw = (f'<a href="{page_href(lang, page, other, page)}" hreflang="{other}" lang="{other}">'
               f'{p["other_lang"]}</a>')
    return f'''</main>
<footer class="site-footer sec inv">
  <div class="hexedge" aria-hidden="true"></div>
  <div class="wrap">
    <p>{p["footer_license"]} <span class="muted">{p["footer_beta"]}</span></p>
    <ul class="footer-product" role="list">{items}</ul>
    <ul class="footer-lang" role="list"><li>{lang_sw}</li></ul>
    <button type="button" class="motion-toggle" id="motion-toggle" aria-pressed="false" data-on="{ui["pause"]}" data-off="{ui["play"]}">{ui["pause"]}</button>
  </div>
</footer>
<script id="hive-config" type="application/json">{json.dumps(hive_config(lang), ensure_ascii=False)}</script>
<script src="{root}app.js" defer></script>
</body>
</html>
'''


def sec(inner, *, cls="", scheme=None, ident=None, labelledby=None, side=None, edge=True, full=False):
    attrs = f' id="{ident}"' if ident else ""
    if labelledby:
        attrs += f' aria-labelledby="{labelledby}"'
    classes = "sec " + cls + (" full" if full else "")
    if scheme == "dark":
        classes += " inv"
    sd = f'<div class="side side-{side}" aria-hidden="true"></div>' if side else ""
    ed = '<div class="hexedge" aria-hidden="true"></div>' if edge else ""
    return f'<section class="{classes.strip()}"{attrs}>\n{ed}{sd}<div class="wrap">\n{inner}\n</div>\n</section>\n'


def code(text):
    return f'<pre><code>{html.escape(text)}</code></pre>'


# ---------------------------------------------------------------------------
# Shared blocks
# ---------------------------------------------------------------------------

WIN_ICO = ('<svg class="os-ico" viewBox="0 0 48 48" aria-hidden="true" focusable="false">'
           '<rect x="6" y="6" width="17" height="17" rx="3"/><rect x="25" y="6" width="17" height="17" rx="3"/>'
           '<rect x="6" y="25" width="17" height="17" rx="3"/><rect x="25" y="25" width="17" height="17" rx="3"/></svg>')
LIN_ICO = ('<svg class="os-ico" viewBox="0 0 48 48" aria-hidden="true" focusable="false">'
           '<rect x="5" y="9" width="38" height="30" rx="5" class="line"/>'
           '<path class="line" d="M14 20L22 26L14 32M26 33H35"/></svg>')


SOON = {"ar": "قريبا", "en": "Coming soon"}
LATIN = re.compile(r"\b(?:MSI|DEB|RPM|ZIP|EXE)\b|\d+\.\d+\.\d+(?:-[A-Za-z]+\.\d+)?")


def bidi(text, lang):
    """Arabic text: isolate Latin file codes and versions so they keep their order."""
    if lang != "ar":
        return text
    parts = re.split(r"(<[^>]*>)", text)
    return "".join(x if x.startswith("<") else LATIN.sub(lambda m: f"<bdi>{m.group(0)}</bdi>", x) for x in parts)


def dl_link(lang, kind, label, cls):
    """A download button. Until the release exists it opens the releases page."""
    href = FILES[kind] if PUBLISHED else RELEASES
    soon = "" if PUBLISHED else f' <span class="soon">{SOON[lang]}</span>'
    return f'<a class="{cls}" href="{href}" data-dl="{kind}">{label}{soon}</a>'


def dl_buttons(lang, which):
    """Direct download buttons. Names follow scripts/release/dry-run.sh and make-msi.sh."""
    ar = lang == "ar"
    word = "تنزيل" if ar else "Download"
    big = "button button-primary button-lg"
    if which == "win":
        return (dl_link(lang, "msi", bidi(f"{word} MSI", lang), big)
                + dl_link(lang, "zip", bidi("ZIP محمول" if ar else "Portable ZIP", lang), "button"))
    return (dl_link(lang, "deb", bidi(f"{word} DEB", lang), big)
            + dl_link(lang, "rpm", bidi(f"{word} RPM", lang), big))


def dl_cards(lang):
    ar = lang == "ar"
    win_t = "ويندوز" if ar else "Windows"
    win_s = "10 و11، 64 بت" if ar else "10 and 11, 64 bit"
    lin_t = "لينكس" if ar else "Linux"
    lin_s = "فيدورا وأوبونتو وديبيان، 64 بت" if ar else "Fedora, Ubuntu, Debian, 64 bit"
    return f'''<div class="dl-grid">
  <div class="dl-card" data-card="win">{WIN_ICO}<h3>{win_t}</h3><p class="dl-sub">{win_s}</p>
    <div class="actions">{dl_buttons(lang, "win")}</div></div>
  <div class="dl-card" data-card="linux">{LIN_ICO}<h3>{lin_t}</h3><p class="dl-sub">{lin_s}</p>
    <div class="actions">{dl_buttons(lang, "linux")}</div></div>
</div>'''


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


def marquee_block():
    codes = ["PNG", "JPG", "WEBP", "AVIF", "GIF", "PDF", "DOCX", "ODT", "MP4", "WEBM",
             "MKV", "MP3", "OGG", "FLAC", "WAV", "M4A", "XLSX", "PPTX", "TIFF", "ICO"]
    items = "".join(f"<li>{c}</li>" for c in codes)
    return (f'<div class="marquee sec" aria-hidden="true"><div class="hexedge"></div>'
            f'<ul class="track" dir="ltr">{items}{items}</ul></div>\n')


def tile(art, title, codes, span):
    chips = "".join(f"<li>{c}</li>" for c in codes)
    return (f'<div class="tile tile-{span} reveal" data-reveal>{clip_frame_bare(art)}'
            f'<h3>{title}</h3><ul class="chips" aria-hidden="true">{chips}</ul></div>')


def clip_frame_bare(kind):
    html_ = clip_frame(kind, "")
    return html_.replace("<figcaption></figcaption>", "").replace("<figure", "<div").replace("</figure>", "</div>")


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
        "how": "ثلاث خطوات",
        "steps": ["حدّد", "اختر", "خذ الناتج"],
        "privacy": "ملفاتك تبقى عندك",
        "ticks": ["بلا رفع", "بلا تتبع", "الأصل لا يتغير"],
        "formats": "صور وفيديو وصوت ومستندات",
        "tiles": ["صور", "فيديو", "صوت", "مستندات", "PDF"],
        "get": "نزّل صبار هايف",
        "open": "مجاني ومفتوح",
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
        "how": "Three steps",
        "steps": ["Select", "Pick", "Take it"],
        "privacy": "Your files stay with you",
        "ticks": ["No uploads", "No tracking", "Original untouched"],
        "formats": "Images, video, audio, documents",
        "tiles": ["Images", "Video", "Audio", "Documents", "PDF"],
        "get": "Get SbarHive",
        "open": "Free and open",
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
    """The try-it demo (S-061 AC7, AC8). Behaviour lives in app.js between the sim markers."""
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
<h2 id="sim-title">{t["h2"]}</h2>
<p class="section-intro">{t["intro"]}</p>
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
    return sec(inner, cls="s-sim", ident="sim", labelledby="sim-title", edge=False)


def support_section(lang):
    """The owner's badge in the Sbarah theme: the only link in the section."""
    t, p, r = T_HOME[lang], lang_pack(lang), rel_root(lang, "index")
    pic = (f'<picture><source media="(prefers-color-scheme: dark)" srcset="{r}assets/support-sbarah-dark.svg">'
           f'<img src="{r}assets/support-sbarah-light.svg" alt="{p["support"]}" width="520" height="140"></picture>')
    inner = f'''
<h2 id="support-title">{t["support_h2"]}</h2>
<p class="section-intro">{t["support_p"]}</p>
{support_a("support-badge", pic)}
'''
    return sec(inner, cls="s-support", ident="support", labelledby="support-title")


def home(lang):
    t = T_HOME[lang]
    win_href = page_href(lang, "index", lang, "windows")
    dl_href = page_href(lang, "index", lang, "download")
    win_a = f'<a href="{win_href}">{t["win_link"]}</a>'
    note = (t["note"] if PUBLISHED else t["note_soon"]).format(win=win_a)
    rel = ' data-rel="1"' if PUBLISHED else ""
    hero = f'''<section class="hero sec full" aria-labelledby="hero-title">
<div class="wrap hero-grid">
<div class="hero-copy">
<h1 id="hero-title">{t["h1"]}</h1>
<p class="lead">{t["sub"]}</p>
<div class="actions">
<a class="button button-primary button-lg" href="{dl_href}" data-os-default="linux"{rel}>{t["dl"]}</a>
<a class="link-all" href="{dl_href}">{t["all"]}</a>
</div>
<p class="hero-note">{note}</p>
</div>
{stage(lang)}
</div>
</section>
'''
    how = sec(f'''
<h2 id="how-title" class="reveal" data-reveal>{t["how"]}</h2>
<div class="journey-wrap reveal" data-reveal>{journey_svg(lang)}</div>
<ol class="step-words" role="list">
  <li class="reveal" data-reveal><span class="n">1</span><h3>{t["steps"][0]}</h3></li>
  <li class="reveal" data-reveal><span class="n">2</span><h3>{t["steps"][1]}</h3></li>
  <li class="reveal" data-reveal><span class="n">3</span><h3>{t["steps"][2]}</h3></li>
</ol>
''', cls="s-how", scheme="dark", ident="how", labelledby="how-title", side="start", full=True)
    ticks = "".join(f'<li class="reveal" data-reveal>{x}</li>' for x in t["ticks"])
    privacy = sec(f'''
<div class="split">
  <div>
    <h2 id="privacy-title" class="reveal" data-reveal>{t["privacy"]}</h2>
    <ul class="ticks" role="list">{ticks}</ul>
  </div>
  <div class="reveal" data-reveal>{privacy_svg(lang)}</div>
</div>
''', cls="s-privacy", ident="privacy", labelledby="privacy-title", side="end", full=True)
    tiles = [
        tile("resize", t["tiles"][0], ["PNG", "JPG", "WEBP", "AVIF"], "a"),
        tile("film", t["tiles"][1], ["MP4", "WEBM", "MKV", "GIF"], "b"),
        tile("audio", t["tiles"][2], ["MP3", "OGG", "FLAC", "WAV"], "c"),
        tile("docs", t["tiles"][3], ["DOCX", "ODT", "XLSX", "PPTX"], "d"),
        tile("merge", t["tiles"][4], ["PDF"], "e"),
    ]
    formats = sec(f'''
<h2 id="formats-title" class="reveal" data-reveal>{t["formats"]}</h2>
<div class="bento">{"".join(tiles)}</div>
''', cls="alt s-formats", ident="formats", labelledby="formats-title", side="start", full=True)
    get = sec(f'''
<h2 id="get-title" class="reveal" data-reveal>{t["get"]}</h2>
{dl_cards(lang)}
{pills(lang, dl_href + "#verify", win_href)}
''', cls="s-get", scheme="dark", ident="download", labelledby="get-title", side="end", full=True)
    open_ = sec(f'''
<h2 id="open-title" class="reveal" data-reveal>{t["open"]}</h2>
<div class="actions">
  <a class="button button-lg" href="{REPO}">{t["gh"]}</a>
</div>
''', cls="s-open", ident="open", labelledby="open-title", side="start")
    return hero + marquee_block() + sim_section(lang) + how + privacy + formats + get + open_ + support_section(lang)


# ---------------------------------------------------------------------------
# Inner pages
# ---------------------------------------------------------------------------

def page_head(title, lead=""):
    lead_html = f'<p class="lead">{lead}</p>' if lead else ""
    return (f'<section class="page-head sec"><div class="side side-end" aria-hidden="true"></div><div class="wrap">'
            f'<h1>{title}</h1>{lead_html}</div></section>\n')


def card(title, body="", tag=""):
    tg = f'<span class="tag {tag[0]}">{tag[1]}</span>' if tag else ""
    bd = f"<p>{body}</p>" if body else ""
    return f'<div class="card reveal" data-reveal>{tg}<h3>{title}</h3>{bd}</div>'


def download(lang):
    ar = lang == "ar"
    win_href = page_href(lang, "download", lang, "windows")
    if ar:
        h = page_head("نزّل صبار هايف", f"أول إصدار بيتا <code>{VERSION}</code>" + ("" if PUBLISHED else "، قريبا"))
        top = sec(f'{dl_cards(lang)}<ul class="facts" role="list"><li><span class="pill pill-hot"><bdi>{VERSION}</bdi></span></li><li><a href="{win_href}">ويندوز غير موقّع</a></li><li><a href="{RELEASES}">كل الإصدارات</a></li></ul>',
                  cls="s-dl", ident="choose", edge=False)
        ver = sec(f'''<h2 id="v-title" class="reveal" data-reveal>تحقق من الملف</h2>
<p class="section-intro">نزّل <code>SHA256SUMS</code> في المجلد نفسه الذي فيه الملفات، ويجب أن يظهر OK أمام كل ملف.</p>
<div class="split"><div class="reveal" data-reveal><h3>لينكس</h3>{code("sha256sum -c SHA256SUMS --ignore-missing")}</div>
<div class="reveal" data-reveal><h3>ويندوز</h3>{code(f"certutil -hashfile sbarhive-{VERSION}-windows-x64.msi SHA256")}</div></div>
<p class="section-intro">وإن اختلفت القيمة فلا تشغّل الملف.</p>''', cls="alt s-verify", ident="verify", labelledby="v-title", side="start")
        src = sec(f'''<h2 id="b-title" class="reveal" data-reveal>البناء من الشيفرة على لينكس</h2>
{code(f"git clone {REPO}" + chr(10) + "cd sbarhive" + chr(10) + "./scripts/install.sh")}
<p class="section-intro">يحتاج Rust 1.85 أو أحدث. للإزالة: <code>./scripts/uninstall.sh</code></p>''', cls="s-build", ident="build", labelledby="b-title", side="end")
    else:
        h = page_head("Download SbarHive", f"First beta, <code>{VERSION}</code>" + ("" if PUBLISHED else ", coming soon"))
        top = sec(f'{dl_cards(lang)}<ul class="facts" role="list"><li><span class="pill pill-hot"><bdi>{VERSION}</bdi></span></li><li><a href="{win_href}">Windows is unsigned</a></li><li><a href="{RELEASES}">All releases</a></li></ul>',
                  cls="s-dl", ident="choose", edge=False)
        ver = sec(f'''<h2 id="v-title" class="reveal" data-reveal>Verify the file</h2>
<p class="section-intro">Put <code>SHA256SUMS</code> in the packages folder; every file must say OK.</p>
<div class="split"><div class="reveal" data-reveal><h3>Linux</h3>{code("sha256sum -c SHA256SUMS --ignore-missing")}</div>
<div class="reveal" data-reveal><h3>Windows</h3>{code(f"certutil -hashfile sbarhive-{VERSION}-windows-x64.msi SHA256")}</div></div>
<p class="section-intro">If a value differs, do not run the file.</p>''', cls="alt s-verify", ident="verify", labelledby="v-title", side="start")
        src = sec(f'''<h2 id="b-title" class="reveal" data-reveal>From source on Linux</h2>
{code(f"git clone {REPO}" + chr(10) + "cd sbarhive" + chr(10) + "./scripts/install.sh")}
<p class="section-intro">Needs Rust 1.85 or newer. To remove: <code>./scripts/uninstall.sh</code></p>''', cls="s-build", ident="build", labelledby="b-title", side="end")
    return h + top + ver + src


def windows(lang):
    ar = lang == "ar"
    dl = page_href(lang, "windows", lang, "download")
    if ar:
        h = page_head("صبار هايف على ويندوز", "غير موقّع بعد، فيظهر تحذير SmartScreen")
        a = sec(f'''<h2 id="w1" class="reveal" data-reveal>ثلاث خطوات</h2>
<ol class="step-words" role="list">
<li class="reveal" data-reveal><span class="n">1</span><h3>تحقق من الملف</h3><p><a href="{dl}#verify">SHA256SUMS</a></p></li>
<li class="reveal" data-reveal><span class="n">2</span><h3>More info</h3></li>
<li class="reveal" data-reveal><span class="n">3</span><h3>Run anyway</h3></li></ol>
<p class="section-intro">ليست لدينا شهادة توقيع بعد، ولا نعد بمتجر أو بموعد.</p>''', cls="s-w1", ident="why", labelledby="w1", edge=False, side="start")
        b = sec(f'''<h2 id="w2" class="reveal" data-reveal>لم يكتمل بعد</h2>
<div class="grid grid-3">{card("لم يُجرَّب على جهاز ويندوز حقيقي", "", ("tag-warn", "تنبيه"))}{card("بلا خلية الإفلات بعد", "", ("tag-warn", "غير جاهز"))}{card("بلا إشعارات بعد", "التقدم في تلميح الأيقونة", ("tag-warn", "غير جاهز"))}</div>''',
                cls="alt s-w2", ident="gaps", labelledby="w2", side="end")
    else:
        h = page_head("SbarHive on Windows", "Not signed yet, so SmartScreen warns you")
        a = sec(f'''<h2 id="w1" class="reveal" data-reveal>Three steps</h2>
<ol class="step-words" role="list">
<li class="reveal" data-reveal><span class="n">1</span><h3>Verify the file</h3><p><a href="{dl}#verify">SHA256SUMS</a></p></li>
<li class="reveal" data-reveal><span class="n">2</span><h3>More info</h3></li>
<li class="reveal" data-reveal><span class="n">3</span><h3>Run anyway</h3></li></ol>
<p class="section-intro">We have no signing certificate yet, and promise no store or date.</p>''', cls="s-w1", ident="why", labelledby="w1", edge=False, side="start")
        b = sec(f'''<h2 id="w2" class="reveal" data-reveal>Not finished yet</h2>
<div class="grid grid-3">{card("Not run on a real Windows machine", "", ("tag-warn", "Note"))}{card("No drop hive", "", ("tag-warn", "Missing"))}{card("No notifications", "Progress shows in the tray tooltip", ("tag-warn", "Missing"))}</div>''',
                cls="alt s-w2", ident="gaps", labelledby="w2", side="end")
    return h + a + b


def privacy(lang):
    ar = lang == "ar"
    if ar:
        h = page_head("ملفاتك تبقى عندك")
        a = sec(f'''<div class="split"><div><ul class="ticks" role="list"><li class="reveal" data-reveal>بلا رفع</li><li class="reveal" data-reveal>بلا تتبع</li><li class="reveal" data-reveal>بلا حسابات</li></ul></div>
<div class="reveal" data-reveal>{privacy_svg(lang)}</div></div>''', cls="s-p1", ident="never", edge=False, side="start")
        b = sec(f'''<h2 id="p2" class="reveal" data-reveal>الاتصال الوحيد هو فحص التحديثات</h2>
<div class="grid grid-2">{card("مرة في اليوم", "يقرأ البرنامج ملف تحديث موقّعا من صفحة إصدارات GitHub، فيرى GitHub عنوان IP الخاص بك كما في أي تنزيل.")}{card("تحقق قبل التنزيل", "التوقيع، ثم الحجم، ثم SHA-256.")}{card("قناتا التحديث", "<code>beta</code> و<code>stable</code>.")}{card("إيقاف الفحص", "<code>update_check = false</code> في <code>config.toml</code>.")}</div>''',
                cls="alt s-p2", ident="updates", labelledby="p2", side="end")
        c = sec(f'''<h2 id="p3" class="reveal" data-reveal>الأصل لا يُمسّ</h2>
<p class="section-intro">يُكتب الناتج أولا في ملف مؤقت، ثم يُعاد تسميته. الشيفرة مفتوحة على <a href="{REPO}">GitHub</a>.</p>''', cls="s-p3", ident="originals", labelledby="p3", side="start")
    else:
        h = page_head("Your files stay with you")
        a = sec(f'''<div class="split"><div><ul class="ticks" role="list"><li class="reveal" data-reveal>No uploads</li><li class="reveal" data-reveal>No tracking</li><li class="reveal" data-reveal>No accounts</li></ul></div>
<div class="reveal" data-reveal>{privacy_svg(lang)}</div></div>''', cls="s-p1", ident="never", edge=False, side="start")
        b = sec(f'''<h2 id="p2" class="reveal" data-reveal>The only connection: updates</h2>
<div class="grid grid-2">{card("Once a day", "The app reads a signed manifest from GitHub releases. GitHub sees your address, as with any download.")}{card("Verified first", "Signature, then size, then SHA-256.")}{card("Two channels", "<code>beta</code> and <code>stable</code>.")}{card("Turn it off", "<code>update_check = false</code> in <code>config.toml</code>.")}</div>''',
                cls="alt s-p2", ident="updates", labelledby="p2", side="end")
        c = sec(f'''<h2 id="p3" class="reveal" data-reveal>The original is never touched</h2>
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
            out.append(f'<figure class="diagram"><picture>'
                       f'<source media="(prefers-color-scheme: dark)" srcset="{r_}diagrams/{name}.{lang}.dark.svg">'
                       f'<img src="{r_}diagrams/{name}.{lang}.light.svg" alt="{html.escape(alt)}" loading="lazy"></picture></figure>')
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
        tiles.append(f'<a class="doc-tile reveal" data-reveal href="{page_href(lang, page, lang, "docs/" + slug)}">'
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
        out.append(f'<article class="release reveal" data-reveal><h2 id="v{ver.replace(".", "-")}"><span dir="ltr">{ver}</span>{name}</h2>'
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
    for f in (ROOT / "docs" / "diagrams").glob("*.svg"):
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
