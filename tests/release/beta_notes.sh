#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-055 AC4 and the doc half of AC9: the release notes, CHANGELOG, PLAN.
#
# DECISIONS (the story leaves them open):
#   - the English file has the six headings as `## Status`, `## What works`,
#     `## Known gaps`, `## Updates`, `## Privacy`, `## Verify your download`, in this order;
#   - the Arabic file has exactly six `## ` headings, each holding an Arabic letter, in the
#     same order; it keeps the Windows button names "More info" and "Run anyway" and the
#     literal `update_check = false`, `SmartScreen`, `GNOME`, `KDE` as they appear in the UI
#     or the config file;
#   - the manager ruling 3: both updaters (S-054a, S-054b) are done, so the notes describe
#     in-app updates as working and name the two untested live parts.
# shellcheck disable=SC2016,SC2012
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT" || exit 1
EN=docs/release-notes-0.4.0-beta.1.md
AR=docs/release-notes-0.4.0-beta.1.ar.md
SWITCH='[English](release-notes-0.4.0-beta.1.md) | [العربية](release-notes-0.4.0-beta.1.ar.md)'
LONG_EM="$(printf '\xe2\x80\x94')"; LONG_EN="$(printf '\xe2\x80\x93')"

check "AC4 $EN exists and is not empty" test -s "$EN"
check "AC4 $AR exists and is not empty" test -s "$AR"
[ -s "$EN" ] && [ -s "$AR" ] || { finish; exit 1; }

check "AC4 English first line is the switch line" test "$(head -1 "$EN")" = "$SWITCH"
check "AC4 Arabic first line is the switch line" test "$(head -1 "$AR")" = "$SWITCH"

sect() { # sect FILE HEADING: the body of a `## HEADING` section
  awk -v h="## $2" '/^## /{on=($0==h); next} on' "$1"
}
want="Status
What works
Known gaps
Updates
Privacy
Verify your download"
check "AC4 English headings are exactly the six, in order" \
  test "$(grep '^## ' "$EN" | sed 's/^## //')" = "$want"
check "AC4 Arabic has exactly six headings" test "$(grep -c '^## ' "$AR")" -eq 6
check "AC4 every Arabic heading holds Arabic letters" \
  test "$(grep '^## ' "$AR" | LC_ALL=C grep -c -e "$(printf '\xd8')" -e "$(printf '\xd9')")" -eq 6
for h in Status "What works" "Known gaps" Updates Privacy "Verify your download"; do
  check "AC4 English section '$h' has text" test -n "$(sect "$EN" "$h" | tr -d '[:space:]')"
done

low() { tr '[:upper:]' '[:lower:]'; }
ST="$(sect "$EN" Status | low)"
check "AC4 Status says beta plainly" grep -qw 'beta' <<<"$ST"
KG="$(sect "$EN" "Known gaps")"
KGL="$(low <<<"$KG")"
check "AC4 gaps: Windows is unsigned" bash -c 'grep -qw unsigned <<<"$1" || grep -q "not signed" <<<"$1"' _ "$KGL"
check "AC4 gaps: SmartScreen" grep -qF 'SmartScreen' <<<"$KG"
check "AC4 gaps: More info step" grep -qF 'More info' <<<"$KG"
check "AC4 gaps: Run anyway step" grep -qF 'Run anyway' <<<"$KG"
check "AC4 gaps: no drop hive on Windows" grep -qi 'drop hive' <<<"$KG"
check "AC4 gaps: no notifications on Windows" grep -qi 'notification' <<<"$KG"
check "AC4 gaps: Linux focus tests on GNOME" grep -qF 'GNOME' <<<"$KG"
check "AC4 gaps: Linux focus tests on KDE" grep -qF 'KDE' <<<"$KG"
check "AC4 gaps: the focus tests are pending" grep -qi 'pending' <<<"$KG"
check "AC4 gaps: the in-app download percentage is untested live" grep -qi 'percent' <<<"$KG"
check "AC4 gaps: the live Windows helper is untested" grep -qi 'helper' <<<"$KG"
UP="$(sect "$EN" Updates)"
check "AC4 updates: signed manifest" grep -qi 'signed manifest' <<<"$UP"
check "AC4 updates: channel beta" grep -qF 'beta' <<<"$UP"
check "AC4 updates: channel stable" grep -qF 'stable' <<<"$UP"
check "AC4 updates: update_check = false" grep -qF 'update_check = false' <<<"$UP"
check "AC4 updates: a manual check exists" grep -qi 'manual' <<<"$UP"
check "AC4 privacy: no telemetry" grep -qi 'no telemetry' <<<"$(sect "$EN" Privacy)"
VF="$(sect "$EN" "Verify your download")"
check "AC4 verify: SHA256SUMS" grep -qF 'SHA256SUMS' <<<"$VF"
check "AC4 verify: sha256sum -c" grep -qF 'sha256sum -c' <<<"$VF"

for t in SmartScreen "More info" "Run anyway" GNOME KDE 'update_check = false' SHA256SUMS 'sha256sum -c'; do
  check "AC4 Arabic copy keeps '$t'" grep -qF "$t" "$AR"
done
check "AC4 Arabic copy is a real translation (many Arabic letters)" \
  test "$(LC_ALL=C grep -c -e "$(printf '\xd8')" -e "$(printf '\xd9')" "$AR")" -ge 10

for f in "$EN" "$AR"; do
  check "AC4 $f has no em dash or en dash" bash -c '! LC_ALL=C grep -q -e "$1" -e "$2" "$3"' _ "$LONG_EM" "$LONG_EN" "$f"
  check "AC4 $f has no personal data (home path, email, user name)" \
    bash -c '! grep -Eqi "/home/|sbarah|gmail|@[a-z0-9-]+\.[a-z]" "$1"' _ "$f"
done
check "AC4 the English file has no Arabic letters after the switch line" \
  bash -c '! tail -n +2 "$1" | LC_ALL=C grep -q -e "$2" -e "$3"' _ "$EN" "$(printf '\xd8')" "$(printf '\xd9')"
expect_status "AC9 design-lint is clean with the notes in place" 0 scripts/design-lint.sh

# AC9: CHANGELOG and PLAN
check "AC9 exactly one CHANGELOG line for S-055" test "$(grep -c 'S-055' CHANGELOG.md)" -eq 1
LINE="$(grep 'S-055' CHANGELOG.md | head -1)"
check "AC9 the CHANGELOG line is a list item with an English part" grep -Eq '^- [A-Za-z].*\(S-055\)' <<<"$LINE"
check "AC9 the CHANGELOG line names 0.4.0-beta.1" grep -qF '0.4.0-beta.1' <<<"$LINE"
check "AC9 the CHANGELOG line has an Arabic part in guillemets" \
  bash -c 'LC_ALL=C grep -q -e "$2" -e "$3" <<<"$1"' _ "$LINE" "$(printf '\xc2\xab\xd8')" "$(printf '\xc2\xab\xd9')"
check "AC9 the CHANGELOG line has no long dash" bash -c '! LC_ALL=C grep -q -e "$2" -e "$3" <<<"$1"' _ "$LINE" "$LONG_EM" "$LONG_EN"
check "AC9 docs/PLAN.md lists 0.4.0-beta.1" grep -qF '0.4.0-beta.1' docs/PLAN.md
finish
