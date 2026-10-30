#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103e: AUR kawaro-bin channel. Offline: fixture SHA256SUMS for two
# versions, golden PKGBUILD and .SRCINFO, a local bare repository standing in
# for the AUR.
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT" || exit 1
CH="$REPO_ROOT/scripts/channels/aur.sh"
GEN="$REPO_ROOT/packaging/aur/generate.sh"
GOLD="$REPO_ROOT/tests/release/golden"
export TMPDIR="$TMP" CANONICAL_REPO=OWNER/kawaro
KEY="fixture-key-material-do-not-print-5678"

unset AUR_SSH_KEY AUR_REPO_URL AUR_KNOWN_HOSTS AUR_ENABLED

# AC-e1: two fixture versions equal the golden files, sha256 equals the DEB line.
for v in 0.4.3 1.2.0; do
  sums="$GOLD/channels/SHA256SUMS-$v"
  mkdir -p "$TMP/o$v"
  "$GEN" "v$v" "$sums" OWNER/kawaro "$TMP/o$v" >/dev/null 2>&1
  check "e1 $v PKGBUILD equals golden" diff -u "$GOLD/aur-$v/PKGBUILD" "$TMP/o$v/PKGBUILD"
  check "e1 $v .SRCINFO equals golden" diff -u "$GOLD/aur-$v/.SRCINFO" "$TMP/o$v/.SRCINFO"
  deb="$(grep '_amd64.deb$' "$sums" | cut -d' ' -f1)"
  check "e1 $v sha256sums equals the DEB line" grep -q "^sha256sums=('$deb')$" "$TMP/o$v/PKGBUILD"
  # .SRCINFO agrees with the values bash computes from the PKGBUILD itself.
  got="$(bash -c 'srcdir=/x pkgdir=/y; . "$1"; printf "%s\n" "$pkgver" "${provides[0]}" "${noextract[0]}" "${source[0]}" "${sha256sums[0]}"' _ "$TMP/o$v/PKGBUILD")"
  want="$(printf '%s\n' "$v" "kawaro=$v" "kawaro_${v}_amd64.deb" \
    "$(sed -n 's/^\tsource = //p' "$TMP/o$v/.SRCINFO")" "$deb")"
  check "e1 $v .SRCINFO consistent with the sourced PKGBUILD" test "$got" = "$want"
  check "e1 $v bash -n PKGBUILD" bash -n "$TMP/o$v/PKGBUILD"
  if command -v shellcheck >/dev/null 2>&1; then
    check "e2 $v shellcheck PKGBUILD" shellcheck -S warning -s bash -e SC2034,SC2154,SC2164 "$TMP/o$v/PKGBUILD"
  fi
done
check "e1 PKGBUILD carries provides, conflicts and the licence" \
  grep -q -e "^conflicts=('kawaro')$" -e "^license=('GPL-3.0-or-later')$" "$TMP/o0.4.3/PKGBUILD"
expect_status "e1 wrong SHA256SUMS (no DEB line) fails" 1 "$GEN" v9.9.9 "$GOLD/channels/SHA256SUMS-0.4.3" OWNER/kawaro "$TMP/o0.4.3"

# Channel contract.
export SHA256SUMS_FILE="$GOLD/channels/SHA256SUMS-0.4.3"
out="$("$CH" v0.4.3 --dry-run 2>&1)"; rc=$?
check "dry run exits 0" test "$rc" = 0
check "dry run prints the DRYRUN line" grep -qx 'DRYRUN aur: would publish 0.4.3' <<<"$out"
check "dry run shows strict host key checking" grep -q 'StrictHostKeyChecking=yes' <<<"$out"
out="$("$CH" v0.4.3 2>&1)"; rc=$?
check "no key: exit 10, one SKIP line" test "$rc:$out" = "10:SKIP aur: AUR_SSH_KEY not configured"
out="$("$CH" v0.5.0-rc.1 2>&1)"; rc=$?
check "prerelease tag: exit 10" test "$rc" = 10
expect_status "bad tag: failure code 2" 2 "$CH" notatag

# Pinned host key: without the committed known_hosts the channel skips.
export AUR_SSH_KEY="$KEY"
out="$(AUR_KNOWN_HOSTS="$TMP/absent" "$CH" v0.4.3 2>&1)"; rc=$?
check "missing known_hosts: exit 10, SKIP names it" test "$rc:$out" = "10:SKIP aur: packaging/aur/known_hosts not configured"

# The account switch: key and host keys alone do not publish (no AUR account
# exists yet), the variable AUR_ENABLED=true turns the channel on.
printf 'localhost ssh-ed25519 AAAAfixture\n' >"$TMP/kh0"
out="$(AUR_KNOWN_HOSTS="$TMP/kh0" "$CH" v0.4.3 2>&1)"; rc=$?
check "key and host keys but no AUR_ENABLED: clean SKIP naming it" test "$rc:$out" = "10:SKIP aur: AUR_ENABLED not configured"
out="$(AUR_ENABLED=false AUR_KNOWN_HOSTS="$TMP/kh0" "$CH" v0.4.3 2>&1)"; rc=$?
check "AUR_ENABLED=false is a clean SKIP" test "$rc:$out" = "10:SKIP aur: AUR_ENABLED not configured"
check "the committed AUR known_hosts has the three host keys" test "$(grep -c '^aur.archlinux.org \(ssh-ed25519\|ssh-rsa\|ecdsa-sha2-nistp256\) ' "$REPO_ROOT/packaging/aur/known_hosts")" = 3

# AC-e3: push to a local bare repository, fast forward only, rerun is a no-op.
git init -q --bare -b master "$TMP/aur.git"
printf 'localhost ssh-ed25519 AAAAfixture\n' >"$TMP/kh"
export AUR_ENABLED=true AUR_REPO_URL="file://$TMP/aur.git" AUR_KNOWN_HOSTS="$TMP/kh"
out1="$("$CH" v0.4.3 2>&1)"; rc1=$?
n1="$(git -C "$TMP/aur.git" rev-list --count master 2>/dev/null || echo 0)"
out2="$("$CH" v0.4.3 2>&1)"; rc2=$?
n2="$(git -C "$TMP/aur.git" rev-list --count master 2>/dev/null || echo 0)"
check "e3 first run exits 0 with PUBLISHED" test "$rc1:$(grep -c '^PUBLISHED aur: 0.4.3$' <<<"$out1")" = "0:1"
check "e3 first run made 1 commit on master" test "$n1" = 1
check "e3 rerun reports UNCHANGED" test "$rc2:$out2" = "0:UNCHANGED aur: 0.4.3"
check "e3 rerun made 0 new commits" test "$n2" = "$n1"
files_in="$(git -C "$TMP/aur.git" ls-tree --name-only master | tr '\n' ' ')"
check "e3 repository holds .SRCINFO and PKGBUILD" test "$files_in" = ".SRCINFO PKGBUILD "
git -C "$TMP/aur.git" show master:PKGBUILD >"$TMP/p.pkg" 2>/dev/null
check "e3 pushed PKGBUILD equals golden" diff -q "$GOLD/aur-0.4.3/PKGBUILD" "$TMP/p.pkg"
# A second version is a fast forward on top of the first.
SHA256SUMS_FILE="$GOLD/channels/SHA256SUMS-1.2.0" "$CH" v1.2.0 >/dev/null 2>&1
check "e3 next version is a fast forward (2 commits, first is an ancestor)" \
  test "$(git -C "$TMP/aur.git" rev-list --count master)" = 2
check "e3 key never printed" test -z "$(printf '%s\n%s\n' "$out1" "$out2" | grep -F "$KEY")"
check "e3 key file removed after the run" bash -c 'shopt -s nocaseglob nullglob; f=("$1"/*key*); [ "${#f[@]}" -eq 0 ]' _ "$TMP"
check "e3 script never forces a push" test -z "$(grep -E 'git .*push.*(--force| -f |[[:space:]]\+)' "$CH" | grep -v '^ *#')"
check "e3 script pins UserKnownHostsFile and never disables host checks" \
  test -n "$(grep -c 'UserKnownHostsFile' "$CH" | grep -v '^0$')" -a -z "$(grep -E 'StrictHostKeyChecking=(no|accept-new)' "$CH")"

# Hygiene.
files="$CH $GEN $REPO_ROOT/packaging/aur/PKGBUILD.tpl $REPO_ROOT/packaging/aur/SRCINFO.tpl $REPO_ROOT/docs/distribution-channels/aur.md $0"
dash="$(printf '\xe2\x80\x94\|\xe2\x80\x93')"
check "no long dashes" test -z "$(grep -l "$dash" $files 2>/dev/null)"
check "no email address or home path (the AUR ssh URL is not an address)" test -z "$(cat $files 2>/dev/null | grep -v 'ssh://aur@' | grep -E '[[:alnum:]._-]+@[[:alnum:]][[:alnum:]-]*\.[a-z]+|/ho''me/')"
if command -v shellcheck >/dev/null 2>&1; then
  check "shellcheck" shellcheck -S warning -x "$CH" "$GEN"
fi
finish
