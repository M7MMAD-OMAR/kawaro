# SPDX-License-Identifier: GPL-3.0-or-later
# S-112 helper: static checks of the AppStream metainfo. Reads files only.
# Usage: python3 -I s112_appstream_check.py METAINFO REPO_ROOT icon|screenshots|releases
# Prints one line per problem and exits 1 when there is any.
import re
import struct
import sys
import xml.etree.ElementTree as ET

XML_LANG = "{http://www.w3.org/XML/1998/namespace}lang"
ARABIC = re.compile("[؀-ۿ]")
LONG_DASH = re.compile("[" + chr(0x2013) + chr(0x2014) + "]")
ARABIC_INDIC = re.compile("[٠-٩۰-۹]")


def png_size(path):
    with open(path, "rb") as f:
        head = f.read(24)
    if head[:8] != b"\x89PNG\r\n\x1a\n" or head[12:16] != b"IHDR":
        raise ValueError(f"{path}: not a PNG")
    return struct.unpack(">II", head[16:24])


def text(el):
    return " ".join("".join(el.itertext()).split())


def check_icon(root, repo, errs):
    icons = root.findall("icon")
    if len(icons) != 1:
        errs.append(f"want exactly 1 icon element, found {len(icons)}")
        return
    ic = icons[0]
    if ic.get("type") != "stock" or (ic.text or "").strip() != "kawaro":
        errs.append("the icon must be <icon type=\"stock\">kawaro</icon>")


def check_screenshots(root, repo, errs):
    shots = root.findall("screenshots/screenshot")
    if len(shots) != 2:
        errs.append(f"want exactly 2 screenshots, found {len(shots)}")
        return
    if shots[0].get("type") != "default":
        errs.append("the first screenshot must have type=\"default\"")
    if any(s.get("type") == "default" for s in shots[1:]):
        errs.append("only the first screenshot may be the default")
    files = set()
    for i, s in enumerate(shots):
        caps = s.findall("caption")
        en = [c for c in caps if c.get(XML_LANG) is None and text(c)]
        ar = [c for c in caps if c.get(XML_LANG) == "ar" and ARABIC.search(text(c))]
        if len(en) != 1:
            errs.append(f"screenshot {i}: want one non empty English caption")
        if len(ar) != 1:
            errs.append(f"screenshot {i}: want one Arabic caption with Arabic letters")
        imgs = [im for im in s.findall("image") if im.get("type") == "source"]
        if len(imgs) != 1:
            errs.append(f"screenshot {i}: want exactly one <image type=\"source\">")
            continue
        url = (imgs[0].text or "").strip()
        m = re.fullmatch(r"https://kawaro\.sbarah\.com/screenshots/([A-Za-z0-9._-]+\.png)", url)
        if not m:
            errs.append(f"screenshot {i}: bad URL {url!r}")
            continue
        files.add(m.group(1))
        try:
            w, h = png_size(f"{repo}/packaging/screenshots/{m.group(1)}")
        except (OSError, ValueError) as e:
            errs.append(f"screenshot {i}: {e}")
            continue
        if (imgs[0].get("width"), imgs[0].get("height")) != (str(w), str(h)):
            errs.append(f"screenshot {i}: width/height {imgs[0].get('width')}x{imgs[0].get('height')} but the PNG is {w}x{h}")
    if files != {"hive-en-light.png", "hive-ar-dark.png"}:
        errs.append(f"the screenshot files must be hive-en-light.png and hive-ar-dark.png, got {sorted(files)}")


def check_releases(root, repo, errs):
    rels = root.findall("releases/release")
    if not rels:
        errs.append("no <release> entries")
    for r in rels:
        v = r.get("version")
        d = r.find("description")
        if d is None:
            errs.append(f"release {v}: no description")
            continue
        ps = d.findall("p")
        en = [p for p in ps if p.get(XML_LANG) is None and text(p)]
        ar = [p for p in ps if p.get(XML_LANG) == "ar"]
        if not en:
            errs.append(f"release {v}: no English paragraph")
        if not ar:
            errs.append(f"release {v}: no Arabic paragraph")
        for p in ar:
            t = text(p)
            if not ARABIC.search(t):
                errs.append(f"release {v}: the Arabic paragraph has no Arabic letters")
            if ARABIC_INDIC.search(t):
                errs.append(f"release {v}: Arabic-Indic digits, use Western digits")
        for p in ps:
            if LONG_DASH.search(text(p)):
                errs.append(f"release {v}: long dash")
        if len(ar) != len(en):
            errs.append(f"release {v}: {len(en)} English and {len(ar)} Arabic paragraphs")


def main():
    path, repo, what = sys.argv[1:4]
    errs = []
    root = ET.parse(path).getroot()
    {"icon": check_icon, "screenshots": check_screenshots, "releases": check_releases}[what](root, repo, errs)
    for e in errs:
        print(e)
    return 1 if errs else 0


sys.exit(main())
