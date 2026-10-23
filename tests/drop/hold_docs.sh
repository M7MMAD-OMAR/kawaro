#!/usr/bin/env bash
# S-019e docs (backlog/S-019-hive-at-pointer.md, AC22 and AC23, amended
# 2026-10-21). No compositor; a static check for the reviewer, who still
# reads the text (and AC16, which names no words to look for). A section is
# the text between two headings (any level); it is Arabic when its prose
# (not code) has more Arabic than Latin letters, else English. Arabic is
# compared without harakat, with أ إ آ read as ا, and with Arabic-Indic
# digits read as 0 to 9.
#   AC22  integrations/linux/hyprland/README.md, in an Arabic and in an
#         English section each: a hyprlang press bind that runs
#         `sbarhive drop --hold` with the non consuming flag n (bindn...), a
#         hyprlang release bind that runs `sbarhive drop --release` with the
#         flags r, n and i (bindrni, bindnir...), a Lua `hl.bind(` that runs
#         `drop --hold` and one that runs `drop --release` with
#         `non_consuming = true`, the words
#         `ignore_mods`, and a note that the binds are not verified yet
#         (English "not verified", "unverified" or "assumed"; Arabic
#         "مفترض" or "غير متحقق").
#   AC23  README.md: its Arabic and its English settings section each name
#         `hold_ms` with 400, 100 and 2000; docs/architecture.md names
#         `hold_ms` and `drop --hold`; docs/adr/0003-drop-hive.md names
#         `hold_ms` and 150; CHANGELOG.md has one line that names `--hold`
#         or `hold_ms` in English and in Arabic (one line per story, as the
#         file's other entries are written).
#   S-021 AC12 (backlog/S-021-drop-only-on-drag.md), on the same sections:
#     integrations/linux/hyprland/README.md, in one Arabic and in one English
#     section each, three sentences (a sentence ends at . ! ? or a line
#     end; inline code is left out):
#       only      the hive shows only during a drag of files with an action:
#                 English "only" + "drag..." + "action"; Arabic "فقط" or
#                 "الا" + "سحب" + "اجراء";
#       first     the order, start the drag and then hold the key: English
#                 "start" or "begin" + "drag..." + "then", "first" or
#                 "before" + "hold" or "press"; Arabic "ابدا" + "سحب" +
#                 "ثم", "اولا" or "قبل" + "مسك" or "ضغط";
#       nothing   with no drag, or for a file with no action, nothing shows:
#                 English "without" or "no" + "drag" + "no action(s)",
#                 "without an action" or "with no action" + "nothing",
#                 "never" or "does not show|appear"; Arabic "دون سحب",
#                 "بلا سحب" or "من غير سحب" + "بلا اجراء", "دون اجراء",
#                 "ليس له اجراء" or "لا اجراء" + "لا يظهر", "لا تظهر" or
#                 "لا شيء".
#     docs/architecture.md: one section names the `Probing` state, the
#     `hive reveal` and `hive probe` lines, a hidden catcher ("hidden" and
#     "catcher" in one sentence) and all six hide reasons as words:
#     pointer, no-uri-list, no-actions, release, cap and replaced (S-022
#     AC9 added replaced).
#   S-021 AC11: CHANGELOG.md has one line that names S-021 (or S-021a, b,
#   c) with Arabic and English text, as the file's other entries are
#   written; it was under "## [Unreleased]" until the 0.3.1 release.
#   S-022 AC9 (backlog/S-022-drop-followups.md): that same section of
#   docs/architecture.md also says the opening animation starts at the
#   reveal ("open" or "opening", then "animation", and "reveal" within 120
#   characters either way, line ends included); CHANGELOG.md has one line
#   that names S-022 with Arabic and English text (it was under
#   "## [Unreleased]" until the 0.3.2 release).
#   S-022 AC10: no prose line of tests/drop/README.md is longer than 80
#   columns (characters); lines inside ``` fences are not counted.
#   Consuming release binds: no hyprlang `bind...r...` line without the flag
#   n, and no Lua bind with `release = true` without `non_consuming = true`,
#   in any .md outside backlog/. Hyprland swallows the press of a key whose
#   release bind is consuming, and the release bind ignores modifiers, so a
#   documented `bindri = SUPER, Z` swallowed every Z typed anywhere and one
#   on ALT_L every Alt (2026-10-23, KeybindManager.cpp in Hyprland 0.56.2).
#   No em dash or en dash in any of these files, nor in tests/drop/README.md.
#   Language pairs: each user doc is English NAME.md with an Arabic copy
#   NAME.ar.md beside it. An Arabic check above reads the .ar.md copy (the
#   Hyprland README.ar.md, README.ar.md), an English check the English file.
#   README.md and CONTRIBUTING.md each have both files, and both start with
#   the switch line "[English](NAME.md) | [العربية](NAME.ar.md)".
# Usage: tests/drop/hold_docs.sh
# shellcheck disable=SC2016,SC2034
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
FAILS=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }

HYPR="$ROOT/integrations/linux/hyprland/README.md"
HYPR_AR="$ROOT/integrations/linux/hyprland/README.ar.md"
README="$ROOT/README.md"
README_AR="$ROOT/README.ar.md"
ARCH="$ROOT/docs/architecture.md"
ADR="$ROOT/docs/adr/0003-drop-hive.md"
LOG="$ROOT/CHANGELOG.md"

# sections FILE: prints "LANG KEY" for every key a section satisfies.
sections() {
  python3 -I - "$1" <<'PY'
import re, sys
AR = re.compile(r"[؀-ۿ]")
LAT = re.compile(r"[A-Za-z]")
DIAC = re.compile(r"[\u064b-\u0652\u0670]")
HAMZA = str.maketrans("أإآ" + "٠١٢٣٤٥٦٧٨٩" + "۰۱۲۳۴۵۶۷۸۹", "ااا" + "0123456789" * 2)
text = DIAC.sub("", open(sys.argv[1], encoding="utf-8").read()).translate(HAMZA)
parts = re.split(r"(?m)^#{1,6} .*$", text)
heads = re.findall(r"(?m)^#{1,6} .*$", text)
KEYS = {
    "press-hyprlang": re.compile(r"^\s*bind[a-z]*n[a-z]*\s*=.*\bexec\s*,.*\bsbarhive drop --hold\b", re.M),
    "release-hyprlang": re.compile(r"^\s*bind(?=[a-z]*r)(?=[a-z]*n)(?=[a-z]*i)[a-z]*\s*=.*\bexec\s*,.*\bsbarhive drop --release\b", re.M),
    "press-lua": re.compile(r"hl\.bind\(.{0,300}?\bdrop --hold\b", re.S),
    "release-lua": re.compile(r"hl\.bind\(.{0,300}?\bdrop --release\b[^{}]{0,40}\{[^}]*\bnon_consuming = true\b", re.S),
    "ignore_mods": re.compile(r"ignore_mods"),
    "unverified": re.compile(r"not verified|unverified|assumed|مفترض|غير متحقق", re.I),
    "hold_ms": re.compile(r"hold_ms"),
    "range": re.compile(r"(?s)(?=.*(?<!\d)400(?!\d))(?=.*(?<!\d)100(?!\d))(?=.*(?<!\d)2000(?!\d))"),
    "probing": re.compile(r"\bProbing\b"),
    "reveal-line": re.compile(r"\bhive reveal\b"),
    "probe-line": re.compile(r"\bhive probe\b"),
    "reasons": re.compile(r"(?s)" + "".join(r"(?=.*(?<![\w-])" + w + r"(?![\w-]))" for w in ("pointer", "no-uri-list", "no-actions", "release", "cap", "replaced"))),
    "open-at-reveal": re.compile(r"(?is)\bopen(?:ing)?\s+animation\b.{0,120}?\breveal|\breveal.{0,120}?\bopen(?:ing)?\s+animation\b"),
}
# S-021 AC12: keys met by one sentence of the prose, every pattern of one
# list (English patterns ignore case).
W = lambda w: r"(?<!\S)(?:" + w + r")(?!\S)"
SENT = {
    "only": [
        [r"\bonly\b", r"\bdrag", r"\baction"],
        [W("فقط|الا"), r"سحب", r"اجراء"],
    ],
    "first": [
        [r"\b(start|begin)\b", r"\bdrag", r"\b(then|first|before)\b", r"\b(hold|press)"],
        [W("ابدا"), r"سحب", W("ثم|اولا|قبل"), r"مسك|ضغط"],
    ],
    "nothing": [
        [r"\b(without|no)\b[^,;]{0,20}\bdrag", r"\bno actions?\b|\bwithout an action\b|\bwith no action\b",
         r"\bnothing\b|\bnever\b|\bdoes not (show|appear)\b"],
        [W("دون|بلا|من غير") + r" (?:ال)?سحب", W("بلا|دون|لا|ليس له") + r" (?:اي )?اجراء", r"لا (?:يظهر|تظهر)|لا شيء"],
    ],
    "hidden-catcher": [
        [r"\bhidden\b", r"\bcatcher"],
    ],
}
for i, body in enumerate(parts):
    head = heads[i - 1] if i > 0 else ""
    prose = re.sub(r"(?s)```.*?```|`[^`]*`", "", head + "\n" + body)
    lang = "ar" if len(AR.findall(prose)) > len(LAT.findall(prose)) else "en"
    found = [k for k, r in KEYS.items() if r.search(body)]
    sentences = re.split(r"[.!?؟](?:\s|$)|\n", prose)
    for k, lists in SENT.items():
        if any(all(re.search(p, t, re.I) for p in pats) for pats in lists for t in sentences):
            found.append(k)
    for k in found:
        print(lang, k, head.strip())
PY
}

# has FILE LANG KEY...: one section of FILE in LANG has every KEY.
has() {
  local file=$1 lang=$2; shift 2
  sections "$file" | python3 -I -c '
import sys
lang, keys = sys.argv[1], set(sys.argv[2:])
by = {}
for line in sys.stdin:
    parts = line.rstrip("\n").split(" ", 2)
    if len(parts) >= 2 and parts[0] == lang:
        by.setdefault(parts[2] if len(parts) > 2 else "", set()).add(parts[1])
sys.exit(0 if any(keys <= v for v in by.values()) else 1)' "$lang" "$@"
}

# doc_for LANG FILE: FILE for English, its .ar.md copy for Arabic.
doc_for() { if [ "$1" = ar ]; then echo "${2%.md}.ar.md"; else echo "$2"; fi; }
for lang in ar en; do
  name=$([ $lang = ar ] && echo Arabic || echo English)
  hypr=$(doc_for $lang "$HYPR") readme=$(doc_for $lang "$README")
  check "S-019 AC22: Hyprland README, one $name section with the hyprlang press bind (flag n, drop --hold) and release bind (flags r, n and i, drop --release)" \
    'has "$hypr" $lang press-hyprlang release-hyprlang'
  check "S-019 AC22: Hyprland README, one $name section with the Lua press and release binds" \
    'has "$hypr" $lang press-lua release-lua'
  check "S-019 AC22: Hyprland README, the $name binds name ignore_mods and say they are not verified yet" \
    'has "$hypr" $lang press-hyprlang ignore_mods unverified'
  check "S-019 AC23: README.md, the $name settings section names hold_ms with 400, 100 and 2000" \
    'has "$readme" $lang hold_ms range'
done
check "S-019 AC23: docs/architecture.md names hold_ms and drop --hold" 'grep -q "hold_ms" "$ARCH" && grep -q -- "drop --hold" "$ARCH"'
check "S-019 AC20/AC23: ADR 0003 names hold_ms and the 150 ms budget" 'grep -q "hold_ms" "$ADR" && grep -q "150" "$ADR"'
check "S-019 AC23: CHANGELOG.md has a line for the long press (--hold or hold_ms) in English and in Arabic" \
  'grep -E -- "--hold|hold_ms" "$LOG" | grep -P "[\x{0600}-\x{06FF}]" | grep -qE "[A-Za-z]{4,} [A-Za-z]{2,}"'
for lang in ar en; do
  name=$([ $lang = ar ] && echo Arabic || echo English)
  hypr=$(doc_for $lang "$HYPR")
  check "S-021 AC12: Hyprland README, one $name section says the hive shows only during a drag of files with an action, to start the drag first and then hold the key, and that with no drag or for a file with no action nothing shows" \
    'has "$hypr" $lang only first nothing'
done
check "S-021 AC12, S-022 AC9: docs/architecture.md, one section describes the probe: the Probing state, the hidden catcher, the \"hive reveal\" and \"hive probe\" lines and the six hide reasons (pointer, no-uri-list, no-actions, release, cap, replaced)" \
  'has "$ARCH" en probing hidden-catcher reveal-line probe-line reasons'
check "S-022 AC9: docs/architecture.md, that section also says the opening animation starts at the reveal" \
  'has "$ARCH" en probing reveal-line probe-line reasons open-at-reveal'
check "S-021 AC11: CHANGELOG.md has a line that names S-021 in English and in Arabic (released in 0.3.1)" \
  'grep -E "S-021" "$LOG" | grep -P "[\x{0600}-\x{06FF}]" | grep -qE "[A-Za-z]{4,} [A-Za-z]{2,}"'
check "S-022 AC9: CHANGELOG.md has a line that names S-022 in English and in Arabic (released in 0.3.2)" \
  'grep -E "S-022" "$LOG" | grep -P "[\x{0600}-\x{06FF}]" | grep -qE "[A-Za-z]{4,} [A-Za-z]{2,}"'
# S-022 AC10: prose lines of tests/drop/README.md over 80 columns, outside
# ``` fences ("LINE:COLUMNS" each).
long=$(awk '/^```/ { f = !f; next } !f && length($0) > 80 { printf "%d:%d ", NR, length($0) }' "$HERE/README.md")
echo "INFO S-022 AC10: tests/drop/README.md prose lines over 80 columns: ${long:-none}"
check "S-022 AC10: no prose line of tests/drop/README.md is longer than 80 columns (outside code fences)" '[ -z "$long" ]'
# consuming_release: "FILE:LINE" for each consuming release bind in the docs.
consuming_release() {
  find "$ROOT" -name '*.md' -not -path "$ROOT/backlog/*" -not -path '*/node_modules/*' \
    -not -path '*/target/*' -not -path '*/.git/*' -not -path '*/.claude/*' -print0 | xargs -0 python3 -I -c '
import re, sys
for f in sys.argv[1:]:
    try:
        t = open(f, encoding="utf-8").read()
    except (OSError, UnicodeDecodeError) as e:
        print(f"{f}:unreadable {e}")
        continue
    for m in re.finditer(r"(?m)^\s*bind([a-z]*)\s*=", t):
        if "r" in m.group(1) and "n" not in m.group(1):
            print(f"{f}:{t.count(chr(10), 0, m.start()) + 1}")
    for m in re.finditer(r"\{[^{}]*\brelease = true\b[^{}]*\}", t):
        if not re.search(r"\bnon_consuming = true\b", m.group(0)):
            print(f"{f}:{t.count(chr(10), 0, m.start()) + 1}")'
}
bad=$(consuming_release)
echo "INFO consuming release binds in the docs: ${bad:-none}"
check "no doc shows a consuming release bind (they swallow the key, e.g. every Z for SUPER+Z)" '[ -z "$bad" ]'
check "S-019 AC22/AC23, S-021 AC12: no em dash or en dash in these files" \
  '! grep -nP "\x{2014}|\x{2013}" "$HYPR" "$HYPR_AR" "$README" "$README_AR" "$ARCH" "$ADR" "$LOG" "$HERE/README.md"'
# Language pairs: both files exist and line 1 is the switch line.
switch_ok() {
  local en=$1 ar=${1%.md}.ar.md n
  n=$(basename "$en" .md)
  local want="[English]($n.md) | [العربية]($n.ar.md)"
  [ -f "$en" ] && [ -f "$ar" ] && [ "$(head -1 "$en")" = "$want" ] && [ "$(head -1 "$ar")" = "$want" ]
}
check "pair: README.md and README.ar.md exist and start with the language switch line" 'switch_ok "$README"'
check "pair: CONTRIBUTING.md and CONTRIBUTING.ar.md exist and start with the language switch line" 'switch_ok "$ROOT/CONTRIBUTING.md"'
if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
[ "$FAILS" -eq 0 ]
