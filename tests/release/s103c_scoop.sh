#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103c: Scoop bucket channel. Offline: fixture SHA256SUMS, golden manifest,
# a local bare repository standing in for the bucket repository.
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT" || exit 1
CH="$REPO_ROOT/scripts/channels/scoop.sh"
GEN="$REPO_ROOT/packaging/scoop/generate.sh"
VAL="$REPO_ROOT/packaging/scoop/validate.sh"
GOLD="$REPO_ROOT/tests/release/golden"
SUMS="$GOLD/channels/SHA256SUMS-0.4.3"
export TMPDIR="$TMP" CANONICAL_REPO=OWNER/kawaro
TOKEN="fixture-deploy-key-do-not-print-1234"

unset BUCKET_REPO SCOOP_DEPLOY_KEY SCOOP_BUCKET_URL
export SHA256SUMS_FILE="$SUMS"

# AC-c1: generated manifest equals the golden file, hash equals the ZIP line.
"$GEN" v0.4.3 "$SUMS" OWNER/kawaro >"$TMP/kawaro.json" 2>/dev/null
check "c1 manifest equals golden" diff -u "$GOLD/scoop/kawaro.json" "$TMP/kawaro.json"
zip_hash="$(grep 'windows-x64.zip$' "$SUMS" | cut -d' ' -f1)"
check "c1 hash equals the ZIP line of SHA256SUMS" test "$(jq -r '.architecture."64bit".hash' "$TMP/kawaro.json")" = "$zip_hash"
expect_status "c1 generator rejects a tag without a ZIP line" 1 "$GEN" v9.9.9 "$SUMS" OWNER/kawaro

# AC-c2: schema style check passes, and fails on a one character wrong hash.
expect_status "c2 validate passes on the generated file" 0 "$VAL" "$TMP/kawaro.json" "$SUMS"
sed "s/$zip_hash/${zip_hash:0:63}0/" "$TMP/kawaro.json" >"$TMP/badhash.json"
expect_status "c2 one character wrong hash fails" 1 "$VAL" "$TMP/badhash.json" "$SUMS"
jq 'del(.bin)' "$TMP/kawaro.json" >"$TMP/nobin.json"
expect_status "c2 missing bin fails" 1 "$VAL" "$TMP/nobin.json" "$SUMS"
jq '.autoupdate.architecture."64bit".url |= sub("\\$version"; "1.0.0"; "g")' "$TMP/kawaro.json" >"$TMP/noauto.json"
expect_status "c2 autoupdate without \$version fails" 1 "$VAL" "$TMP/noauto.json" "$SUMS"

# Channel contract.
out="$("$CH" v0.4.3 --dry-run 2>&1)"; rc=$?
check "dry run exits 0" test "$rc" = 0
check "dry run prints the DRYRUN line" grep -qx 'DRYRUN scoop: would publish 0.4.3' <<<"$out"
check "dry run names the ssh push with strict host key checking" \
  bash -c 'grep -q "StrictHostKeyChecking=yes" <<<"$1" && grep -q github_known_hosts <<<"$1"' _ "$out"
check "the committed GitHub known_hosts has the ed25519 host key" grep -Eq '^github\.com ssh-ed25519 AAAA' "$REPO_ROOT/packaging/release/github_known_hosts"
check "scoop.sh does not use HTTPS tokens or relax host keys" \
  bash -c '! grep -qE "extraheader|x-access-token|StrictHostKeyChecking=no|accept-new" "$1" && grep -q "StrictHostKeyChecking=yes" "$1"' _ "$CH"
out="$("$CH" v0.4.3 2>&1)"; rc=$?
check "no deploy key: exit 10" test "$rc" = 10
check "no deploy key: one SKIP line naming the variable" test "$out" = "SKIP scoop: SCOOP_DEPLOY_KEY not configured"
out="$(SCOOP_DEPLOY_KEY=$TOKEN "$CH" v0.4.3 2>&1)"; rc=$?
check "no bucket repo: exit 10 and SKIP names BUCKET_REPO" test "$rc:$out" = "10:SKIP scoop: BUCKET_REPO not configured"
out="$("$CH" v0.5.0-beta.1 2>&1)"; rc=$?
check "prerelease tag: exit 10" test "$rc" = 10
expect_status "bad tag: failure code 2" 2 "$CH" notatag

# AC-c3: mocked push into a local bare repository, rerun creates no commit.
git init -q --bare "$TMP/bucket.git"
export BUCKET_REPO=OWNER/scoop-kawaro SCOOP_DEPLOY_KEY="$TOKEN" SCOOP_BUCKET_URL="file://$TMP/bucket.git"
out1="$("$CH" v0.4.3 2>&1)"; rc1=$?
check "c3 first run exits 0" test "$rc1" = 0
check "c3 first run reports PUBLISHED" grep -qx 'PUBLISHED scoop: 0.4.3' <<<"$out1"
n1="$(git -C "$TMP/bucket.git" rev-list --count HEAD 2>/dev/null || echo 0)"
out2="$("$CH" v0.4.3 2>&1)"; rc2=$?
n2="$(git -C "$TMP/bucket.git" rev-list --count HEAD 2>/dev/null || echo 0)"
check "c3 first run made 1 commit" test "$n1" = 1
check "c3 rerun exits 0 and reports UNCHANGED" test "$rc2:$out2" = "0:UNCHANGED scoop: 0.4.3"
check "c3 rerun made 0 new commits" test "$n2" = "$n1"
git -C "$TMP/bucket.git" show HEAD:bucket/kawaro.json >"$TMP/pushed.json" 2>/dev/null
check "c3 pushed file equals golden" diff -q "$GOLD/scoop/kawaro.json" "$TMP/pushed.json"
check "c3 deploy key never printed" test -z "$(printf '%s\n%s\n' "$out1" "$out2" | grep -F "$TOKEN")"
check "c3 the commit has no personal identity" bash -c '! git -C "$1" log -1 --format="%an %ae %cn %ce" | grep -qE "@[A-Za-z0-9.-]+\.[a-z]{2,}"' _ "$TMP/bucket.git"
check "c3 script never forces a push" test -z "$(grep -E 'git .*push.*(--force| -f |[[:space:]]\+)' "$CH" | grep -v '^ *#')"

# Hygiene.
files="$CH $GEN $VAL $REPO_ROOT/packaging/scoop/kawaro.json.tpl $REPO_ROOT/docs/distribution-channels/scoop.md $0"
dash="$(printf '\xe2\x80\x94\|\xe2\x80\x93')"
check "no long dashes" test -z "$(grep -l "$dash" $files 2>/dev/null)"
check "no email address or home path" test -z "$(grep -lE '[[:alnum:]._-]+@[[:alnum:]][[:alnum:]-]*\.[a-z]+|/ho''me/' $files 2>/dev/null)"
if command -v shellcheck >/dev/null 2>&1; then
  check "shellcheck" shellcheck -S warning -x "$CH" "$GEN" "$VAL"
fi
finish
