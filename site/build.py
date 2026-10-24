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
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOKENS = json.loads((HERE.parent / "design" / "tokens.json").read_text(encoding="utf-8"))
SQRT3 = math.sqrt(3)
R = TOKENS["hex"]["size"]["md"] / 2
GAP = TOKENS["hex"]["gap"]
PAD = 4
BASE = "https://m7mmad-omar.github.io/sbarhive/"
REPO = "https://github.com/M7MMAD-OMAR/sbarhive"
RELEASES = REPO + "/releases"
VERSION = "0.4.0-beta.1"
DIRECTIONS = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)]

PAGES = ["index", "download", "windows", "privacy"]


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
HALF_W = S * SQRT3 * 2 + R * SQRT3 / 2 + PAD
HALF_H = S * 1.5 * 2 + R + PAD
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
        ("رمادي" if ar else "GRAY", "رمادي: جعل الصورة رمادية" if ar else "Gray: make it gray", "photo-gray.png"),
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
        return {"key": key, "label": label, "selection": sel, "files": files,
                "cells": [{"c": c, "n": n, "o": o} for c, n, o in cells]}

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
            "idle": "مرّر على خلية" if ar else "Hover a cell",
            "working": "جارٍ التحويل" if ar else "Working",
            "done": "تم" if ar else "Done",
            "tray": "صبار هايف" if ar else "SbarHive",
            "doneNote": "اكتمل: " if ar else "Finished: ",
            "original": "الأصل كما هو" if ar else "Original, untouched",
            "fresh": "جديد" if ar else "New",
            "stepsOn": ["١", "٢", "٣"] if ar else ["1", "2", "3"],
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
    for i, (q, r) in enumerate(spiral(2)):
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
    t_none = "لا ملف يخرج" if ar else "No file leaves"
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
    else:
        art = ('<rect class="a-sheet a-s1" x="26" y="34" width="38" height="52" rx="6"/>'
               '<rect class="a-sheet a-s2" x="56" y="34" width="38" height="52" rx="6"/>'
               '<rect class="a-sheet a-merged" x="41" y="30" width="38" height="60" rx="6"/>')
    return (f'<figure class="clip clip-{kind}"><div class="hexframe"><div class="hexframe-in">'
            f'<svg viewBox="0 0 120 120" aria-hidden="true" focusable="false">{art}</svg></div></div>'
            f'<figcaption>{caption}</figcaption></figure>')


# ---------------------------------------------------------------------------
# Language packs. Two originals, same structure.
# ---------------------------------------------------------------------------

AR = {
    "lang": "ar", "dir": "rtl", "name": "صبار هايف", "locale": "ar_AR", "alt": "en_US",
    "nav_skip": "انتقل إلى المحتوى",
    "nav": {"index": "الرئيسية", "download": "التنزيل", "windows": "ويندوز", "privacy": "الخصوصية"},
    "nav_label": "التنقل الرئيسي",
    "other_lang": "English",
    "beta": "تجريبي",
    "titles": {
        "index": "صبار هايف: حوّل ملفاتك من قائمة النقر بالزر الأيمن",
        "download": "تنزيل صبار هايف " + VERSION + " للينكس وويندوز",
        "windows": "صبار هايف على ويندوز: تحذير SmartScreen والتحقق من الملف",
        "privacy": "الخصوصية في صبار هايف: ملفاتك لا تغادر جهازك",
    },
    "descs": {
        "index": "محوّل ملفات مجاني ومفتوح المصدر للينكس وويندوز. حدّد ملفاتك واختر الصيغة من خلية سداسية، فيُحفظ الناتج بجانب الأصل وتبقى ملفاتك على جهازك.",
        "download": "الإصدار التجريبي الأول من صبار هايف للينكس وويندوز، مع خطوات التحقق من الملف بـ SHA256SUMS.",
        "windows": "نسخة ويندوز من صبار هايف غير موقّعة بعد. نشرح تحذير SmartScreen ونبيّن كيف تتحقق من ملفك قبل تشغيله.",
        "privacy": "لا تتبع ولا حسابات ولا خوادم معالجة. الاتصال الوحيد هو فحص التحديثات الموقّعة.",
    },
    "og_alt": "شعار صبار هايف بجانب الخلية السداسية وفيها صيغ تحويل الصور",
    "footer_license": "صبار هايف، برنامج حر برخصة GPLv3.",
    "footer_support": "ادعم صبارة",
    "footer_beta": "الإصدار التجريبي <bdi dir=\"ltr\">" + VERSION + "</bdi>",
}

EN = {
    "lang": "en", "dir": "ltr", "name": "SbarHive", "locale": "en_US", "alt": "ar_AR",
    "nav_skip": "Skip to content",
    "nav": {"index": "Home", "download": "Download", "windows": "Windows", "privacy": "Privacy"},
    "nav_label": "Main navigation",
    "other_lang": "العربية",
    "beta": "beta",
    "titles": {
        "index": "SbarHive: convert files from the right click menu",
        "download": "Download SbarHive " + VERSION + " for Linux and Windows",
        "windows": "SbarHive on Windows: the SmartScreen warning and checking your download",
        "privacy": "SbarHive privacy: your files never leave your machine",
    },
    "descs": {
        "index": "A free, open source file converter for Linux and Windows. Select files, pick a format from a hexagonal hive, and the result is saved next to the original while your files stay on your machine.",
        "download": "The first SbarHive beta for Linux and Windows, with the SHA256SUMS steps to verify your download.",
        "windows": "The Windows build of SbarHive is not signed yet. This explains the SmartScreen warning and how to verify your file before running it.",
        "privacy": "No tracking, no accounts, no processing servers. The only network use is the signed update check.",
    },
    "og_alt": "The SbarHive logo next to a hexagonal hive with image formats",
    "footer_license": "SbarHive is free software under the GPLv3.",
    "footer_support": "Support Sbarah",
    "footer_beta": "Beta " + VERSION,
}

SUPPORT = "https://sbarah.com/support"


def lang_pack(lang):
    return AR if lang == "ar" else EN


# ---------------------------------------------------------------------------
# Page shell
# ---------------------------------------------------------------------------

def rel_root(lang, page):
    depth = (1 if lang == "en" else 0) + (0 if page == "index" else 1)
    return "../" * depth


def page_href(from_lang, from_page, to_lang, to_page):
    """Relative link between any two pages."""
    here = (["en"] if from_lang == "en" else []) + ([] if from_page == "index" else [from_page])
    there = (["en"] if to_lang == "en" else []) + ([] if to_page == "index" else [to_page])
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
    parts = (["en"] if lang == "en" else []) + ([] if page == "index" else [page])
    return BASE + "/".join(parts) + ("/" if parts else "")


def head(lang, page, root):
    p = lang_pack(lang)
    title = html.escape(p["titles"][page])
    desc = html.escape(p["descs"][page])
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
<link rel="icon" href="{root}favicon.svg" type="image/svg+xml">
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
<script>document.documentElement.classList.add("js")</script>
</head>
<body>
<a class="skip" href="#main">{p["nav_skip"]}</a>
'''


def header(lang, page, root):
    p = lang_pack(lang)
    other = "en" if lang == "ar" else "ar"
    links = []
    for key in ("download", "windows", "privacy"):
        cur = ' aria-current="page"' if key == page else ""
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
      {sw}
    </nav>
  </div>
</header>
<main id="main">
'''


def footer(lang, page, root):
    p = lang_pack(lang)
    other = "en" if lang == "ar" else "ar"
    links = " · ".join([
        f'<a href="{REPO}">GitHub</a>',
        f'<a href="{RELEASES}">{"الإصدارات" if lang == "ar" else "Releases"}</a>',
        f'<a href="{SUPPORT}">{p["footer_support"]}</a>',
        f'<a href="{page_href(lang, page, other, page)}" hreflang="{other}" lang="{other}">{p["other_lang"]}</a>',
    ])
    return f'''</main>
<footer class="site-footer sec inv">
  <div class="hexedge" aria-hidden="true"></div>
  <div class="wrap">
    <p>{p["footer_license"]} <span class="muted">{p["footer_beta"]}</span></p>
    <p>{links}</p>
  </div>
</footer>
<script id="hive-config" type="application/json">{json.dumps(hive_config(lang), ensure_ascii=False)}</script>
<script src="{root}app.js" defer></script>
</body>
</html>
'''


def sec(inner, *, cls="", scheme=None, ident=None, labelledby=None, side=None, edge=True):
    attrs = f' id="{ident}"' if ident else ""
    if labelledby:
        attrs += f' aria-labelledby="{labelledby}"'
    classes = "sec " + cls
    if scheme == "dark":
        classes += " inv"
    sd = f'<div class="side side-{side}" aria-hidden="true"></div>' if side else ""
    ed = '<div class="hexedge" aria-hidden="true"></div>' if edge else ""
    return f'<section class="{classes.strip()}"{attrs}>\n{ed}{sd}<div class="wrap">\n{inner}\n</div>\n</section>\n'


def rv(inner, tag="div", cls=""):
    c = ("reveal " + cls).strip()
    return f'<{tag} class="{c}" data-reveal>{inner}</{tag}>'


def code(text):
    return f'<pre><code>{html.escape(text)}</code></pre>'


# ---------------------------------------------------------------------------
# Home
# ---------------------------------------------------------------------------

def home_ar(root):
    L = "ar"
    hero = f'''<section class="hero sec" aria-labelledby="hero-title">
<div class="side side-end" aria-hidden="true"></div>
<div class="wrap hero-grid">
  <div class="hero-copy">
    <p class="eyebrow"><span class="pill pill-hot">{VERSION}</span> مجاني ومفتوح المصدر، للينكس وويندوز</p>
    <h1 id="hero-title">حوّل ملفاتك من قائمة <span class="hl">النقر بالزر الأيمن</span></h1>
    <p class="lead">حدّد ملفا أو أكثر، فتظهر خلية سداسية فيها ما يناسب هذه الملفات فقط. اختر خلية، فيُحفظ الناتج بجانب الأصل، وتبقى ملفاتك على جهازك.</p>
    <div class="actions">
      <a class="button button-primary" href="{page_href(L, "index", L, "download")}">نزّل النسخة التجريبية</a>
      <a class="button" href="#how">كيف يعمل؟</a>
    </div>
    <p class="fine">بيتا: قد تصادف بعض العيوب. بناء ويندوز غير موقّع بعد، <a href="{page_href(L, "index", L, "windows")}">ماذا يعني هذا؟</a>.</p>
  </div>
  {stage(L)}
</div>
</section>
'''
    marquee = marquee_block()
    how = sec(f'''
<h2 id="how-title" class="reveal" data-reveal>ثلاث خطوات، ولا نافذة تفتحها</h2>
<div class="journey-wrap reveal" data-reveal>{journey_svg(L)}</div>
<ol class="steps grid grid-3" role="list">
  <li class="card reveal" data-reveal><h3>حدّد الملفات</h3><p>في Dolphin أو Nautilus أو Nemo أو Thunar، أو في مستكشف ويندوز، ثم انقر بالزر الأيمن واختر «صبار هايف».</p></li>
  <li class="card reveal" data-reveal><h3>اختر خلية</h3><p>لا تظهر إلا الإجراءات التي تناسب نوع الملفات وعددها، وتعمل بالأدوات المثبتة عندك.</p></li>
  <li class="card reveal" data-reveal><h3>خذ الناتج</h3><p>يُحفظ الناتج باسم جديد في المجلد نفسه، ويبقى الأصل كما هو. يظهر التقدم في تلميح أيقونة صينية النظام، لا في نافذة تسرق التركيز.</p></li>
</ol>
<p class="section-intro">وعلى Hyprland تستطيع ربط Super مع النقر بالزر الأيمن ليفتح الخلية مباشرة.</p>
''', cls="s-how", scheme="dark", ident="how", labelledby="how-title", side="start")
    privacy = sec(f'''
<div class="split">
  <div>
    <h2 id="privacy-title" class="reveal" data-reveal>ملفاتك لا تغادر جهازك</h2>
    <ul class="ticks" role="list">
      <li class="reveal" data-reveal><strong>لا رفع.</strong> كل تحويل يجري على جهازك. لا خادم يستقبل الملف ولا حساب تسجّل به.</li>
      <li class="reveal" data-reveal><strong>لا جمع بيانات.</strong> لا يرسل البرنامج شيئا عن ملفاتك ولا عن استعماله.</li>
      <li class="reveal" data-reveal><strong>الأصل محفوظ.</strong> يُكتب الناتج أولا في ملف مؤقت ثم يأخذ اسمه النهائي، فلا يُمسّ الأصل ولا يبقى ملف ناقص إن توقف العمل.</li>
    </ul>
    <p class="fine"><a href="{page_href(L, "index", L, "privacy")}">التفاصيل الكاملة، وما تراه GitHub عند التحديث</a></p>
  </div>
  <div class="reveal" data-reveal>{privacy_svg(L)}</div>
</div>
''', cls="s-privacy", ident="privacy", labelledby="privacy-title", side="end")
    formats = sec(f'''
<h2 id="formats-title" class="reveal" data-reveal>ما الذي يحوّله؟</h2>
<div class="clips">
  {clip_frame("resize", "الصور: تغيير الحجم والتدوير والضغط")}
  {clip_frame("audio", "الفيديو والصوت: استخراج الصوت والتحويل")}
  {clip_frame("merge", "PDF: دمج وتقسيم وضغط")}
</div>
<p class="fine center">رسوم متحركة صُمّمت للموقع، وليست تسجيلا للتطبيق.</p>
<div class="grid grid-3">
  <div class="card reveal" data-reveal><span class="tag">الصور</span><h3>PNG وJPEG وWebP</h3>
    <p>تحويل الصيغة، وتغيير الحجم، وتدوير، وقلب، وتحويل الصورة إلى الرمادي، وضغطها، وحذف البيانات الوصفية، وجمع عدة صور في PDF واحد.</p>
    <ul class="chips" aria-label="صيغ الصور"><li>PNG</li><li>JPEG</li><li>WebP</li><li>AVIF</li><li>GIF</li><li>BMP</li><li>TIFF</li><li>ICO</li></ul></div>
  <div class="card reveal" data-reveal><span class="tag">الفيديو والصوت</span><h3>MP4 وMP3 وأكثر</h3>
    <p>تحويل الفيديو وضغطه، واستخراج الصوت منه، وتحويل الصوت بين صيغه. يحتاج FFmpeg.</p>
    <ul class="chips" aria-label="صيغ الفيديو والصوت"><li>MP4</li><li>WebM</li><li>MKV</li><li>MOV</li><li>MP3</li><li>OGG</li><li>FLAC</li><li>WAV</li><li>M4A</li></ul></div>
  <div class="card reveal" data-reveal><span class="tag">المستندات وPDF</span><h3>Office وODF وPDF</h3>
    <p>تحويل المستندات إلى PDF وبين DOCX وODT، وضغط PDF ودمجه وتدويره وتقسيمه وتحويل صفحاته إلى صور. يحتاج LibreOffice وGhostscript.</p>
    <ul class="chips" aria-label="صيغ المستندات"><li>DOCX</li><li>ODT</li><li>XLSX</li><li>PPTX</li><li>ODP</li><li>PDF</li></ul></div>
</div>
<p class="section-intro">الإجراء الذي يحتاج أداة غير مثبتة لا يظهر في الخلية، والأمر <code>sbarhive doctor</code> يعرض ما وُجد على جهازك وما ينقص.</p>
''', cls="alt s-formats", ident="formats", labelledby="formats-title", side="start")
    status = sec(f'''
<h2 id="status-title" class="reveal" data-reveal>أين وصل المشروع فعلا؟</h2>
<div class="grid grid-3">
  <div class="card reveal" data-reveal><span class="tag tag-ok">لينكس</span><h3>مختبر على لينكس</h3>
    <p>الخلية تعمل على Hyprland وKDE Plasma 6 وGNOME 50، وحزم RPM وDEB تُثبَّت وتعمل. على GNOME وKDE ولم يكتمل بعد اختبار تركيز نافذة الخلية على أجهزة GNOME وKDE الحقيقية.</p></div>
  <div class="card reveal" data-reveal><span class="tag tag-warn">ويندوز</span><h3>تجريبي، غير موقّع</h3>
    <p>MSI لكل مستخدم بلا صلاحيات المدير، وبند في قائمة النقر بالزر الأيمن، وأيقونة في صينية النظام. لم يُجرَّب على جهاز ويندوز حقيقي بعد، وفيه تحذير SmartScreen لأن لا شهادة توقيع لدينا.</p></div>
  <div class="card reveal" data-reveal><span class="tag">لاحقا</span><h3>لم يصل بعد</h3>
    <p>X11، وخلية السحب والإفلات والإشعارات على ويندوز، وmacOS. لا وعد بتاريخ، والخطة معلنة في المستودع.</p></div>
</div>
<p class="section-intro">لا نعدك بأن يتوفر البرنامج في متجر تطبيقات أو مدير حزم. المتاح الآن هو ما تراه في صفحة التنزيل.</p>
''', cls="s-status", scheme="dark", ident="status", labelledby="status-title", side="end")
    cta = sec(f'''
<div class="cta-card">
  <div>
    <h2 id="cta-title" class="reveal" data-reveal>جرّبه، وأخبرنا بما تجده</h2>
    <p class="lead">الإصدار التجريبي الأول للينكس وويندوز. التنزيل من صفحة الإصدارات في GitHub، مع ملف SHA256SUMS للتحقق.</p>
  </div>
  <div class="actions">
    <a class="button button-primary" href="{page_href(L, "index", L, "download")}">صفحة التنزيل</a>
    <a class="button" href="{REPO}">الشيفرة على GitHub</a>
  </div>
</div>
''', cls="s-cta", ident="get", labelledby="cta-title", side="start")
    compare = sec(f'''
<h2 id="compare-title" class="reveal" data-reveal>وماذا عن الأدوات الأخرى؟</h2>
<div class="grid grid-3">
  <div class="card reveal" data-reveal><h3>File Converter</h3><p>يعمل بفكرة القائمة نفسها، لكن على ويندوز وحده. صبار هايف يغطي لينكس وويندوز بنسخة واحدة.</p></div>
  <div class="card reveal" data-reveal><h3>FFmpeg وImageMagick</h3><p>يحوّلان كل شيء تقريبا من الطرفية، لمن يحفظ خياراتهما. صبار هايف يستعمل FFmpeg للفيديو والصوت، ويضع الخيارات في خلية.</p></div>
  <div class="card reveal" data-reveal><h3>إضافات مديري الملفات</h3><p>كل منها يقتصر على الصور أو على مهمة واحدة في مدير ملفات واحد. صبار هايف يتولى الصور والفيديو والصوت والمستندات من قائمة واحدة.</p></div>
</div>
''', cls="alt s-compare", ident="compare", labelledby="compare-title")
    license_ = sec(f'''
<h2 id="license-title" class="reveal" data-reveal>مفتوح المصدر</h2>
<p class="section-intro">الرخصة GPLv3 أو أي إصدار لاحق منها. الشيفرة وبلاغات الأخطاء والمساهمات على <a href="{REPO}">GitHub</a>. ولإضافة صيغة جديدة يكفي ملف TOML ومحوّل صغير.</p>
''', cls="s-license", ident="license", labelledby="license-title", side="end")
    return hero + marquee + how + privacy + formats + status + cta + compare + license_


def home_en(root):
    L = "en"
    hero = f'''<section class="hero sec" aria-labelledby="hero-title">
<div class="side side-end" aria-hidden="true"></div>
<div class="wrap hero-grid">
  <div class="hero-copy">
    <p class="eyebrow"><span class="pill pill-hot">{VERSION}</span> Free and open source, for Linux and Windows</p>
    <h1 id="hero-title">Convert files from the <span class="hl">right click</span> menu</h1>
    <p class="lead">Select one file or many and a hexagonal hive shows only what fits them. Pick a cell and the result is saved next to the original, while your files stay on your machine.</p>
    <div class="actions">
      <a class="button button-primary" href="{page_href(L, "index", L, "download")}">Get the beta</a>
      <a class="button" href="#how">How it works</a>
    </div>
    <p class="fine">Beta: expect rough edges. The Windows build is not signed yet, <a href="{page_href(L, "index", L, "windows")}">and here is what that means</a>.</p>
  </div>
  {stage(L)}
</div>
</section>
'''
    marquee = marquee_block()
    how = sec(f'''
<h2 id="how-title" class="reveal" data-reveal>Three steps, and no window to open</h2>
<div class="journey-wrap reveal" data-reveal>{journey_svg(L)}</div>
<ol class="steps grid grid-3" role="list">
  <li class="card reveal" data-reveal><h3>Select files</h3><p>In Dolphin, Nautilus, Nemo or Thunar, or in Windows Explorer. Then right click and choose SbarHive.</p></li>
  <li class="card reveal" data-reveal><h3>Pick a cell</h3><p>Only actions that fit the kind and number of files show up, and only those your installed tools can run.</p></li>
  <li class="card reveal" data-reveal><h3>Take the result</h3><p>The result is saved under a new name in the same folder and the original stays as it was. Progress shows in the tray tooltip, never in a window that steals focus.</p></li>
</ol>
<p class="section-intro">On Hyprland you can bind Super plus right click to open the hive directly.</p>
''', cls="s-how", scheme="dark", ident="how", labelledby="how-title", side="start")
    privacy = sec(f'''
<div class="split">
  <div>
    <h2 id="privacy-title" class="reveal" data-reveal>Your files never leave your machine</h2>
    <ul class="ticks" role="list">
      <li class="reveal" data-reveal><strong>No uploads.</strong> Every conversion runs locally. No server receives your file and there is no account to create.</li>
      <li class="reveal" data-reveal><strong>No data collection.</strong> The app sends nothing about your files or how you use it.</li>
      <li class="reveal" data-reveal><strong>The original is safe.</strong> Output is written to a temporary file first and renamed when complete, so the original is never touched and a stopped job leaves no half file.</li>
    </ul>
    <p class="fine"><a href="{page_href(L, "index", L, "privacy")}">Full details, including what GitHub sees on an update</a></p>
  </div>
  <div class="reveal" data-reveal>{privacy_svg(L)}</div>
</div>
''', cls="s-privacy", ident="privacy", labelledby="privacy-title", side="end")
    formats = sec(f'''
<h2 id="formats-title" class="reveal" data-reveal>What does it convert?</h2>
<div class="clips">
  {clip_frame("resize", "Images: resize, rotate, compress")}
  {clip_frame("audio", "Video and audio: extract and convert")}
  {clip_frame("merge", "PDF: merge, split, compress")}
</div>
<p class="fine center">Animated illustrations made for this site, not a recording of the app.</p>
<div class="grid grid-3">
  <div class="card reveal" data-reveal><span class="tag">Images</span><h3>PNG, JPEG, WebP</h3>
    <p>Convert the format, resize, rotate, flip, make gray, compress, strip metadata, and combine several images into one PDF.</p>
    <ul class="chips" aria-label="Image formats"><li>PNG</li><li>JPEG</li><li>WebP</li><li>AVIF</li><li>GIF</li><li>BMP</li><li>TIFF</li><li>ICO</li></ul></div>
  <div class="card reveal" data-reveal><span class="tag">Video and audio</span><h3>MP4, MP3 and more</h3>
    <p>Convert and compress video, extract its audio, and convert audio between formats. Needs FFmpeg.</p>
    <ul class="chips" aria-label="Video and audio formats"><li>MP4</li><li>WebM</li><li>MKV</li><li>MOV</li><li>MP3</li><li>OGG</li><li>FLAC</li><li>WAV</li><li>M4A</li></ul></div>
  <div class="card reveal" data-reveal><span class="tag">Documents and PDF</span><h3>Office, ODF, PDF</h3>
    <p>Convert documents to PDF and between DOCX and ODT; compress, merge, rotate and split PDFs, and turn pages into images. Needs LibreOffice and Ghostscript.</p>
    <ul class="chips" aria-label="Document formats"><li>DOCX</li><li>ODT</li><li>XLSX</li><li>PPTX</li><li>ODP</li><li>PDF</li></ul></div>
</div>
<p class="section-intro">An action whose tool is not installed does not appear in the hive. Run <code>sbarhive doctor</code> to see what was found on your machine and what is missing.</p>
''', cls="alt s-formats", ident="formats", labelledby="formats-title", side="start")
    status = sec(f'''
<h2 id="status-title" class="reveal" data-reveal>Where the project stands, honestly</h2>
<div class="grid grid-3">
  <div class="card reveal" data-reveal><span class="tag tag-ok">Linux</span><h3>Tested</h3>
    <p>The hive runs on Hyprland, KDE Plasma 6 and GNOME 50, and the RPM and DEB packages install and run. Keyboard focus on real GNOME and KDE desktops is still being verified.</p></div>
  <div class="card reveal" data-reveal><span class="tag tag-warn">Windows</span><h3>Beta, unsigned</h3>
    <p>A per-user MSI that needs no administrator rights, a right click entry and a tray icon. It has not run on a live Windows machine yet, and SmartScreen warns because we have no signing certificate.</p></div>
  <div class="card reveal" data-reveal><span class="tag">Later</span><h3>Not here yet</h3>
    <p>X11, the drop hive and notifications on Windows, and macOS. No dates are promised; the plan is public in the repository.</p></div>
</div>
<p class="section-intro">We do not promise availability in any app store or package manager. What you see on the download page is what exists today.</p>
''', cls="s-status", scheme="dark", ident="status", labelledby="status-title", side="end")
    cta = sec(f'''
<div class="cta-card">
  <div>
    <h2 id="cta-title" class="reveal" data-reveal>Try it, and tell us what breaks</h2>
    <p class="lead">The first beta, for Linux and Windows. Download from the GitHub releases page and check it with the SHA256SUMS file.</p>
  </div>
  <div class="actions">
    <a class="button button-primary" href="{page_href(L, "index", L, "download")}">Download page</a>
    <a class="button" href="{REPO}">Source on GitHub</a>
  </div>
</div>
''', cls="s-cta", ident="get", labelledby="cta-title", side="start")
    compare = sec(f'''
<h2 id="compare-title" class="reveal" data-reveal>And the other tools?</h2>
<div class="grid grid-3">
  <div class="card reveal" data-reveal><h3>File Converter</h3><p>The same right click idea, but Windows only. SbarHive covers Linux and Windows with one app.</p></div>
  <div class="card reveal" data-reveal><h3>FFmpeg and ImageMagick</h3><p>They convert nearly anything from a terminal, if you remember their options. SbarHive uses FFmpeg for video and audio and puts the options in a cell.</p></div>
  <div class="card reveal" data-reveal><h3>File manager add ons</h3><p>Each is limited to images or to one job in one file manager. SbarHive handles images, video, audio and documents from a single menu.</p></div>
</div>
''', cls="alt s-compare", ident="compare", labelledby="compare-title")
    license_ = sec(f'''
<h2 id="license-title" class="reveal" data-reveal>Open source</h2>
<p class="section-intro">Licensed GPLv3 or any later version. Code, bug reports and contributions live on <a href="{REPO}">GitHub</a>. Adding a format takes a TOML file and a small adapter.</p>
''', cls="s-license", ident="license", labelledby="license-title", side="end")
    return hero + marquee + how + privacy + formats + status + cta + compare + license_


def marquee_block():
    codes = ["PNG", "JPG", "WEBP", "AVIF", "GIF", "PDF", "DOCX", "ODT", "MP4", "WEBM",
             "MKV", "MP3", "OGG", "FLAC", "WAV", "M4A", "XLSX", "PPTX", "TIFF", "ICO"]
    items = "".join(f"<li>{c}</li>" for c in codes)
    return (f'<div class="marquee sec" aria-hidden="true"><div class="hexedge"></div>'
            f'<ul class="track" dir="ltr">{items}{items}</ul></div>\n')


def stage(lang):
    ar = lang == "ar"
    t = {
        "steps": ["حدّد", "اختر", "خذ الناتج"] if ar else ["Select", "Pick", "Take it"],
        "pick": "ماذا حدّدت؟" if ar else "What did you select?",
        "folder": "المجلد" if ar else "Folder",
        "hint": "جرّب: اختر نوع الملف، ثم انقر على خلية" if ar else "Try it: choose a file kind, then click a cell",
        "hive_title": "الخلية السداسية: الإجراءات التي تناسب الملف المحدّد" if ar else "The hive: actions that fit the selected file",
        "demo": "عرض توضيحي" if ar else "Demo",
        "pause": "إيقاف الحركة" if ar else "Pause motion",
        "play": "تشغيل الحركة" if ar else "Play motion",
    }
    steps = "".join(f'<li data-step="{i}"><span class="n">{i + 1}</span>{s}</li>' for i, s in enumerate(t["steps"]))
    return f'''<div class="stage" id="stage" role="group" aria-label="{t["demo"]}">
  <ol class="stage-steps" aria-hidden="true">{steps}</ol>
  <div class="kinds" role="radiogroup" aria-label="{t["pick"]}" id="kinds"></div>
  <svg class="hive" id="hive" viewBox="{VIEW}" role="group" aria-label="{t["hive_title"]}">
    <g id="cells">
{static_hive(lang)}
    </g>
  </svg>
  <div class="tray" id="tray" aria-hidden="true">
    <img src="{rel_root(lang, "index")}favicon.svg" alt="" width="20" height="20">
    <span class="tray-name" id="tray-name"></span>
    <span class="bar"><span class="bar-in" id="tray-bar"></span></span>
  </div>
  <div class="folder"><p class="folder-title">{t["folder"]}</p><ul class="files" id="files"></ul></div>
  <p class="stage-hint" id="stage-hint">{t["hint"]}</p>
  <button type="button" class="motion-toggle" id="motion-toggle" aria-pressed="false" data-on="{t["pause"]}" data-off="{t["play"]}">{t["pause"]}</button>
  <p class="sr-only" id="stage-live" role="status" aria-live="polite"></p>
</div>'''


# ---------------------------------------------------------------------------
# Download, Windows, Privacy
# ---------------------------------------------------------------------------

def page_head(title, lead, extra=""):
    return (f'<section class="page-head sec"><div class="side side-end" aria-hidden="true"></div><div class="wrap">'
            f'<h1>{title}</h1><p class="lead">{lead}</p>{extra}</div></section>\n')


def download(lang):
    ar = lang == "ar"
    if ar:
        head_ = page_head("نزّل صبار هايف", f"الإصدار التجريبي الأول <code>{VERSION}</code> للينكس وويندوز. هذه نسخة بيتا: تعمل، وفيها عيوب معروفة نذكرها لك بوضوح.",
                          f'<p><span class="pill pill-hot">{VERSION}</span></p>')
        body = sec(f'''
<h2 id="d-title" class="reveal" data-reveal>اختر نظامك</h2>
<div class="grid grid-2">
  <div class="card reveal" data-reveal><span class="tag tag-ok">لينكس</span><h3>RPM وDEB، 64 بت</h3>
    <p>RPM لفيدورا وعائلتها، وDEB لأوبونتو وديبيان. اختبرنا فيدورا 41 وما بعدها، وأوبونتو 22.04 و24.04، وديبيان 12. الخلية تعمل على Wayland أولا.</p>
    <div class="actions"><a class="button button-primary" href="{RELEASES}">صفحة الإصدارات على GitHub</a></div></div>
  <div class="card reveal" data-reveal><span class="tag tag-warn">ويندوز</span><h3>MSI أو ZIP، ويندوز 10 و11 (64 بت)</h3>
    <p>MSI لكل مستخدم بلا صلاحيات المدير، أو ملف مضغوط محمول. البناء غير موقّع، فيظهر تحذير SmartScreen. <a href="{page_href("ar", "download", "ar", "windows")}">اقرأ ما يعنيه ذلك قبل التثبيت</a>.</p>
    <div class="actions"><a class="button button-primary" href="{RELEASES}">صفحة الإصدارات على GitHub</a></div></div>
</div>
<p class="section-intro">اختر أحدث إصدار تجريبي في الصفحة، ونزّل معه الملف <code>SHA256SUMS</code>. أسماء ملفات ويندوز على النمط <code>sbarhive-*-windows-x64.msi</code> و<code>sbarhive-*-windows-x64.zip</code>.</p>
''', cls="s-dl", ident="choose", labelledby="d-title", edge=False)
        verify = sec(f'''
<h2 id="v-title" class="reveal" data-reveal>تحقق من ملفك قبل تشغيله</h2>
<p class="section-intro">كل إصدار فيه ملف <code>SHA256SUMS</code>. نزّله في المجلد نفسه الذي فيه الحزم، ويجب أن يظهر OK أمام كل ملف نزّلته.</p>
<div class="split">
  <div class="reveal" data-reveal><h3>لينكس</h3>{code("sha256sum -c SHA256SUMS --ignore-missing")}</div>
  <div class="reveal" data-reveal><h3>ويندوز (PowerShell)</h3>{code("certutil -hashfile sbarhive-0.4.0-beta.1-windows-x64.msi SHA256")}
    <p class="fine">قارن الناتج بالسطر المقابل في <code>SHA256SUMS</code>. غيّر اسم الملف إلى ما نزّلته فعلا.</p></div>
</div>
<p class="section-intro">وإن اختلفت القيمة فلا تشغّل الملف، وأبلغنا في GitHub.</p>
''', cls="alt s-verify", ident="verify", labelledby="v-title", side="start")
        build = sec(f'''
<h2 id="b-title" class="reveal" data-reveal>أو ابنِه من الشيفرة على لينكس</h2>
<p class="section-intro">تحتاج Rust 1.85 أو أحدث. يثبّت السكربت البرنامج لحسابك وحده دون صلاحيات المدير، ويضيف صبار هايف إلى قائمة النقر بالزر الأيمن، ويشغّل الخدمة عند تسجيل الدخول.</p>
{code(f"git clone {REPO}" + chr(10) + "cd sbarhive" + chr(10) + "./scripts/install.sh")}
<ul class="install-notes">
  <li>يحتاج Nautilus إلى حزمة <code>nautilus-python</code>.</li>
  <li>للإزالة: <code>./scripts/uninstall.sh</code></li>
</ul>
<h2 id="u-title" class="reveal" data-reveal>التحديثات</h2>
<p class="section-intro">يفحص البرنامج مرة في اليوم وجود إصدار جديد من بيان موقّع على صفحة إصدارات GitHub، ويتحقق من التوقيع بمفتاح عام مدمج فيه قبل تنزيل أي شيء. تفاصيل أكثر في <a href="{page_href("ar", "download", "ar", "privacy")}">صفحة الخصوصية</a>.</p>
''', cls="s-build", ident="build", labelledby="b-title", side="end")
    else:
        head_ = page_head("Download SbarHive", f"The first beta, <code>{VERSION}</code>, for Linux and Windows. It works, and it has known rough edges that we list plainly.",
                          f'<p><span class="pill pill-hot">{VERSION}</span></p>')
        body = sec(f'''
<h2 id="d-title" class="reveal" data-reveal>Pick your system</h2>
<div class="grid grid-2">
  <div class="card reveal" data-reveal><span class="tag tag-ok">Linux</span><h3>RPM and DEB, 64 bit</h3>
    <p>RPM for Fedora and its family, DEB for Ubuntu and Debian. Tested on Fedora 41 and later, Ubuntu 22.04 and 24.04, and Debian 12. The hive targets Wayland first.</p>
    <div class="actions"><a class="button button-primary" href="{RELEASES}">GitHub releases page</a></div></div>
  <div class="card reveal" data-reveal><span class="tag tag-warn">Windows</span><h3>MSI or ZIP, Windows 10 and 11 (64 bit)</h3>
    <p>A per-user MSI that needs no administrator rights, or a portable zip. The build is unsigned, so SmartScreen warns you. <a href="{page_href("en", "download", "en", "windows")}">Read what that means before installing</a>.</p>
    <div class="actions"><a class="button button-primary" href="{RELEASES}">GitHub releases page</a></div></div>
</div>
<p class="section-intro">Pick the latest beta on that page and download <code>SHA256SUMS</code> with it. Windows files are named like <code>sbarhive-*-windows-x64.msi</code> and <code>sbarhive-*-windows-x64.zip</code>.</p>
''', cls="s-dl", ident="choose", labelledby="d-title", edge=False)
        verify = sec(f'''
<h2 id="v-title" class="reveal" data-reveal>Check your download before running it</h2>
<p class="section-intro">Every release lists a <code>SHA256SUMS</code> file. Put it in the folder with the packages; every file you downloaded must say OK.</p>
<div class="split">
  <div class="reveal" data-reveal><h3>Linux</h3>{code("sha256sum -c SHA256SUMS --ignore-missing")}</div>
  <div class="reveal" data-reveal><h3>Windows (PowerShell)</h3>{code("certutil -hashfile sbarhive-0.4.0-beta.1-windows-x64.msi SHA256")}
    <p class="fine">Compare the output with the matching line in <code>SHA256SUMS</code>. Use the file name you actually downloaded.</p></div>
</div>
<p class="section-intro">If the value differs, do not run the file, and tell us on GitHub.</p>
''', cls="alt s-verify", ident="verify", labelledby="v-title", side="start")
        build = sec(f'''
<h2 id="b-title" class="reveal" data-reveal>Or build it from source on Linux</h2>
<p class="section-intro">You need Rust 1.85 or newer. The script installs for your account only, without administrator rights, adds SbarHive to the right click menu and starts the service at login.</p>
{code(f"git clone {REPO}" + chr(10) + "cd sbarhive" + chr(10) + "./scripts/install.sh")}
<ul class="install-notes">
  <li>Nautilus needs the <code>nautilus-python</code> package.</li>
  <li>To remove it: <code>./scripts/uninstall.sh</code></li>
</ul>
<h2 id="u-title" class="reveal" data-reveal>Updates</h2>
<p class="section-intro">Once a day the app reads a signed manifest from the project's GitHub releases and verifies the signature against a public key built into the app before it downloads anything. More on the <a href="{page_href("en", "download", "en", "privacy")}">privacy page</a>.</p>
''', cls="s-build", ident="build", labelledby="b-title", side="end")
    return head_ + body + verify + build


def windows(lang):
    ar = lang == "ar"
    if ar:
        head_ = page_head("صبار هايف على ويندوز", "الإصدار التجريبي يعمل على ويندوز 10 و11 (64 بت)، لكنه غير موقّع. هذه الصفحة تشرح السبب، وتبيّن كيف تتأكد من سلامة ملفك قبل أن تضغط «تشغيل».")
        a = sec(f'''
<h2 id="w1" class="reveal" data-reveal>لماذا يظهر التحذير؟</h2>
<p class="section-intro">يعرض ويندوز SmartScreen الرسالة «Windows protected your PC» لأي برنامج غير موقّع بشهادة توقيع معتمدة. المشروع يطوّره شخص واحد بلا ميزانية، وليس لدينا شهادة توقيع شيفرة بعد. لا نعد بموعد أو بجهة توقيع محددة، ولا بوجود البرنامج في متجر تطبيقات.</p>
<ol class="steps grid grid-3" role="list">
  <li class="card reveal" data-reveal><h3>تحقق أولا</h3><p>قارن بصمة ملفك بما في <code>SHA256SUMS</code> كما في <a href="{page_href("ar", "windows", "ar", "download")}#verify">صفحة التنزيل</a>.</p></li>
  <li class="card reveal" data-reveal><h3>More info</h3><p>في نافذة التحذير اضغط «More info» لتظهر خيارات إضافية.</p></li>
  <li class="card reveal" data-reveal><h3>Run anyway</h3><p>اضغط «Run anyway» بعد أن تطابقت البصمة. وإن لم تتطابق فلا تشغّله.</p></li>
</ol>
''', cls="s-w1", ident="why", labelledby="w1", edge=False, side="start")
        b = sec(f'''
<h2 id="w2" class="reveal" data-reveal>ما الذي لم نتأكد منه بعد؟</h2>
<div class="grid grid-2">
  <div class="card reveal" data-reveal><h3>لم يُجرَّب على جهاز ويندوز حقيقي</h3><p>يُبنى على لينكس ويُختبر هناك. قائمة الفحص على جهاز ويندوز حقيقي هي الخطوة التالية، فقد تجد أشياء لا نعرفها.</p></div>
  <div class="card reveal" data-reveal><h3>ما ينقص حاليا</h3><p>لا خلية سحب وإفلات ولا إشعارات بعد، ويظهر تقدم المهام في تلميح أيقونة صينية النظام فقط. ولم نجرّب بعدُ تحديث MSI عند إغلاق التطبيق على جهاز حقيقي.</p></div>
  <div class="card reveal" data-reveal><h3>التثبيت</h3><p>حزمة MSI تثبَّت لحسابك بلا صلاحيات المدير، وتضيف بند «صبار هايف» إلى قائمة النقر بالزر الأيمن وأيقونة في صينية النظام. وهناك أيضا ملف مضغوط محمول.</p></div>
  <div class="card reveal" data-reveal><h3>التحديث</h3><p>يحدّث البرنامج نفسه بصمت لحسابك، بعد التحقق من التوقيع والحجم والبصمة. تستطيع إيقاف الفحص التلقائي من <code>config.toml</code>.</p></div>
</div>
<div class="actions"><a class="button button-primary" href="{page_href("ar", "windows", "ar", "download")}">إلى صفحة التنزيل</a></div>
''', cls="alt s-w2", ident="gaps", labelledby="w2", side="end")
    else:
        head_ = page_head("SbarHive on Windows", "The beta runs on Windows 10 and 11 (64 bit) but is not signed. This page explains why, and how to make sure your file is intact before you click Run.")
        a = sec(f'''
<h2 id="w1" class="reveal" data-reveal>Why the warning?</h2>
<p class="section-intro">Windows SmartScreen shows "Windows protected your PC" for any program without a signature from a paid certificate. One person builds this project with no budget, and we have no code signing certificate yet. We do not promise a date, a particular signing programme or a place in any app store.</p>
<ol class="steps grid grid-3" role="list">
  <li class="card reveal" data-reveal><h3>Verify first</h3><p>Compare your file's hash with <code>SHA256SUMS</code> as on the <a href="{page_href("en", "windows", "en", "download")}#verify">download page</a>.</p></li>
  <li class="card reveal" data-reveal><h3>More info</h3><p>In the warning window click "More info" to reveal more options.</p></li>
  <li class="card reveal" data-reveal><h3>Run anyway</h3><p>Click "Run anyway" only after the hash matched. If it did not, do not run it.</p></li>
</ol>
''', cls="s-w1", ident="why", labelledby="w1", edge=False, side="start")
        b = sec(f'''
<h2 id="w2" class="reveal" data-reveal>What is not verified yet?</h2>
<div class="grid grid-2">
  <div class="card reveal" data-reveal><h3>Not run on a live machine</h3><p>It is built on Linux and tested there. The live checklist on a real Windows machine is the next step, so you may find things we do not know about.</p></div>
  <div class="card reveal" data-reveal><h3>Missing for now</h3><p>No drop hive and no notifications yet, and job progress shows only in the tray tooltip. The helper that installs an MSI update while the app is closed has not run on a live machine.</p></div>
  <div class="card reveal" data-reveal><h3>Installing</h3><p>The MSI installs for your account without administrator rights and adds a SbarHive entry to the right click menu plus a tray icon. A portable zip is also available.</p></div>
  <div class="card reveal" data-reveal><h3>Updating</h3><p>The app updates itself quietly for your user after checking the signature, size and hash. You can turn the automatic check off in <code>config.toml</code>.</p></div>
</div>
<div class="actions"><a class="button button-primary" href="{page_href("en", "windows", "en", "download")}">Go to the download page</a></div>
''', cls="alt s-w2", ident="gaps", labelledby="w2", side="end")
    return head_ + a + b


def privacy(lang):
    ar = lang == "ar"
    if ar:
        head_ = page_head("الخصوصية", "لا تتبع، ولا حسابات، ولا خوادم معالجة. ملفاتك تبقى على جهازك. وهذا كل ما يرسله البرنامج عبر الشبكة، ولا شيء غير ذلك.")
        a = sec(f'''
<div class="split">
  <div>
    <h2 id="p1" class="reveal" data-reveal>ما لا يحدث أبدا</h2>
    <ul class="ticks" role="list">
      <li class="reveal" data-reveal>لا يُرفع ملف إلى أي خادم.</li>
      <li class="reveal" data-reveal>لا تحليلات ولا تتبع ولا تقارير أعطال تلقائية.</li>
      <li class="reveal" data-reveal>لا حساب ولا تسجيل دخول.</li>
    </ul>
  </div>
  <div class="reveal" data-reveal>{privacy_svg(lang)}</div>
</div>
''', cls="s-p1", ident="never", labelledby="p1", edge=False, side="start")
        b = sec(f'''
<h2 id="p2" class="reveal" data-reveal>الاتصال الوحيد: التحديثات</h2>
<div class="grid grid-2">
  <div class="card reveal" data-reveal><h3>فحص يومي</h3><p>مرة في اليوم يقرأ البرنامج بيانا موقّعا من صفحة إصدارات المشروع على GitHub. يرى GitHub عنوان الإنترنت (IP) الخاص بك ومعرّف البرنامج الثابت (User-Agent)، كما يرى أي تنزيل آخر.</p></div>
  <div class="card reveal" data-reveal><h3>تحقق قبل التنزيل</h3><p>يُتحقق من التوقيع بمفتاح عام مدمج في التطبيق قبل تنزيل أي شيء، ثم يُتحقق من كل حزمة بالحجم وبصمة SHA-256 الواردين في البيان.</p></div>
  <div class="card reveal" data-reveal><h3>قناتان</h3><p>قناة <code>beta</code> وقناة <code>stable</code>. النسخة التجريبية تتبع قناة beta، فتعرض عليك إصدارات تجريبية لاحقة ثم الإصدار النهائي.</p></div>
  <div class="card reveal" data-reveal><h3>إيقاف الفحص</h3><p>اضبط <code>update_check = false</code> في <code>config.toml</code>. الفحص اليدوي لا يتأثر بهذا الإعداد، لكن هذه البيتا ليس فيها أمر أو بند له بعد.</p></div>
</div>
''', cls="alt s-p2", ident="updates", labelledby="p2", side="end")
        c = sec(f'''
<h2 id="p3" class="reveal" data-reveal>الأصل لا يُمسّ</h2>
<p class="section-intro">يُكتب الناتج أولا في ملف مؤقت ثم تُغيَّر تسميته إلى اسمه النهائي في خطوة واحدة بجانب الأصل. لا يُعدَّل الملف الأصلي أبدا، وإن توقف العمل لا يبقى ملف ناقص باسم نهائي.</p>
<p class="section-intro">الشيفرة كلها مفتوحة برخصة GPLv3 على <a href="{REPO}">GitHub</a>، فتستطيع التحقق بنفسك.</p>
''', cls="s-p3", ident="originals", labelledby="p3", side="start")
    else:
        head_ = page_head("Privacy", "No tracking, no accounts, no processing servers. Your files stay on your machine. Below is everything the app sends over the network, with no hidden exception.")
        a = sec(f'''
<div class="split">
  <div>
    <h2 id="p1" class="reveal" data-reveal>What never happens</h2>
    <ul class="ticks" role="list">
      <li class="reveal" data-reveal>No file is uploaded to any server.</li>
      <li class="reveal" data-reveal>No analytics, no tracking, no automatic crash reports.</li>
      <li class="reveal" data-reveal>No account and no sign in.</li>
    </ul>
  </div>
  <div class="reveal" data-reveal>{privacy_svg(lang)}</div>
</div>
''', cls="s-p1", ident="never", labelledby="p1", edge=False, side="start")
        b = sec(f'''
<h2 id="p2" class="reveal" data-reveal>The only connection: updates</h2>
<div class="grid grid-2">
  <div class="card reveal" data-reveal><h3>A daily check</h3><p>Once a day the app reads a signed manifest from the project's GitHub releases. GitHub sees your address and a fixed user agent, like any other download.</p></div>
  <div class="card reveal" data-reveal><h3>Verified before download</h3><p>The signature is checked against a public key built into the app before anything is downloaded, and each package is checked against the size and SHA-256 in the manifest.</p></div>
  <div class="card reveal" data-reveal><h3>Two channels</h3><p>A <code>beta</code> and a <code>stable</code> channel. A beta build follows <code>beta</code>, so you are offered later betas and then the final release.</p></div>
  <div class="card reveal" data-reveal><h3>Turning it off</h3><p>Set <code>update_check = false</code> in <code>config.toml</code>. A manual check ignores that setting, but this beta has no menu entry or command for it yet.</p></div>
</div>
''', cls="alt s-p2", ident="updates", labelledby="p2", side="end")
        c = sec(f'''
<h2 id="p3" class="reveal" data-reveal>The original is never touched</h2>
<p class="section-intro">The result is written to a temporary file first and then atomically renamed to its final name next to the original. The original is never modified, and a stopped job leaves no half written file under a final name.</p>
<p class="section-intro">All the code is open under the GPLv3 on <a href="{REPO}">GitHub</a>, so you can check for yourself.</p>
''', cls="s-p3", ident="originals", labelledby="p3", side="start")
    return head_ + a + b + c


def render(lang, page):
    root = rel_root(lang, page)
    if page == "index":
        body = home_ar(root) if lang == "ar" else home_en(root)
    elif page == "download":
        body = download(lang)
    elif page == "windows":
        body = windows(lang)
    else:
        body = privacy(lang)
    return head(lang, page, root) + header(lang, page, root) + body + footer(lang, page, root)


def main():
    for lang in ("ar", "en"):
        for page in PAGES:
            parts = (["en"] if lang == "en" else []) + ([] if page == "index" else [page])
            out = HERE.joinpath(*parts, "index.html")
            out.parent.mkdir(parents=True, exist_ok=True)
            out.write_text(render(lang, page), encoding="utf-8")
            print("wrote", out.relative_to(HERE.parent))


if __name__ == "__main__":
    main()
