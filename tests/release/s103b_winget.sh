#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103b: scripts/channels/winget.sh follows the channel contract. Dry run
# builds the four manifests locally and validates them (golden); without the
# token it is a clean SKIP (10); with a stub komac the token travels only in
# the environment.
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
S="$REPO_ROOT/scripts/channels/winget.sh"
G="$REPO_ROOT/tests/release/golden"
cd "$REPO_ROOT" || exit 1
TOKEN="fixture-token-0123456789"
export KAWARO_DIST="$G/winget-input"
export CANONICAL_REPO="OWNER/kawaro"

check "b1 winget.sh exists" test -f "$S"
OUT="$TMP/out.txt"

env -u WINGET_CREATE_GITHUB_TOKEN bash "$S" v1.2.3 --dry-run --out "$TMP/m1" > "$OUT" 2>&1
RC=$?
check "b2 dry run without the token: exit 0 (never a SKIP)" test "$RC" -eq 0
check "b3 NOTE line names the secret" grep -qx 'NOTE winget: WINGET_CREATE_GITHUB_TOKEN not configured, a real run would SKIP' "$OUT"
check "b4 manifests equal the golden directory byte for byte" diff -r "$G/winget" "$TMP/m1"

WINGET_CREATE_GITHUB_TOKEN="$TOKEN" bash "$S" v1.2.3 --dry-run --out "$TMP/m2" > "$OUT" 2>&1
RC=$?
check "b5 dry run with the token: exit 0" test "$RC" -eq 0
check "b6 dry run says what it would do" grep -q '^DRYRUN winget: would publish 1.2.3' "$OUT"
check "b7 dry run never prints the token" bash -c "! grep -q '$TOKEN' '$OUT'"

# Validation fails on a missing or malformed MSI hash.
BAD="$TMP/bad"
mkdir -p "$BAD"
grep -v '\.msi$' "$G/winget-input/SHA256SUMS" > "$BAD/SHA256SUMS"
KAWARO_DIST="$BAD" WINGET_CREATE_GITHUB_TOKEN="$TOKEN" bash "$S" v1.2.3 --dry-run --out "$TMP/m3" > "$OUT" 2>&1
check "b8 no MSI line in SHA256SUMS fails" test "$?" -ne 0 -a "$?" -ne 10
BAD2="$TMP/bad2"
mkdir -p "$BAD2"
sed 's/^abcdef01234567/abcdef0123456/' "$G/winget-input/SHA256SUMS" > "$BAD2/SHA256SUMS"
KAWARO_DIST="$BAD2" WINGET_CREATE_GITHUB_TOKEN="$TOKEN" bash "$S" v1.2.3 --dry-run --out "$TMP/m4" > /dev/null 2>&1
RC=$?
check "b9 a short hash fails (exit not 0 or 10)" bash -c "[ $RC -ne 0 ] && [ $RC -ne 10 ]"
expect_status "b10 a prerelease tag is refused with a failure code other than 10" 2 bash "$S" v1.2.3-beta.1 --dry-run

# Real run with a stub komac on PATH.
STUB="$TMP/stub"
mkdir -p "$STUB"
cat > "$STUB/komac" <<'K'
#!/usr/bin/env bash
printf '%s\n' "$*" > "$KOMAC_LOG"
printf '%s' "${GITHUB_TOKEN:-}" > "$KOMAC_LOG.token"
echo "komac: ok"
K
chmod +x "$STUB/komac"
export KOMAC_LOG="$TMP/komac.log"
PATH="$STUB:$PATH" WINGET_CREATE_GITHUB_TOKEN="$TOKEN" bash "$S" v1.2.3 > "$OUT" 2>&1
RC=$?
check "b11 real run with stub komac exits 0" test "$RC" -eq 0
check "b12 komac gets update, identifier, version, MSI url, --submit" grep -qx 'update Sbarah.Kawaro --version 1.2.3 --urls https://github.com/OWNER/kawaro/releases/download/v1.2.3/kawaro-1.2.3-windows-x64.msi --submit' "$KOMAC_LOG"
check "b13 the token reaches komac by environment only" test "$(cat "$KOMAC_LOG.token")" = "$TOKEN"
check "b14 the token is not on the command line" bash -c "! grep -q '$TOKEN' '$KOMAC_LOG'"
check "b15 the token is not in the output" bash -c "! grep -q '$TOKEN' '$OUT'"
PATH="$STUB:$PATH" env -u WINGET_CREATE_GITHUB_TOKEN bash "$S" v1.2.3 > "$OUT" 2>&1
check "b16 real run without the token is a SKIP (10)" test "$?" -eq 10
check "b17 shellcheck passes on winget.sh" bash -c "command -v shellcheck >/dev/null && shellcheck '$S' || ! command -v shellcheck"
check "b18 komac licence is recorded in docs/engines.md" grep -q 'Komac' "$REPO_ROOT/docs/engines.md"
check "b19 the merge gate of winget-pkgs#462805 is documented" grep -q '462805' "$S"
finish
