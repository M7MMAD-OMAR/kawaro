#!/usr/bin/env bash
# S-014d AC12 (second half): the user docs explain how to bind `sbarhive drop`
# to a key by hand. No compositor.
#   - docs/ (any .md outside docs/adr/ and docs/PLAN.md) has, in Arabic and in
#     English, a section on each of:
#       GNOME     Settings, Keyboard, custom shortcut, command `sbarhive drop`
#       KDE       System Settings, Shortcuts, command `sbarhive drop`
#       Hyprland  a Super+Z bind line that runs `sbarhive drop`, and a left
#                 Alt bind line (key ALT_L) that runs sbarhive (the owner's
#                 two binds, S-014 log), hyprlang or Lua syntax
#     and names Super+Z, the default the owner chose.
#   - integrations/linux/hyprland/README.md, where Hyprland users look, gives
#     the same two bind lines in its English section, and its Arabic copy
#     README.ar.md beside it in an Arabic section.
#   - each user doc is a pair, English NAME.md and Arabic NAME.ar.md: an
#     Arabic check counts only sections of a .ar.md file, an English check
#     only sections of the other files. Both files of each pair checked here
#     (docs/drop-shortcut and the Hyprland README) exist and start with the
#     switch line "[English](NAME.md) | [العربية](NAME.ar.md)".
#   - every program a documented bind line runs is `sbarhive` or a helper
#     shipped in integrations/ or scripts/ (it must exist); no /home/ paths.
#   - no em dash or en dash in those files.
# A section is the text between two headings (any level); its language is
# Arabic when its prose (not code) has more Arabic than Latin letters, else
# English. Each topic's words and its code block must sit in one section, with
# no heading between them. Arabic is compared without harakat and with
# أ إ آ read as ا. In docs/ the Hyprland section must name Hyprland.
# Usage: tests/drop/docs.sh
# shellcheck disable=SC2016
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="${DROP_OUT:-$HERE/out}"
mkdir -p "$OUT"
FAILS=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }

DOCS=$(find "$ROOT/docs" -name '*.md' -not -path "$ROOT/docs/adr/*" -not -name PLAN.md | sort)
HYPR="$ROOT/integrations/linux/hyprland/README.md"
HYPR_AR="$ROOT/integrations/linux/hyprland/README.ar.md"

# scan --hyprland-word|--any FILE...
# Prints "KEY ar|en FILE:LINE" for every section that satisfies KEY.
scan() {
  python3 -I - "$@" <<'PY'
import re, sys
AR = re.compile(r"[؀-ۿ]")
LAT = re.compile(r"[A-Za-z]")
DIAC = re.compile(r"[ً-ْ]")
HAMZA = str.maketrans("أإآ", "ااا")
HYPR_WORD = sys.argv[1] == "--hyprland-word"
FILES = sys.argv[2:]
SUPERZ = [
    re.compile(r"^\s*bind[a-z]*\s*=\s*SUPER\s*,\s*Z\s*,\s*exec\s*,.*\bsbarhive drop\b", re.M | re.I),
    re.compile(r"hl\.bind\(\s*\"SUPER \+ Z\".{0,300}?\bsbarhive drop\b", re.S | re.I),
]
ALT = [
    re.compile(r"^\s*bind[a-z]*\s*=\s*(ALT)?\s*,\s*ALT_L\s*,\s*exec\s*,.*\bsbarhive", re.M | re.I),
    re.compile(r"hl\.bind\(\s*\"(ALT \+ )?ALT_L\".{0,300}?\bsbarhive", re.S | re.I),
]
def norm(s):
    return DIAC.sub("", s).translate(HAMZA).lower()
def has(text, *words):
    t = norm(text)
    return all(norm(w) in t for w in words)
KEYS = {
    "gnome": lambda t, l: has(t, "GNOME", "sbarhive drop") and (
        has(t, "الإعدادات", "لوحة المفاتيح", "اختصار") if l == "ar"
        else has(t, "Settings", "Keyboard", "Custom Shortcut")),
    "kde": lambda t, l: has(t, "KDE", "sbarhive drop") and (
        has(t, "إعدادات النظام", "اختصار") if l == "ar"
        else has(t, "System Settings", "Shortcuts")),
    "hypr-superz": lambda t, l: (has(t, "Hyprland") or not HYPR_WORD) and any(r.search(t) for r in SUPERZ),
    "hypr-alt": lambda t, l: (has(t, "Hyprland") or not HYPR_WORD) and any(r.search(t) for r in ALT),
    "superz": lambda t, l: re.search(r"Super\s*\+\s*Z|<Super>z", t, re.I) is not None,
}
for path in FILES:
    try:
        lines = open(path, encoding="utf-8").read().split("\n")
    except OSError:
        continue
    sections, cur, start, fence = [], [], 1, False
    for i, l in enumerate(lines, 1):
        if l.startswith("```"):
            fence = not fence
        if not fence and re.match(r"^#{1,6} ", l):
            sections.append((start, cur)); cur, start = [], i
        cur.append(l)
    sections.append((start, cur))
    for start, sec in sections:
        prose, fence = [], False
        for l in sec:
            if l.startswith("```"):
                fence = not fence; continue
            if not fence:
                prose.append(re.sub(r"`[^`]*`", "", l))
        p = "\n".join(prose)
        lang = "ar" if len(AR.findall(p)) > len(LAT.findall(p)) else "en"
        text = "\n".join(sec)
        for k, f in KEYS.items():
            if f(text, lang):
                print(k, lang, f"{path.split('/sbarhive/')[-1]}:{start}")
PY
}

# shellcheck disable=SC2086
found=$(scan --hyprland-word $DOCS)
echo "INFO docs/ files: $(tr "\n" " " <<<"${DOCS//$ROOT\//}")"
echo "INFO sections in docs/: $(tr '\n' ';' <<<"${found:-none}")"
# has_key KEY LANG FOUND: a LANG section with KEY, Arabic only in a .ar.md
# file and English only in the others.
has_key() {
  if [ "$2" = ar ]; then grep -E "^$1 ar " <<<"$3" | grep -qE '\.ar\.md:[0-9]+$'
  else grep -E "^$1 en " <<<"$3" | grep -qvE '\.ar\.md:[0-9]+$'; fi
}
for k in gnome kde hypr-superz hypr-alt superz; do
  case $k in
    gnome) what='GNOME: Settings, Keyboard, custom shortcut running `sbarhive drop`' ;;
    kde) what='KDE: System Settings, Shortcuts, command `sbarhive drop`' ;;
    hypr-superz) what='Hyprland: a Super+Z bind line running `sbarhive drop`' ;;
    hypr-alt) what='Hyprland: a left Alt (ALT_L) bind line running sbarhive' ;;
    superz) what='names the default shortcut Super+Z' ;;
  esac
  check "AC12: docs/ in Arabic: $what" 'has_key "$k" ar "$found"'
  check "AC12: docs/ in English: $what" 'has_key "$k" en "$found"'
done

# The whole README is about Hyprland: the word is not needed in the section.
hfound=$(scan --any "$HYPR" "$HYPR_AR")
echo "INFO sections in integrations/linux/hyprland/README.md and README.ar.md: $(tr '\n' ';' <<<"${hfound:-none}")"
for k in hypr-superz hypr-alt; do
  check "AC12: integrations/linux/hyprland/README.ar.md in Arabic: $k bind line" 'has_key "$k" ar "$hfound"'
  check "AC12: integrations/linux/hyprland/README.md in English: $k bind line" 'has_key "$k" en "$hfound"'
done

# Programs the documented Super+Z and ALT_L bind lines run: the first word
# after exec (hyprlang) or inside exec_cmd( (Lua), with ~, $HOME and HOME ..
# prefixes and quotes removed. Each must be sbarhive or a file in the repo.
# shellcheck disable=SC2086
progs=$(cat $DOCS "$HYPR" "$HYPR_AR" 2>/dev/null | python3 -I -c '
import re, sys
t = sys.stdin.read()
out = set()
for m in re.finditer(r"^\s*bind[a-z]*\s*=\s*[^,\n]*,\s*(Z|ALT_L)\s*,\s*exec\s*,\s*(\S+)", t, re.M | re.I):
    out.add(m.group(2))
for m in re.finditer(r"hl\.bind\(\s*\"(SUPER \+ Z|(ALT \+ )?ALT_L)\".{0,200}?exec_cmd\((.{0,200}?)\)", t, re.S | re.I):
    w = re.sub(r"HOME\s*\.\.\s*", "", m.group(3))
    w = re.findall(r"[^\s\"\x27]+", w.replace("..", " "))
    if w: out.add(w[0])
for p in sorted(out):
    print(p.replace("$HOME", "~"))
')
echo "INFO programs the documented binds run: $(tr '\n' ' ' <<<"${progs:-none}")"
# A helper a bind runs is installed from integrations/ or scripts/, so only
# those count: a same-named file elsewhere in the tree is no proof.
shipped=$(git -C "$ROOT" ls-files integrations scripts | awk -F/ '{print $NF}' | sort -u)
missing=""
while IFS= read -r p; do
  [ -z "$p" ] && continue
  b=$(basename "$p")
  [ "$b" = sbarhive ] && continue
  grep -qxF "$b" <<<"$shipped" || missing="$missing $p"
done <<<"$progs"
check "AC12: the documented binds run programs that exist (a bind was found)" '[ -n "$progs" ]'
check "AC12: every program a documented bind runs is sbarhive or a helper shipped in integrations/ or scripts/" '[ -n "$progs" ] && [ -z "$missing" ]'
echo "INFO not shipped:${missing:- none}"
# shellcheck disable=SC2086
check "AC12: no /home/ path in the docs" '! grep -n "/home/" $DOCS "$HYPR" "$HYPR_AR"'
# shellcheck disable=SC2086
check "AC12: no em dash or en dash in the docs" '! grep -nP "\x{2014}|\x{2013}" $DOCS "$HYPR" "$HYPR_AR"'

# Language pairs: both files exist and line 1 is the switch line.
switch_ok() {
  local en=$1 ar=${1%.md}.ar.md n
  n=$(basename "$en" .md)
  local want="[English]($n.md) | [العربية]($n.ar.md)"
  [ -f "$en" ] && [ -f "$ar" ] && [ "$(head -1 "$en")" = "$want" ] && [ "$(head -1 "$ar")" = "$want" ]
}
check "pair: docs/drop-shortcut.md and docs/drop-shortcut.ar.md exist and start with the language switch line" 'switch_ok "$ROOT/docs/drop-shortcut.md"'
check "pair: integrations/linux/hyprland/README.md and README.ar.md exist and start with the language switch line" 'switch_ok "$HYPR"'

if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
[ "$FAILS" -eq 0 ]
