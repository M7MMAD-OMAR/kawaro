#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-053c AC3: scripts/sign-update-manifest.sh, with throwaway keys only.
#
# Contract (DECISION marks a gap in the story):
#   UPDATE_SIGNING_PASSWORD=... sign-update-manifest.sh KEY_FILE MANIFEST
#   - writes MANIFEST.minisig (minisign's default name) with `minisign -S`,
#     called as `minisign` through PATH (DECISION, so the test can shadow it).
#   - the passphrase is read from UPDATE_SIGNING_PASSWORD and handed to
#     minisign on its standard input, never on its command line (DECISION).
#   - exit 2 when: not two arguments, KEY_FILE is not a file, MANIFEST is not
#     a file, or UPDATE_SIGNING_PASSWORD is unset (DECISION). Nothing is
#     written then. Any minisign failure (wrong passphrase) is a non zero exit
#     and no .minisig is left behind (DECISION).
#   - the output of a run contains neither the passphrase nor the key file's
#     secret line, on success or failure; the script has no xtrace.
# shellcheck disable=SC2016,SC2012
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
S="$REPO_ROOT/scripts/sign-update-manifest.sh"
PW="S3ntinel-Pass-9f2c41"
BADPW="Wr0ng-Pass-77aa10"

K="$TMP/keys"
mkdir -p "$K"
printf '%s\n%s\n' "$PW" "$PW" | minisign -G -p "$K/enc.pub" -s "$K/enc.key" >/dev/null 2>&1
minisign -G -W -p "$K/plain.pub" -s "$K/plain.key" >/dev/null 2>&1
check "setup throwaway encrypted key" test -s "$K/enc.key"
SECRET_LINE="$(sed -n 2p "$K/enc.key")"

check "AC3 script exists and is executable" test -x "$S"
# every later check is only meaningful when the script exists
[ -x "$S" ] || { finish; exit 1; }

M="$TMP/beta.json"
printf '{"schema":1}\n' > "$M"
cp "$M" "$M.orig"
UPDATE_SIGNING_PASSWORD="$PW" "$S" "$K/enc.key" "$M" >"$TMP/ok.log" 2>&1
check "AC3 signing with the right passphrase exits 0" test "$?" -eq 0
check "AC3 writes MANIFEST.minisig" test -s "$M.minisig"
check "AC3 the manifest itself is untouched" cmp -s "$M" "$M.orig"
minisign -V -p "$K/enc.pub" -m "$M" >/dev/null 2>&1
check "AC3 minisign verifies the signature with the public key" test "$?" -eq 0
check "AC3 output has no passphrase" test "$(grep -c "$PW" "$TMP/ok.log")" -eq 0
check "AC3 output has no secret key line" test "$(grep -cF "$SECRET_LINE" "$TMP/ok.log")" -eq 0

# wrong passphrase
rm -f "$M.minisig"
UPDATE_SIGNING_PASSWORD="$BADPW" "$S" "$K/enc.key" "$M" >"$TMP/bad.log" 2>&1
check "AC3 wrong passphrase exits non zero" test "$?" -ne 0
check "AC3 wrong passphrase leaves no .minisig" test ! -e "$M.minisig"
check "AC3 failure output has no passphrase" \
  test "$(grep -c -e "$BADPW" -e "$PW" "$TMP/bad.log")" -eq 0
check "AC3 failure output has no secret key line" test "$(grep -cF "$SECRET_LINE" "$TMP/bad.log")" -eq 0

# refusals
rm -f "$M.minisig"
UPDATE_SIGNING_PASSWORD="$PW" "$S" "$K/missing.key" "$M" >"$TMP/miss.log" 2>&1
check "AC3 missing key file exits 2" test "$?" -eq 2
check "AC3 missing key file writes no .minisig" test ! -e "$M.minisig"
check "AC3 missing key output has no passphrase" test "$(grep -c "$PW" "$TMP/miss.log")" -eq 0
UPDATE_SIGNING_PASSWORD="$PW" "$S" "$K/enc.key" "$TMP/no-manifest.json" >/dev/null 2>&1
check "AC3 missing manifest exits 2" test "$?" -eq 2
env -u UPDATE_SIGNING_PASSWORD "$S" "$K/enc.key" "$M" >/dev/null 2>&1
check "AC3 unset UPDATE_SIGNING_PASSWORD exits 2" test "$?" -eq 2
check "AC3 unset passphrase writes no .minisig" test ! -e "$M.minisig"
UPDATE_SIGNING_PASSWORD="$PW" "$S" "$K/enc.key" >/dev/null 2>&1
check "AC3 one argument exits 2" test "$?" -eq 2

# a key without a passphrase works too (the throwaway -W key)
UPDATE_SIGNING_PASSWORD="$PW" "$S" "$K/plain.key" "$M" >"$TMP/plain.log" 2>&1
check "AC3 an unencrypted throwaway key signs" test "$?" -eq 0
minisign -V -p "$K/plain.pub" -m "$M" >/dev/null 2>&1
check "AC3 and verifies" test "$?" -eq 0
check "AC3 passphrase not echoed for the unencrypted key either" test "$(grep -c "$PW" "$TMP/plain.log")" -eq 0
rm -f "$M.minisig"

# the passphrase goes to minisign on stdin, never on the command line
FAKE="$TMP/fakebin"
mkdir -p "$FAKE"
cat > "$FAKE/minisign" <<'FAKE_EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$FAKE_LOG.args"
cat > "$FAKE_LOG.stdin"
m=""
while [ "$#" -gt 0 ]; do [ "$1" = -m ] && m="$2"; shift; done
printf 'fake signature\n' > "$m.minisig"
FAKE_EOF
chmod +x "$FAKE/minisign"
PATH="$FAKE:$PATH" FAKE_LOG="$TMP/fake" UPDATE_SIGNING_PASSWORD="$PW" \
  "$S" "$K/enc.key" "$M" >"$TMP/fake.log" 2>&1
check "AC3 runs minisign from PATH and succeeds with the fake" test "$?" -eq 0
check "AC3 minisign gets -S" grep -qx -e '-S' "$TMP/fake.args"
check "AC3 minisign gets -s KEY_FILE" grep -qxF -e "$K/enc.key" "$TMP/fake.args"
check "AC3 minisign gets -m MANIFEST" grep -qxF -e "$M" "$TMP/fake.args"
check "AC3 the passphrase is not in minisign's arguments" test "$(grep -c "$PW" "$TMP/fake.args")" -eq 0
check "AC3 the passphrase reaches minisign on stdin" grep -qF "$PW" "$TMP/fake.stdin"
check "AC3 the fake run printed no passphrase" test "$(grep -c "$PW" "$TMP/fake.log")" -eq 0

# static: no tracing, no passphrase on a command line
check "AC3 no set -x or xtrace in the script" \
  bash -c '! grep -Eq "set +-[a-z]*x|xtrace|bash +-[a-z]*x" "$1"' _ "$S"
check "AC3 the script names UPDATE_SIGNING_PASSWORD" grep -q 'UPDATE_SIGNING_PASSWORD' "$S"

finish
