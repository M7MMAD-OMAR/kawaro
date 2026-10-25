#!/usr/bin/env bash
# S-006 AC6: merge_uca.py adds and removes our Thunar action idempotently.
set -u
# shellcheck source=tests/integrations/lib.sh
. "$(dirname "$0")/lib.sh"
make_sandbox
trap drop_sandbox EXIT

M="$REPO_ROOT/integrations/linux/thunar/merge_uca.py"
FX="$REPO_ROOT/tests/integrations/fixtures"
UCA="$XDG_CONFIG_HOME/Thunar/uca.xml"
mkdir -p "$XDG_CONFIG_HOME/Thunar"
cp "$FX/uca-user.xml" "$UCA"

count_ours () { grep -c '<unique-id>kawaro</unique-id>' "$1" 2>/dev/null || true; }
sha () { sha256sum < "$1" | cut -c1-16; }

echo "== add twice (default path from XDG_CONFIG_HOME)"
check "add #1 exits 0" python3 "$M" add --command "/opt/x/kawaro open -- %F"
s1="$(sha "$UCA")"
check "add #2 exits 0" python3 "$M" add --command "/opt/x/kawaro open -- %F"
s2="$(sha "$UCA")"
assert_eq "one kawaro action" "$(count_ours "$UCA")" "1"
assert_eq "second add leaves the file byte identical" "$s2" "$s1"
assert_contains "command override" "$UCA" "<command>/opt/x/kawaro open -- %F</command>"
assert_contains "other action kept" "$UCA" "<unique-id>1700000000000001-1</unique-id>"
assert_contains "comment kept" "$UCA" "kept by the user: must survive every merge"
check "result is well formed" xmllint --noout "$UCA"

echo "== add with a new command updates in place"
check "add #3 exits 0" python3 "$M" add --file "$UCA" --command "kawaro open -- %F"
assert_eq "still one kawaro action" "$(count_ours "$UCA")" "1"
assert_contains "command updated" "$UCA" "<command>kawaro open -- %F</command>"

echo "== remove twice"
check "remove #1 exits 0" python3 "$M" remove --file "$UCA"
r1="$(sha "$UCA")"
check "remove #2 exits 0" python3 "$M" remove --file "$UCA"
r2="$(sha "$UCA")"
assert_eq "no kawaro action" "$(count_ours "$UCA")" "0"
assert_eq "second remove leaves the file byte identical" "$r2" "$r1"
assert_contains "other action still there" "$UCA" "<unique-id>1700000000000001-1</unique-id>"
assert_contains "comment still there" "$UCA" "kept by the user: must survive every merge"

echo "== broken uca.xml is never rewritten"
B="$SB/broken.xml"
cp "$FX/uca-broken.xml" "$B"
b0="$(sha "$B")"
if python3 "$M" add --file "$B" 2>/dev/null; then fail "add on broken file exited 0"; else pass; fi
if python3 "$M" remove --file "$B" 2>/dev/null; then fail "remove on broken file exited 0"; else pass; fi
assert_eq "broken file untouched" "$(sha "$B")" "$b0"

echo "== missing file seeded from the system file, and removed again"
S="$SB/fresh/Thunar/uca.xml"
check "add with seed" python3 "$M" add --file "$S" --seed "$FX/uca-system.xml"
assert_contains "seed action copied" "$S" "<unique-id>1-1</unique-id>"
assert_eq "ours added" "$(count_ours "$S")" "1"
check "remove with seed" python3 "$M" remove --file "$S" --seed "$FX/uca-system.xml"
assert_absent "$S"

echo "== missing file with no seed starts empty"
E="$SB/empty/uca.xml"
check "add without seed" python3 "$M" add --file "$E" --seed "$SB/does-not-exist.xml"
assert_eq "ours added" "$(count_ours "$E")" "1"
check "remove with absent seed" python3 "$M" remove --file "$E" --seed "$SB/does-not-exist.xml"
assert_absent "$E"
check "remove on missing file exits 0" python3 "$M" remove --file "$E"

finish test_merge_uca
