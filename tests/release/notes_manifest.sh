#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-099a A4 and A5 (script half): scripts/make-update-manifest.sh carries the
# notes of docs/whats-new/<version>.json into the manifest before signing.
#
# Contract (pinned in backlog/S-099-update-experience.md):
#   - the notes directory is $KAWARO_WHATS_NEW_DIR when set, otherwise
#     <repo>/docs/whats-new.
#   - a valid file (3 to 5 lines per language, one line each, 1 to 80
#     characters) becomes the top level "notes" key, {"en":[..],"ar":[..]},
#     with the same lines as the file.
#   - no file for the version: no "notes" key, exit 0 (the manifest stays
#     valid, the update is still offered).
#   - a file that breaks the rule: exit 2 and OUT_JSON is not created, so a
#     release never ships notes the client would drop.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
S="$REPO_ROOT/scripts/make-update-manifest.sh"
V="0.4.3"
BASE="https://github.com/M7MMAD-OMAR/kawaro/releases/download/v$V"
RPM="kawaro-0.4.3-1.x86_64.rpm"

DIST="$TMP/dist"
NOTES="$TMP/notes"
mkdir -p "$DIST" "$NOTES"
printf 'rpm bytes\n' > "$DIST/$RPM"
jq -n --arg b "$BASE" --arg n "$RPM" --argjson s "$(stat -c %s "$DIST/$RPM")" \
  '{assets: [{name: $n, url: ($b + "/" + $n), size: $s}]}' > "$TMP/assets.json"

run() { KAWARO_WHATS_NEW_DIR="$NOTES" "$S" --dist "$DIST" stable "$V" "$TMP/assets.json" "$1"; }

GOOD='{"en":["One","Two","Three","Four"],"ar":["واحد","اثنان","ثلاثة","أربعة"]}'
printf '%s\n' "$GOOD" > "$NOTES/$V.json"
OUT="$TMP/with.json"
run "$OUT" >"$TMP/run.log" 2>&1
check "A4 exit 0 with a valid notes file" test "$?" -eq 0
check "A4 the notes key carries the file lines" \
  test "$(jq -c .notes "$OUT" 2>/dev/null)" = "$(jq -c . <<<"$GOOD")"
check "A4 the other keys are unchanged" \
  test "$(jq -c 'keys' "$OUT" 2>/dev/null)" = '["assets","channel","notes","notes_url","published","schema","version"]'

rm -f "$NOTES/$V.json"
OUT="$TMP/without.json"
run "$OUT" >"$TMP/run.log" 2>&1
check "A4 exit 0 without a notes file" test "$?" -eq 0
check "A4 no notes key without a notes file" test "$(jq 'has("notes")' "$OUT" 2>/dev/null)" = false

bad() { # bad "description" JSON
  printf '%s\n' "$2" > "$NOTES/$V.json"
  rm -f "$TMP/bad.json"
  expect_status "A4 $1: exit 2" 2 run "$TMP/bad.json"
  check "A4 $1: no output file" test ! -e "$TMP/bad.json"
}
bad "2 lines" '{"en":["a","b"],"ar":["a","b","c"]}'
bad "6 lines" '{"en":["a","b","c","d","e","f"],"ar":["a","b","c"]}'
bad "81 characters" "$(jq -nc --arg l "$(printf 'x%.0s' $(seq 81))" '{en:[$l,"b","c"],ar:["a","b","c"]}')"
bad "an empty line" '{"en":["a","","c"],"ar":["a","b","c"]}'
bad "a missing language" '{"en":["a","b","c"]}'

finish
