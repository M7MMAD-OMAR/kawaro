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
#         flags r and i (bindri, bindir...), a Lua `hl.bind(` that runs
#         `drop --hold` and one that runs `drop --release`, the words
#         `ignore_mods`, and a note that the binds are not verified yet
#         (English "not verified", "unverified" or "assumed"; Arabic
#         "مفترض" or "غير متحقق").
#   AC23  README.md: its Arabic and its English settings section each name
#         `hold_ms` with 400, 100 and 2000; docs/architecture.md names
#         `hold_ms` and `drop --hold`; docs/adr/0003-drop-hive.md names
#         `hold_ms` and 150; CHANGELOG.md has one line that names `--hold`
#         or `hold_ms` in English and in Arabic (one line per story, as the
#         file's other entries are written).
#   No em dash or en dash in any of these files.
# Usage: tests/drop/hold_docs.sh
# shellcheck disable=SC2016,SC2034
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
FAILS=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }

HYPR="$ROOT/integrations/linux/hyprland/README.md"
README="$ROOT/README.md"
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
    "release-hyprlang": re.compile(r"^\s*bind(?=[a-z]*r)(?=[a-z]*i)[a-z]*\s*=.*\bexec\s*,.*\bsbarhive drop --release\b", re.M),
    "press-lua": re.compile(r"hl\.bind\(.{0,300}?\bdrop --hold\b", re.S),
    "release-lua": re.compile(r"hl\.bind\(.{0,300}?\bdrop --release\b", re.S),
    "ignore_mods": re.compile(r"ignore_mods"),
    "unverified": re.compile(r"not verified|unverified|assumed|مفترض|غير متحقق", re.I),
    "hold_ms": re.compile(r"hold_ms"),
    "range": re.compile(r"(?s)(?=.*(?<!\d)400(?!\d))(?=.*(?<!\d)100(?!\d))(?=.*(?<!\d)2000(?!\d))"),
}
for i, body in enumerate(parts):
    head = heads[i - 1] if i > 0 else ""
    prose = re.sub(r"(?s)```.*?```|`[^`]*`", "", head + "\n" + body)
    lang = "ar" if len(AR.findall(prose)) > len(LAT.findall(prose)) else "en"
    found = [k for k, r in KEYS.items() if r.search(body)]
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

for lang in ar en; do
  name=$([ $lang = ar ] && echo Arabic || echo English)
  check "S-019 AC22: Hyprland README, one $name section with the hyprlang press bind (flag n, drop --hold) and release bind (flags r and i, drop --release)" \
    'has "$HYPR" $lang press-hyprlang release-hyprlang'
  check "S-019 AC22: Hyprland README, one $name section with the Lua press and release binds" \
    'has "$HYPR" $lang press-lua release-lua'
  check "S-019 AC22: Hyprland README, the $name binds name ignore_mods and say they are not verified yet" \
    'has "$HYPR" $lang press-hyprlang ignore_mods unverified'
  check "S-019 AC23: README.md, the $name settings section names hold_ms with 400, 100 and 2000" \
    'has "$README" $lang hold_ms range'
done
check "S-019 AC23: docs/architecture.md names hold_ms and drop --hold" 'grep -q "hold_ms" "$ARCH" && grep -q -- "drop --hold" "$ARCH"'
check "S-019 AC20/AC23: ADR 0003 names hold_ms and the 150 ms budget" 'grep -q "hold_ms" "$ADR" && grep -q "150" "$ADR"'
check "S-019 AC23: CHANGELOG.md has a line for the long press (--hold or hold_ms) in English and in Arabic" \
  'grep -E -- "--hold|hold_ms" "$LOG" | grep -P "[\x{0600}-\x{06FF}]" | grep -qE "[A-Za-z]{4,} [A-Za-z]{2,}"'
check "S-019 AC22/AC23: no em dash or en dash in these files" \
  '! grep -nP "\x{2014}|\x{2013}" "$HYPR" "$README" "$ARCH" "$ADR" "$LOG"'
if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
[ "$FAILS" -eq 0 ]
