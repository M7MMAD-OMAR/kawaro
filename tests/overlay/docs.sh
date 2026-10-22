#!/usr/bin/env bash
# S-018 AC19 and AC20 (S-018f): licences and documentation. No compositor.
#   AC19  i-slint-backend-winit and i-slint-core pinned "=1.18.1" in
#         crates/ui/Cargo.toml; those two, md-5 and png in docs/engines.md with
#         the version sbarhive-ui resolves in Cargo.lock and a licence; `cargo tree -d` adds
#         no duplicate over main before S-018 (BASE, default 4047d9d, the
#         commit that approved S-018, extracted with git archive into a
#         temporary folder, no worktree); every package in Cargo.lock that is
#         not in BASE's Cargo.lock is in docs/engines.md with its version.
#   AC20  docs/architecture.md names both hive paths and the choice rule (the
#         four globals, the screen reader, SBARHIVE_SURFACE); the user docs
#         (README.md, integrations/linux/hyprland/README.md) give a Hyprland
#         layerrule that turns off the animation of the "sbarhive" namespace,
#         in Arabic and in English, each marked as not verified; the
#         Arabic rule is read from the Arabic copies (README.ar.md beside
#         each), the English rule from the English files;
#         tests/overlay/README.md runs every overlay test with one command
#         (the command is read, not run).
# Usage: tests/overlay/docs.sh [BASE]
# shellcheck disable=SC2016
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
BASE="${1:-4047d9d}"
FAILS=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }
ENG="$ROOT/docs/engines.md"
LOCK="$ROOT/Cargo.lock"

# name version pairs of a Cargo.lock on stdin.
lock_pairs() { awk '/^\[\[package\]\]/{n=""} /^name = /{gsub(/"/,"",$3); n=$3} /^version = /{gsub(/"/,"",$3); if(n!="") print n, $3}' | sort -u; }
# The engines.md table row of a crate: "| name | ...".
row() { grep -E "^\| *$1 *\|" "$ENG"; }

echo "== AC19"
for c in i-slint-backend-winit i-slint-core; do
  check "AC19: $c pinned \"=1.18.1\" in crates/ui/Cargo.toml" \
    'grep -qE "^$c *= *(\"=1\.18\.1\"|\{.*version *= *\"=1\.18\.1\")" "$ROOT/crates/ui/Cargo.toml"'
done
# The versions sbarhive-ui itself resolves (Cargo.lock can hold several, png
# 0.17 for us and 0.18 for another crate).
direct=$(cd "$ROOT" && nice -n 19 cargo tree -p sbarhive-ui --offline --locked --depth 1 --prefix none -e normal,build,dev 2>/dev/null | sed 's/ (.*//' | sort -u)
for c in i-slint-backend-winit i-slint-core md-5 png; do
  v=$(awk -v n="$c" '$1==n{sub(/^v/,"",$2); print $2}' <<<"$direct" | head -1)
  r=$(row "$c")
  echo "INFO $c: sbarhive-ui resolves ${v:-none}; engines.md row: ${r:-none}"
  # The row's version column and a licence (an SPDX id or the Slint licence).
  check "AC19: $c in docs/engines.md with the version sbarhive-ui resolves (${v:-?}) and a licence" \
    '[ -n "$v" ] && [ -n "$r" ] && awk -F"|" "{print \$3}" <<<"$r" | grep -qF "$v" && awk -F"|" "{print \$4}" <<<"$r" | grep -qE "MIT|Apache-2\.0|GPL-3\.0|BSD|Zlib|Unlicense|ISC|MPL"'
done

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
git -C "$ROOT" archive "$BASE" | tar -x -C "$TMP" || echo "INFO git archive $BASE failed"
tree_d() { (cd "$1" && nice -n 19 cargo tree -d --offline --locked --workspace --depth 0 --prefix none 2>/dev/null) | grep -E '^[A-Za-z0-9_-]+ v[0-9]' | sed 's/ (.*//' | sort -u; }
tree_d "$TMP" >"$TMP/.dups-base"
tree_d "$ROOT" >"$TMP/.dups-now"
added=$(comm -13 "$TMP/.dups-base" "$TMP/.dups-now" | tr '\n' ' ')
echo "INFO cargo tree -d: $BASE $(wc -l <"$TMP/.dups-base") duplicate versions, now $(wc -l <"$TMP/.dups-now"); added: ${added:-none}"
check "AC19: cargo tree -d ran on $BASE and now" '[ -s "$TMP/.dups-now" ] && [ -s "$TMP/.dups-base" ]'
check "AC19: cargo tree -d adds no duplicate version over $BASE" '[ -z "$added" ]'

lock_pairs <"$TMP/Cargo.lock" >"$TMP/.lock-base"
lock_pairs <"$LOCK" >"$TMP/.lock-now"
newnames=$(comm -13 <(cut -d' ' -f1 "$TMP/.lock-base" | sort -u) <(cut -d' ' -f1 "$TMP/.lock-now" | sort -u))
missing=""
for c in $newnames; do
  v=$(awk -v n="$c" '$1==n{print $2}' "$TMP/.lock-now" | sort -V | tail -1)
  r=$(row "$c")
  { [ -n "$r" ] && grep -qF "$v" <<<"$r"; } || missing="$missing $c@$v"
done
echo "INFO packages in Cargo.lock new since $BASE: $(tr '\n' ' ' <<<"${newnames:-none}"); unregistered:${missing:- none}"
check "AC19: every package new in Cargo.lock since $BASE is in docs/engines.md with its version" '[ -s "$TMP/.lock-base" ] && [ -z "$missing" ]'

echo "== AC20"
ARCH="$ROOT/docs/architecture.md"
for t in zwlr_layer_shell_v1 wl_subcompositor wp_viewporter wp_single_pixel_buffer SBARHIVE_SURFACE; do
  check "AC20: docs/architecture.md names $t (the layer path and the choice rule)" 'grep -qF "$t" "$ARCH"'
done
check "AC20: docs/architecture.md describes the window path (xdg_toplevel, xdg_surface or xdg-shell)" 'grep -qE "xdg_toplevel|xdg_surface|xdg-shell" "$ARCH"'
check "AC20: docs/architecture.md gives the screen reader rule" 'grep -qiE "screen reader|ScreenReaderEnabled" "$ARCH"'

# A Hyprland layer rule that turns off the animation of the "sbarhive"
# namespace (noanim or no_anim, any Hyprland syntax), in each language, with
# the "not verified" mark within 5 lines of it, the Arabic and the English
# rule on different lines. Arabic is matched with its diacritics removed.
rules=$(python3 -I - "$ROOT/README.md" "$ROOT/README.ar.md" "$ROOT/integrations/linux/hyprland/README.md" "$ROOT/integrations/linux/hyprland/README.ar.md" <<'PY'
import re, sys
strip = lambda s: re.sub("[ً-ْ]", "", s)
rule = re.compile(r"layerrule.*(noanim|no_anim)|(noanim|no_anim).*layerrule", re.I)
ar, en = strip("لم يُتحقق منها"), re.compile(r"not verified|unverified", re.I)
found = {"ar": [], "en": []}
for path in sys.argv[1:]:
    try:
        lines = open(path, encoding="utf-8").read().split("\n")
    except OSError:
        continue
    for i, l in enumerate(lines):
        if rule.search(l) and "sbarhive" in l:
            near = "\n".join(lines[max(0, i - 5): i + 6])
            # The Arabic mark counts in an Arabic copy (.ar.md), the
            # English one in an English file.
            if path.endswith(".ar.md") and ar in strip(near):
                found["ar"].append(f"{path.split('/')[-3:]}:{i + 1}")
            if not path.endswith(".ar.md") and en.search(near):
                found["en"].append(f"{path.split('/')[-3:]}:{i + 1}")
# One rule per language: an Arabic and an English rule on different lines.
two = any(a != e for a in found["ar"] for e in found["en"])
print(f"ar {len(found['ar'])} en {len(found['en'])} {'two' if two else 'one'} {found}")
PY
)
echo "INFO Hyprland layer rule: $rules"
read -r _ nar _ nen lines _ <<<"$rules"
check "AC20: Arabic user docs give a Hyprland layerrule turning off sbarhive's animation, marked «لم يُتحقق منها»" '[ "${nar:-0}" -ge 1 ]'
check "AC20: English user docs give the same layerrule, marked not verified" '[ "${nen:-0}" -ge 1 ]'
check "AC20: the Arabic and the English rule are two lines (one per language)" '[ "${lines:-}" = two ]'

README="$HERE/README.md"
check "AC20: tests/overlay/README.md exists" '[ -s "$README" ]'
# The one command: the first line of a fenced block that names a tests/
# script and is not a cargo build. Every overlay test must be in it or in the
# script it runs (a script under tests/ the command names).
cmd=$(awk '/^```/{f=!f; next} f' "$README" 2>/dev/null | grep -E 'tests/[A-Za-z0-9_/.-]+\.sh' | grep -vE '^\s*cargo ' | head -1)
script=$(grep -oE 'tests/[A-Za-z0-9_/.-]+\.sh' <<<"$cmd" | head -1)
echo "INFO README command: ${cmd:-none}"
check "AC20: the README's command runs a script that exists" '[ -n "$script" ] && [ -x "$ROOT/$script" ]'
body=""
[ -n "$script" ] && [ -f "$ROOT/$script" ] && body="$cmd
$(cat "$ROOT/$script")"
cov_missing=""
for m in window input token a11y drop scheme 1.5; do
  grep -qwF -- "$m" <<<"$body" || cov_missing="$cov_missing kwin.sh:$m"
done
for s in tests/overlay/kwin.sh tests/overlay/gnome.sh tests/overlay/focus.sh tests/overlay/budget.sh tests/overlay/docs.sh; do
  grep -qE "(^|[/ \"'\$])$(basename "$s" | sed 's/\./\\./')" <<<"$body" || cov_missing="$cov_missing $s"
done
echo "INFO not covered by the one command:${cov_missing:- none}"
check "AC20: the one command covers kwin.sh in every mode, gnome.sh, focus.sh, budget.sh and docs.sh" '[ -n "$body" ] && [ -z "$cov_missing" ]'

if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
[ "$FAILS" -eq 0 ]
