#!/usr/bin/env bash
# S-014e3 (backlog/S-014-drop-hive.md, AC22): the portal section of the drop
# shortcut docs. No compositor; a static check for the reviewer, who still
# reads the text.
#
# AC22: docs/drop-shortcut.md, in Arabic and in English, has the portal
# section: on GNOME 48 and later and KDE 6 the daemon registers Super+Z
# itself, with a consent dialog the first time, and it is changed in the
# settings (GNOME: Apps, then "Global shortcuts"; KDE: System Settings, then
# Shortcuts); on Hyprland the line `bind = SUPER, Z, global,
# io.github.m7mmad_omar.SbarHive:drop` (the identifier AC20 proves); the
# portal shortcut works by holding (`hold_ms`), not by a tap; and where there
# is no portal the manual bind stays. A static check finds these lines in
# each language, tests/drop/docs.sh and tests/drop/hold_docs.sh pass, no em
# dash or en dash, and one CHANGELOG.md line in both languages.
# The manager's answers (2026-10-22) and note add, in the same section:
#   answer 2  a Super+Z bound by hand is to be removed once the portal has
#             registered it;
#   answer 5  a refusal of the dialog is remembered as
#             `portal_shortcut = false` in config.toml, and registration can
#             be turned back on (the settings toggle is S-014f's);
#             check 9 as S-014f2 updated it (AC27, below);
#   answer 6  on Hyprland the documented `exec` bind with `--at` stays the
#             first way and the portal `global` line is the alternative: the
#             first Super+Z exec line of the file comes before the global
#             line, and the global line's section calls it the alternative;
#   note      (2026-10-22) on GNOME the consent dialog may open behind a
#             full screen window.
#
# Language pairs (CLAUDE.md): English NAME.md and its Arabic copy NAME.ar.md
# beside it. An English check reads docs/drop-shortcut.md, an Arabic check
# docs/drop-shortcut.ar.md. The Hyprland README pair
# (integrations/linux/hyprland/README.md and README.ar.md) gives the global
# line too, after its exec Super+Z line, called the alternative.
#
# The portal section: a heading whose text names the portal (English
# "portal", Arabic "بوابة") and everything up to the next heading of the
# same or a higher level, its subsections included. Every key below must be
# met inside one such section of the file. Arabic is compared without
# harakat, with أ إ آ read as ا. A key of two or more word groups is met
# by one sentence of the prose (a sentence ends at . ! ? ؟ or a line end;
# inline code is kept, code blocks are left out); a literal (a bind line,
# `hold_ms`, `portal_shortcut = false`) is looked for anywhere in the
# section, code blocks included.
#
# S-014f2 (AC27) updates check 9: the section says that a refusal is
# remembered as `portal_shortcut = false`; that the settings switch, its
# label quoted in that language ("Register the drop shortcut with the
# desktop", «تسجيل اختصار الإفلات لدى سطح المكتب», in straight, curly or
# angle quotes), turns registration back on or off (one sentence), within
# seconds with no restart (one sentence); and that off leaves the manual
# bind (one sentence). No portal section has
# `systemctl --user restart sbarhive` any more.
#
# CHANGELOG.md: one line that names S-014e (or S-014e1, e2, e3) with
# Arabic and English text (it was under "## [Unreleased]" until the 0.3.3
# release).
# S-014f4 (AC28): the portal section also says, in each language, that a
# key changed in the desktop's settings is kept by SbarHive (one sentence:
# changed, kept, SbarHive or the daemon); that on KDE it applies at once
# (one sentence naming KDE and not GNOME); and that on GNOME it applies at
# the latest after logging out and back in (one sentence naming GNOME and
# not KDE). A GNOME sentence that says "at once" (or immediately) is allowed
# only if the last `tests/drop/gnome.sh smart portal-real rebind` run, its
# output in $DROP_OUT or tests/drop/out (gnome-smart-portal-real-rebind.txt),
# printed "INFO AC16 GNOME live rebind: followed". CHANGELOG.md has exactly
# one line under "## [Unreleased]" (up to the next "## [") naming S-014f
# (or S-014f1 to f4), with English and Arabic text. No em dash or en dash in
# any file changed since S014F4_BASE (default 8a6623f, main before S-014f4)
# or untracked, besides the files above.
# Usage: tests/drop/portal_docs.sh
# shellcheck disable=SC2016,SC2034
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
FAILS=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }

DOC="$ROOT/docs/drop-shortcut.md"
DOC_AR="$ROOT/docs/drop-shortcut.ar.md"
HYPR="$ROOT/integrations/linux/hyprland/README.md"
HYPR_AR="$ROOT/integrations/linux/hyprland/README.ar.md"
LOG="$ROOT/CHANGELOG.md"

# keys LANG FILE [--any]: prints every key the file's portal section meets,
# one per line ("KEY HEADING"); with --any, every section counts (the
# Hyprland README, where the file is all about Hyprland and its portal
# lines may sit under a Hyprland heading).
keys() {
  python3 -I - "$@" <<'PY'
import re, sys
lang, path = sys.argv[1], sys.argv[2]
anysec = "--any" in sys.argv[3:]
DIAC = re.compile(r"[ً-ْٰ]")
HAMZA = str.maketrans("أإآ", "ااا")
try:
    text = DIAC.sub("", open(path, encoding="utf-8").read()).translate(HAMZA)
except OSError:
    sys.exit(0)
lines = text.split("\n")
# Headings outside code fences: (line index, level, text).
heads, fence = [], False
for i, l in enumerate(lines):
    if l.startswith("```"):
        fence = not fence
        continue
    m = None if fence else re.match(r"^(#{1,6}) (.*)$", l)
    if m:
        heads.append((i, len(m.group(1)), m.group(2)))
spans = []
for n, (i, lvl, h) in enumerate(heads):
    end = len(lines)
    for j, l2, _ in heads[n + 1:]:
        if l2 <= lvl:
            end = j
            break
    spans.append((h, i, end))
if anysec:
    spans.append(("(whole file)", 0, len(lines)))
PORTAL_HEAD = re.compile(r"portal" if lang == "en" else r"بوابة|البوابة", re.I)

GLOBAL = re.compile(r"^\s*bind\s*=\s*SUPER\s*,\s*Z\s*,\s*global\s*,\s*io\.github\.m7mmad_omar\.SbarHive:drop\s*$", re.M)
EXEC = re.compile(r"^\s*bind[a-z]*\s*=\s*SUPER\s*,\s*Z\s*,\s*exec\s*,.*\bsbarhive drop\b.*--at\b", re.M)
W = lambda w: r"(?<!\S)(?:" + w + r")(?!\S)"
# AC27: the switch's label, quoted (Arabic compared as the text above, so
# the hamza of الإفلات is read as ا).
LABEL_EN = r"[\"“«]Register the drop shortcut with the desktop[\"”»]"
LABEL_AR = r"[\"“«]تسجيل اختصار الافلات لدى سطح المكتب[\"”»]"
# Sentence keys: every pattern of one list in one sentence (case ignored).
SENT = {
    "en": {
        "versions": [[r"\bGNOME 48\b", r"\b(later|newer|and up|or above)\b", r"\b(KDE|Plasma) 6\b"]],
        "registers-itself": [[r"\bregister", r"Super\s*\+\s*Z|\bit\b|\bshortcut"], ],
        "consent-first": [[r"\bdialog", r"\bfirst\b"]],
        "gnome-change": [[r"\bApps\b", r"\bGlobal Shortcuts\b"]],
        "kde-change": [[r"\bSystem Settings\b", r"\bShortcuts\b"]],
        "hold-not-tap": [[r"\bhold", r"\btap"]],
        "manual-stays": [[r"\b(no|without( a)?|lacks?( a)?)\b[^.]{0,20}\bportal\b", r"\b(manual|by hand)\b"]],
        "remove-manual": [[r"\bremove\b", r"\b(manual|by hand)\b"]],
        "behind-full-screen": [[r"\bbehind\b", r"\bfull[ -]?screen\b"]],
        "refusal-remembered": [[r"\b(refus|declin|deny|denied|cancel)", r"portal_shortcut = false"]],
        "switch-on-off": [[LABEL_EN, r"\bon\b", r"\boff\b"]],
        "live-no-restart": [[r"\bseconds?\b", r"\b(no|without|never|not)\b[^.]{0,30}\brestart"]],
        "off-manual": [[r"\boff\b", r"\b(manual|by hand)\b"]],
        "alternative": [[r"\balternative\b|\binstead\b", r"\bglobal\b"]],
        # AC28 (S-014f4).
        "key-kept": [[r"\bchang", r"\b(keeps?|kept)\b", r"\bSbarHive\b|\bdaemon\b"]],
        "kde-at-once": [[r"\bKDE\b", r"\bat once\b|\bimmediately\b|\bright away\b"]],
        "gnome-logout": [[r"\bGNOME\b", r"\b(log|sign)(ging)? out\b", r"\bback in\b|\b(log|sign)(ging)? (back )?in\b"]],
        "gnome-at-once": [[r"\bGNOME\b", r"\bat once\b|\bimmediately\b|\bright away\b"]],
    },
    "ar": {
        "versions": [[r"GNOME 48", W("وما بعده|وما بعدها|او احدث|واحدث|فما بعد|او ما بعده"), r"(KDE|Plasma) 6"]],
        "registers-itself": [[r"تسجل|يسجل|تسجيل", r"Super\s*\+\s*Z|الاختصار"]],
        "consent-first": [[r"نافذة", r"اول مرة|المرة الاولى|اول تشغيل|لاول مرة"]],
        "gnome-change": [[r"التطبيقات", r"Global Shortcuts"]],
        "kde-change": [[r"اعدادات النظام", r"الاختصارات"]],
        "hold-not-tap": [[r"مسك|امساك|الضغط المطول", r"نقر"]],
        "manual-stays": [[W("لا|دون|بلا|غياب|من غير") + r" (?:ال)?بوابة|غياب البوابة|لا توجد بوابة|حين لا بوابة", r"يدوي"]],
        "remove-manual": [[r"احذف|حذف|ازل|ازالة|امسح", r"يدوي"]],
        "behind-full-screen": [[r"خلف|وراء", r"ملء الشاشة|كامل الشاشة|الشاشة الكاملة|تملا الشاشة"]],
        "refusal-remembered": [[r"رفض|الغ|ترفض", r"portal_shortcut = false"]],
        "switch-on-off": [[LABEL_AR, r"تشغيل|تفعيل|تعيد", r"ايقاف|وقف|تعطيل|عطل|اطفاء"]],
        "live-no-restart": [[r"ثوان|ثانية", r"(دون|بلا|من غير|لا حاجة|لا تحتاج|لا يلزم)[^.]{0,30}(اعادة|تعيد|تعد) (ال)?تشغيل"]],
        "off-manual": [[r"ايقاف|وقف|تعطيل|عطل|اطفاء", r"يدوي"]],
        "alternative": [[r"بديل|بدلا", r"global"]],
        # AC28 (S-014f4).
        "key-kept": [[r"غير|تغيير|تغير", r"يحتفظ|تحتفظ|يبقي|يبقى|تبقى|يحافظ", r"SbarHive|صبار هايف|الخدمة"]],
        "kde-at-once": [[r"KDE", r"فورا|في الحال|على الفور|مباشرة"]],
        "gnome-logout": [[r"GNOME", r"الخروج", r"الدخول"]],
        "gnome-at-once": [[r"GNOME", r"فورا|في الحال|على الفور|مباشرة"]],
    },
}
# AC28: a per desktop sentence must not name the other desktop.
NOT = {
    "kde-at-once": r"GNOME",
    "gnome-logout": r"\b(KDE|Plasma)\b",
    "gnome-at-once": r"\b(KDE|Plasma)\b",
}
LIT = {
    "hold_ms": re.compile(r"\bhold_ms\b"),
    "portal_shortcut": re.compile(r"\bportal_shortcut = false\b"),
    "global-line": GLOBAL,
    # AC27: found means the section still has the restart command.
    "restart-cmd": re.compile(r"systemctl --user restart sbarhive"),
}
first_exec = EXEC.search(text)
first_global = GLOBAL.search(text)
for h, a, b in spans:
    if not anysec and not PORTAL_HEAD.search(h):
        continue
    body = "\n".join(lines[a:b])
    prose = re.sub(r"(?s)```.*?```", "", body)
    sentences = re.split(r"[.!?؟](?:\s|$)|\n", prose)
    found = [k for k, r in LIT.items() if r.search(body)]
    for k, lists in SENT[lang].items():
        flags = 0 if lang == "ar" else re.I
        if any(all(re.search(p, t, flags) for p in pats) and not (k in NOT and re.search(NOT[k], t, flags))
               for pats in lists for t in sentences):
            found.append(k)
    # Answer 6: the first exec Super+Z line of the file (with --at) comes
    # before the first global line.
    if first_exec and first_global and first_exec.start() < first_global.start():
        found.append("exec-first")
    for k in found:
        print(k, h.strip())
PY
}

# has LANG FILE [--any] KEY...: one portal section of FILE meets every KEY.
has() {
  local lang=$1 file=$2 any=(); shift 2
  [ "${1:-}" = --any ] && { any=(--any); shift; }
  keys "$lang" "$file" "${any[@]}" | python3 -I -c '
import sys
want = set(sys.argv[1:])
by = {}
for line in sys.stdin:
    k, _, h = line.rstrip("\n").partition(" ")
    by.setdefault(h, set()).add(k)
sys.exit(0 if any(want <= v for v in by.values()) else 1)' "$@"
}

# AC28: the AC16 GNOME INFO line of the last rebind run.
GOUT="${DROP_OUT:-$HERE/out}/gnome-smart-portal-real-rebind.txt"
GNOME_LIVE=$(sed -nE 's/^INFO AC16 GNOME live rebind: (followed|not followed)$/\1/p' "$GOUT" 2>/dev/null | tail -1)
echo "INFO AC28: AC16 GNOME live rebind in ${GOUT#"$ROOT/"}: ${GNOME_LIVE:-no line}"

for lang in en ar; do
  name=$([ $lang = ar ] && echo Arabic || echo English)
  doc=$([ $lang = ar ] && echo "$DOC_AR" || echo "$DOC")
  rel=${doc#"$ROOT/"}
  echo "INFO AC22 $rel: keys met by portal sections: $(keys $lang "$doc" | sort | tr '\n' ';')"
  pat=$([ $lang = ar ] && echo 'بوابة' || echo '[Pp]ortal')
  check "AC22 $rel ($name): a portal section (a heading naming the portal) exists" \
    'grep -qE "^#{1,6} .*($pat)" "$doc"'
  check "AC22 $rel ($name): in GNOME 48 and later and KDE 6 the daemon registers Super+Z itself, with a consent dialog the first time" \
    'has $lang "$doc" versions registers-itself consent-first'
  check "AC22 $rel ($name): it is changed in the settings (GNOME: Apps, then \"Global shortcuts\"; KDE: System Settings, then Shortcuts)" \
    'has $lang "$doc" gnome-change kde-change'
  check "AC22 $rel ($name): on Hyprland the line \`bind = SUPER, Z, global, io.github.m7mmad_omar.SbarHive:drop\`, as the alternative, the documented exec bind with --at staying the first way (answer 6)" \
    'has $lang "$doc" global-line alternative exec-first'
  check "AC22 $rel ($name): the portal shortcut works by holding (\`hold_ms\`), not by a tap" \
    'has $lang "$doc" hold_ms hold-not-tap'
  check "AC22 $rel ($name): where there is no portal the manual bind stays" \
    'has $lang "$doc" manual-stays'
  check "AC22 $rel ($name): a Super+Z bound by hand is to be removed once the portal registers it (answer 2)" \
    'has $lang "$doc" remove-manual'
  check "AC22 $rel ($name): on GNOME the consent dialog may open behind a full screen window (note 2026-10-22)" \
    'has $lang "$doc" behind-full-screen'
  check "AC27 $rel ($name): a refusal is remembered as \`portal_shortcut = false\`; the settings switch, its label quoted, turns registration back on or off within seconds with no restart; off leaves the manual bind; no \`systemctl --user restart sbarhive\` in the portal section (check 9, S-014f2)" \
    'has $lang "$doc" portal_shortcut refusal-remembered switch-on-off live-no-restart off-manual && ! keys $lang "$doc" | grep -q "^restart-cmd "'
  hypr=$([ $lang = ar ] && echo "$HYPR_AR" || echo "$HYPR")
  check "AC22 ${hypr#"$ROOT/"} ($name): the global line, after the exec Super+Z line with --at, called the alternative (answer 6)" \
    'has $lang "$hypr" --any global-line alternative exec-first'
  # S-014f4 (AC28).
  check "AC28 $rel ($name): a key changed in the desktop's settings is kept by SbarHive (one sentence)" \
    'has $lang "$doc" key-kept'
  check "AC28 $rel ($name): on KDE the changed key applies at once (a sentence naming KDE and not GNOME)" \
    'has $lang "$doc" kde-at-once'
  check "AC28 $rel ($name): on GNOME the changed key applies at the latest after logging out and back in (a sentence naming GNOME and not KDE)" \
    'has $lang "$doc" gnome-logout'
  if has $lang "$doc" gnome-at-once; then
    echo "INFO AC28 $rel: the GNOME sentence says at once; the last GNOME rebind run: ${GNOME_LIVE:-no output file}"
    check "AC28 $rel ($name): GNOME \"at once\" only because the AC16 INFO line said followed" '[ "$GNOME_LIVE" = followed ]'
  fi
done

# The S-014d and S-019 docs checks still pass.
dout=$("$HERE/docs.sh" 2>&1); drc=$?
hout=$("$HERE/hold_docs.sh" 2>&1); hrc=$?
echo "INFO AC22: tests/drop/docs.sh $(tail -1 <<<"$dout"); tests/drop/hold_docs.sh $(tail -1 <<<"$hout")"
check "AC22: tests/drop/docs.sh passes" '[ "$drc" -eq 0 ]'
check "AC22: tests/drop/hold_docs.sh passes" '[ "$hrc" -eq 0 ]'
check "AC22: no em dash or en dash in docs/drop-shortcut.md, its Arabic copy, the Hyprland README pair and CHANGELOG.md" \
  '! grep -nP "\x{2014}|\x{2013}" "$DOC" "$DOC_AR" "$HYPR" "$HYPR_AR" "$LOG"'
lines=$(grep -E 'S-014e[0-9]?\b' "$LOG" | grep -P "[\x{0600}-\x{06FF}]" | grep -cE "[A-Za-z]{4,} [A-Za-z]{2,}")
echo "INFO AC22: CHANGELOG.md lines naming S-014e in both languages: $lines"
check "AC22: CHANGELOG.md has one line that names S-014e, in English and in Arabic (released in 0.3.3)" '[ "$lines" -eq 1 ]'
# AC28: one S-014f line under "## [Unreleased]", in both languages.
unrel=$(awk '/^## \[Unreleased\]/ { on = 1; next } /^## \[/ { on = 0 } on' "$LOG")
fl=$(grep -E 'S-014f[0-9]?\b' <<<"$unrel")
fn=$(grep -c . <<<"$fl")
fb=$(grep -P "[\x{0600}-\x{06FF}]" <<<"$fl" | grep -cE "[A-Za-z]{4,} [A-Za-z]{2,}")
echo "INFO AC28: CHANGELOG.md lines under [Unreleased] naming S-014f: $fn, with English and Arabic: $fb"
check "AC28: CHANGELOG.md has exactly one line under \"## [Unreleased]\" naming S-014f, in English and in Arabic" \
  '[ "$fn" -eq 1 ] && [ "$fb" -eq 1 ]'
# AC28: no long dash in any file changed in S-014f4.
BASE=${S014F4_BASE:-8a6623f}
changed=$(cd "$ROOT" && git diff --name-only "$BASE" -- 2>/dev/null) && changed+=$'\n'$(cd "$ROOT" && git ls-files --others --exclude-standard) || changed="(git diff against $BASE failed)"
changed=$(sort -u <<<"$changed")
dashed=""
while IFS= read -r f; do
  [ -n "$f" ] && [ -f "$ROOT/$f" ] && grep -qIP "\x{2014}|\x{2013}" "$ROOT/$f" && dashed+="$f "
done <<<"$changed"
echo "INFO AC28: files changed since $BASE or untracked: $(grep -c . <<<"$changed"); with a long dash: ${dashed:-none}"
check "AC28: no em dash or en dash in any file changed since $BASE (or untracked)" '[ -z "$dashed" ] && ! grep -q "^(git diff" <<<"$changed"'
if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
[ "$FAILS" -eq 0 ]
