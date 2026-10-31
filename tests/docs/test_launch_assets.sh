#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-072 launch assets: static checks of AC1 to AC8 in backlog/S-072-launch-assets.md.
# No network, no build. Run from anywhere: tests/docs/test_launch_assets.sh
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT" || exit 2
python3 -I - "$ROOT" <<'PY'
import hashlib, os, re, struct, subprocess, sys, unicodedata

root = sys.argv[1]
os.chdir(root)
fails = 0


def check(name, ok, detail=""):
    global fails
    print(("PASS " if ok else "FAIL ") + name + ("" if ok or not detail else ": " + detail))
    if not ok:
        fails += 1


def read(path):
    try:
        with open(path, encoding="utf-8") as f:
            return f.read()
    except OSError:
        return None


def sha(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def png_size(path):
    with open(path, "rb") as f:
        d = f.read(24)
    if d[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", d[16:24])


def strip_marks(s):
    return "".join(c for c in unicodedata.normalize("NFD", s) if unicodedata.category(c) != "Mn")


def tables(text):
    """Markdown tables as (header, rows); cells stripped, backticks removed."""
    out, cur = [], []
    for line in (text or "").splitlines() + [""]:
        if line.strip().startswith("|"):
            cur.append([c.strip().strip("`").strip() for c in line.strip().strip("|").split("|")])
        else:
            if len(cur) >= 2:
                header = [h.lower() for h in cur[0]]
                rows = [r for r in cur[2:]]
                out.append((header, rows))
            cur = []
    return out


def sections(text):
    """Split on H2 headings: list of (heading, body)."""
    parts = re.split(r"(?m)^## +(.*)$", text)
    return [(parts[i].strip(), parts[i + 1].strip()) for i in range(1, len(parts), 2)]


KIT = [
    "docs/launch/assets.md",
    "docs/launch/announcement.md",
    "docs/launch/announcement.ar.md",
    "docs/launch/owner-steps.md",
]
kit_text = {p: read(p) for p in KIT}

# ---------------------------------------------------------------- AC1
assets = kit_text["docs/launch/assets.md"] or ""
asset_rows = []
for header, rows in tables(assets):
    if all(k in header for k in ("file", "source", "language", "theme")):
        for r in rows:
            asset_rows.append(dict(zip(header, r)))
check("AC1 manifest exists with an assets table", bool(asset_rows))
ALLOWED = (
    "design/brand/",
    "docs/screenshots/",
    "docs/diagrams/",
    "site/og.png",
    "site/media/hero-",
)
check("AC1 every listed file exists", all(os.path.isfile(r["file"]) for r in asset_rows) and bool(asset_rows),
      str([r["file"] for r in asset_rows if not os.path.isfile(r["file"])]))
check("AC1 every row names an existing source", all(os.path.isfile(r["source"]) for r in asset_rows) and bool(asset_rows),
      str([r["source"] for r in asset_rows if not os.path.isfile(r["source"])]))
check("AC1 every source is on the allowed list", all(r["source"].startswith(ALLOWED) for r in asset_rows) and bool(asset_rows))
files = [r["file"] for r in asset_rows]
check("AC1 one row per file, no duplicates", len(files) == len(set(files)) and bool(files))
check("AC1 language is en, ar or none; theme is light, dark or none",
      all(r["language"] in ("en", "ar", "none") and r["theme"] in ("light", "dark", "none") for r in asset_rows) and bool(asset_rows))
check("AC1 every kit file exists", all(kit_text[p] is not None for p in KIT),
      str([p for p in KIT if kit_text[p] is None]))

# ---------------------------------------------------------------- AC2
LOGOS = {
    "design/brand/kawaro-cells.svg",
    "design/brand/kawaro-cells-dark.svg",
    "design/brand/wordmark.svg",
    "design/brand/wordmark-dark.svg",
}
brand_rows = [r for r in asset_rows if r["file"].startswith("design/brand/")]
check("AC2 kit uses only the allowed logo files",
      bool(brand_rows) and all(r["file"] in LOGOS or re.fullmatch(r"design/brand/png/kawaro-\d+\.png", r["file"]) for r in brand_rows))
check("AC2 the four logo SVGs are all listed", LOGOS <= {r["file"] for r in brand_rows})
check("AC2 manifest SHA-256 equals the file for every row",
      bool(asset_rows) and all(os.path.isfile(r["file"]) and r.get("sha256", "") == sha(r["file"]) for r in asset_rows))
readme_en, readme_ar = read("README.md") or "", read("README.ar.md") or ""
check("AC2 README.md names the saguaro cactus built from hive cells",
      "saguaro" in readme_en.lower() and "cells" in readme_en.lower())
ar_plain = strip_marks(readme_ar)
check("AC2 README.ar.md names the cactus built from hive cells", "صبار" in ar_plain and "خلايا" in ar_plain)
check("AC2 the word domed is absent from both READMEs",
      "domed" not in readme_en.lower() and "domed" not in readme_ar.lower() and "مقبب" not in ar_plain)

# ---------------------------------------------------------------- AC3
shots = [r for r in asset_rows if r["file"].startswith("docs/screenshots/")]
def lang_ok(lang):
    rs = [r for r in shots if r["language"] == lang]
    return len(rs) >= 2 and {"light", "dark"} <= {r["theme"] for r in rs}
check("AC3 at least 4 screenshots", len(shots) >= 4)
check("AC3 English: 2 or more, light and dark", lang_ok("en"))
check("AC3 Arabic: 2 or more, light and dark", lang_ok("ar"))
check("AC3 every screenshot is a PNG at least 880 px wide",
      bool(shots) and all(os.path.isfile(r["file"]) and (png_size(r["file"]) or (0, 0))[0] >= 880 for r in shots))
check("AC3 every screenshot is its own source (same SHA-256, no edited copy)",
      bool(shots) and all(os.path.isfile(r["source"]) and sha(r["source"]) == sha(r["file"]) == r.get("sha256") for r in shots))
def produced_ok(r):
    p = r.get("produced by", "")
    paths = re.findall(r"[\w./-]+\.(?:sh|rs|py)\b", p)
    return bool(paths) and all(os.path.isfile(x) for x in paths) and re.search(r"\bS-\d+[a-z]?\b", p) is not None
check("AC3 each screenshot names an existing script and a story", bool(shots) and all(produced_ok(r) for r in shots))
check("AC3 Arabic screenshots are the Arabic ones by name", all("-ar-" in r["file"] for r in shots if r["language"] == "ar"))

# ---------------------------------------------------------------- AC4
cargo = read("Cargo.toml") or ""
m = re.search(r"(?ms)^\[workspace\.package\].*?^version\s*=\s*\"([^\"]+)\"", cargo)
ver = m.group(1) if m else None
check("AC4 Cargo.toml version found", ver is not None)
SWITCH = "[English](README.md) | [العربية](README.ar.md)"
check("AC4 both READMEs start with the language switch line",
      readme_en.splitlines()[:1] == [SWITCH] and readme_ar.splitlines()[:1] == [SWITCH])
h2en = re.findall(r"(?m)^## ", readme_en)
h2ar = re.findall(r"(?m)^## ", readme_ar)
check("AC4 same number of H2 headings", len(h2en) == len(h2ar), f"{len(h2en)} vs {len(h2ar)}")


def named_table(text, first_col_hints):
    for header, rows in tables(text):
        if any(h in first_col_hints for h in header[:1]):
            return header, rows
    return None, []


_, dl_en = named_table(readme_en, ("system",))
_, dl_ar = named_table(readme_ar, ("النظام",))
check("AC4 download rows: same count", len(dl_en) == len(dl_ar) and len(dl_en) > 0, f"{len(dl_en)} vs {len(dl_ar)}")
dl_en_raw = [r for r in tables(readme_en) if r[0][:1] == ["system"]]
dl_ar_raw = [r for r in tables(readme_ar) if r[0][:1] == ["النظام"]]
check("AC4 download rows: same files in the same order",
      bool(dl_en_raw) and bool(dl_ar_raw) and [r[1] for r in dl_en_raw[0][1]] == [r[1] for r in dl_ar_raw[0][1]])
plat_en = [t for t in tables(readme_en) if t[0][:1] == ["platform"]]
plat_ar = [t for t in tables(readme_ar) if t[0][:1] == ["النظام"] and len(t[0]) == 3 and t[0][1] == "الاختبار"]


def state(cell, lang):
    c = strip_marks(cell).lower()
    if lang == "en":
        for key, v in (("not supported", "unsupported"), ("not tested", "untested"), ("virtual", "virtual"), ("tested", "tested")):
            if key in c:
                return v
    else:
        for key, v in (("غير مدعوم", "unsupported"), ("لم يجرب", "untested"), ("افتراض", "virtual"), ("جرب", "tested")):
            if key in c:
                return v
    return "?"


check("AC4 platform table rows: same count", bool(plat_en) and bool(plat_ar) and len(plat_en[0][1]) == len(plat_ar[0][1]))
check("AC4 platform table rows: same tested state per row",
      bool(plat_en) and bool(plat_ar) and len(plat_en[0][1]) == len(plat_ar[0][1])
      and [state(r[1], "en") for r in plat_en[0][1]] == [state(r[1], "ar") for r in plat_ar[0][1]]
      and "?" not in [state(r[1], "en") for r in plat_en[0][1]])
check("AC4 README.md release line equals Cargo.toml", ver is not None and f"Current release: {ver}." in readme_en)
check("AC4 README.ar.md release line equals Cargo.toml", ver is not None and f"الإصدار الحالي: {ver}." in readme_ar)
check("AC4 no other release number in either README release line",
      not re.search(r"Current release: (?!%s\b)\d" % re.escape(ver or "x"), readme_en)
      and not re.search(r"الإصدار الحالي: (?!%s\b)\d" % re.escape(ver or "x"), readme_ar))

og = png_size("site/og.png")
check("AC4 site/og.png is 1200 x 630", og == (1200, 630), str(og))
hero = {}
for page, lang in (("site/index.html", "ar"), ("site/en/index.html", "en")):
    html = read(page) or ""
    sec = re.search(r'(?s)<section class="hero".*?</section>', html)
    sec = sec.group(0) if sec else ""
    h1 = re.search(r"(?s)<h1[^>]*>(.*?)</h1>", sec)
    lead = re.search(r'(?s)<p class="lead">(.*?)</p>', sec)
    text = re.sub(r"<[^>]+>", " ", (h1.group(1) if h1 else "") + " " + (lead.group(1) if lead else ""))
    hero[lang] = re.sub(r"\s+", " ", text).strip()
    media = re.findall(r"(?s)<(video|img)\b([^>]*)>", sec)
    ok = bool(media)
    for tag, attrs in media:
        if tag == "video":
            ok = ok and re.search(r'aria-label="[^"]+"', attrs) is not None
        else:
            ok = ok and re.search(r'\balt="[^"]+"', attrs) is not None
    check(f"AC4 {page} hero media has a non-empty alt or aria-label", ok)
h_en, h_ar = hero.get("en", "").lower(), strip_marks(hero.get("ar", ""))
check("AC4 English hero says convert, locally, next to the original",
      "convert" in h_en and "your machine" in h_en and "next to the original" in h_en, h_en)
check("AC4 Arabic hero says convert, locally, next to the original",
      "حول" in h_ar and "جهازك" in h_ar and "بجانب الاصل" in strip_marks(h_ar).replace("الأصل", "الاصل"), h_ar)

# ---------------------------------------------------------------- AC5
claims = []
for header, rows in tables(assets):
    if "claim" in header and "evidence" in header:
        for r in rows:
            claims.append(dict(zip(header, r)))
check("AC5 claims table exists", bool(claims))


def evidence_paths(cell):
    toks = re.findall(r"[\w./-]+/[\w./-]+|[\w-]+\.(?:md|sh|rs|toml|html|py|png|svg|json)\b", cell)
    return toks


check("AC5 every claim has an evidence path and every path exists",
      bool(claims) and all(evidence_paths(c["evidence"]) and all(os.path.exists(p) for p in evidence_paths(c["evidence"])) for c in claims),
      str([c["claim"] for c in claims if not evidence_paths(c["evidence"]) or not all(os.path.exists(p) for p in evidence_paths(c["evidence"]))]))
claims_blob = " ".join(c["claim"] for c in claims).lower()

PLATFORMS_EN = ["x11", "windows", "macos", "winget", "linux", "wayland", "gnome", "kde", "hyprland", "scoop", "apt", "aur", "copr", "rpm", "deb", "smartscreen"]
PLATFORMS_AR = {"ويندوز": "windows", "لينكس": "linux", "ماك": "macos", "سمارت": "smartscreen"}
FORMATS = ["webp", "png", "jpg", "avif", "pdf", "mp4", "docx", "gif", "mp3", "flac", "wav"]

texts = {
    "announcement.md": (kit_text["docs/launch/announcement.md"] or "", "en"),
    "announcement.ar.md": (kit_text["docs/launch/announcement.ar.md"] or "", "ar"),
    "hero en": (hero.get("en", ""), "en"),
    "hero ar": (hero.get("ar", ""), "ar"),
}
missing = []
for name, (t, lang) in texts.items():
    body = "\n".join(l for l in t.splitlines() if not l.startswith("[English]"))
    body = re.sub(r"https?://\S+|\(\S+?\.md\)|`[^`]*\.(?:md|sh)`", " ", body)
    for n in re.findall(r"\d+(?:\.\d+)*", body):
        if n not in claims_blob:
            missing.append((name, n))
    low = body.lower()
    for p in PLATFORMS_EN:
        if re.search(r"(?<![a-z0-9])" + p + r"(?![a-z0-9])", low) and p not in claims_blob:
            missing.append((name, p))
    for f in FORMATS:
        if re.search(r"(?<![a-z0-9])" + f + r"(?![a-z0-9])", low) and f not in claims_blob:
            missing.append((name, f))
    for ar, en in PLATFORMS_AR.items():
        if ar in body and en not in claims_blob:
            missing.append((name, en))
check("AC5 every number, platform and format maps to a claims row", not missing and any(t for t, _ in texts.values()), str(missing))

BANNED = [r"testimonial", r"trusted by", r"users love", r"\bstars?\b", r"\bratings?\b", r"\bfastest\b", r"\bbest\b", r"\bdownloads? count"]
banned_hits = []
for name, (t, lang) in texts.items():
    for b in BANNED:
        if re.search(b, t.lower()):
            banned_hits.append((name, b))
check("AC5 no banned marketing words", not banned_hits and any(t for t, _ in texts.values()), str(banned_hits))
ms_bad = []
for name, (t, lang) in texts.items():
    for line in t.splitlines():
        for mm in re.finditer(r"(\d+)\s*ms\b", line):
            if mm.group(1) != "150" or not re.search(r"budget", line.lower()):
                ms_bad.append((name, line.strip()[:60]))
check("AC5 the only timing is the 150 ms budget, worded as a budget", not ms_bad and any(t for t, _ in texts.values()), str(ms_bad))
en_ann = texts["announcement.md"][0]
platform_wording = True
if en_ann:
    low = en_ann.lower()
    if "x11" in low:
        platform_wording &= "virtual" in low or "private" in low
    if "macos" in low:
        platform_wording &= "not supported yet" in low
    if "winget" in low:
        platform_wording &= "waiting for review" in low or "waits for review" in low
    if "windows" in low:
        platform_wording &= "virtual machine" in low or "smartscreen" in low
check("AC5 platform wording matches README Platform support", bool(en_ann) and platform_wording)

# ---------------------------------------------------------------- AC6
for path, lang in (("docs/launch/announcement.md", "en"), ("docs/launch/announcement.ar.md", "ar")):
    t = kit_text[path]
    tag = f"AC6 {path}"
    if t is None:
        check(tag + " exists", False)
        continue
    want = "[English](announcement.md) | [العربية](announcement.ar.md)"
    check(tag + " starts with the language switch line", t.splitlines()[:1] == [want])
    secs = sections(t)
    check(tag + " has exactly 3 H2 parts", len(secs) == 3, str(len(secs)))
    if len(secs) == 3:
        rel, ren, post = secs[0][1], secs[1][1], secs[2][1]
        n = len([l for l in rel.splitlines() if l.strip()])
        check(tag + " release note is 5 to 8 lines", 5 <= n <= 8, str(n))
        paras = [p for p in re.split(r"\n\s*\n", ren) if p.strip()]
        check(tag + " rename note is one paragraph", len(paras) == 1, str(len(paras)))
        plain = strip_marks(ren)
        if lang == "en":
            check(tag + " rename note: old name, new name, mark unchanged, meaning",
                  "SbarHive" in ren and "Kawaro" in ren and re.search(r"mark (did not|has not|is not|stays|is unchanged)|did not change|unchanged", ren)
                  and "كوارة" in plain)
        else:
            check(tag + " rename note: old name, new name, mark unchanged, meaning",
                  "SbarHive" in ren and "كوارو" in plain and "كوارة" in plain and ("لم يتغير" in plain or "لم تتغير" in plain or "ثابت" in plain or "كما هو" in plain))
        check(tag + " post is at most 280 characters", 0 < len(post.strip()) <= 280, str(len(post.strip())))
    gaps = ("known gaps" in t.lower()) if lang == "en" else ("الثغرات المعروفة" in strip_marks(t))
    check(tag + " has the known gaps line", gaps)
    check(tag + " names SmartScreen and the X11 private session",
          "SmartScreen" in t and "X11" in t and re.search(r"virtual|private|افتراض", strip_marks(t)) is not None)
    check(tag + " has no Arabic-Indic digits", re.search("[٠-٩۰-۹]", t) is None)
    check(tag + " has no guillemets", "«" not in t and "»" not in t)

# ---------------------------------------------------------------- AC7
scan = [p for p in KIT if kit_text[p] is not None] + ["README.md", "README.ar.md"]
bad = []
for p in scan:
    t = read(p) or ""
    t2 = t.replace("M7MMAD-OMAR", "").replace("m7mmad_omar", "")
    if "/home/" in t: bad.append((p, "home path"))
    if re.search(r"[\w.+-]+@[\w-]+\.[\w.-]+", t): bad.append((p, "email"))
    if re.search(r"(?<![\w])\+\d[\d ()-]{7,}\d", t): bad.append((p, "phone"))
    if re.search(r"muhmad|mohammad|mohamed|haj hmdo|محمد|حاج حمدو", t2, re.I): bad.append((p, "personal name"))
    if "–" in t or "—" in t: bad.append((p, "long dash"))
check("AC7 no home path, email, phone, personal name or long dash", not bad and all(kit_text[p] is not None for p in KIT), str(bad))
r = subprocess.run(["bash", "scripts/design-lint.sh"], capture_output=True, text=True)
check("AC7 scripts/design-lint.sh passes", r.returncode == 0, r.stdout[-300:])
story = read("backlog/S-072-launch-assets.md") or ""
check("AC7 review log records the by eye screenshot check with a date",
      re.search(r"(?i)by eye[^\n]*\d{4}-\d{2}-\d{2}|\d{4}-\d{2}-\d{2}[^\n]*by eye", story) is not None)

# ---------------------------------------------------------------- AC8
steps = kit_text["docs/launch/owner-steps.md"]
if steps is None:
    check("AC8 owner-steps.md exists", False)
else:
    low = steps.lower()
    check("AC8 names the posting, accounts, social preview, AUR and COPR steps",
          all(k in low for k in ("announcement", "account", "social preview", "aur", "copr")))
    check("AC8 each step points at a text file to paste", "docs/launch/announcement.md" in steps and "docs/launch/announcement.ar.md" in steps)
    check("AC8 no past tense claim about posting",
          re.search(r"\b(was|were|has been|have been|already) (posted|published|sent)\b|\b(we|i|maintainer) (posted|published)\b|\bposted\b", low) is None)

print("failures: %d" % fails)
sys.exit(1 if fails else 0)
PY
