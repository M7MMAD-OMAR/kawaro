#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103f: the apt repository channel. Offline: `curl` is a stub, the Pages
# repository is a local bare git repository (the channel pushes one commit),
# the keys are generated here and thrown away.
# Needs gpg, gpgv, dpkg-deb, apt-ftparchive; apt-get is used when present.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

APT="$REPO_ROOT/scripts/channels/apt.sh"
BUILD="$REPO_ROOT/packaging/apt/build-repo.sh"
DOC="$REPO_ROOT/docs/distribution-channels/apt.md"
PASS_SENT='sentinel-apt-passphrase-4c1e'

check "S-103f apt.sh exists and is executable" test -x "$APT"
check "S-103f build-repo.sh exists and is executable" test -x "$BUILD"
check "S-103f apt.sh passes shellcheck" shellcheck -S warning -x "$APT"
check "S-103f build-repo.sh passes shellcheck" shellcheck -S warning -x "$BUILD"
check "S-103f the test is part of run_all" grep -qw s103f_apt "$REPO_ROOT/tests/release/run_all.sh"
check "S-103f engines.md records apt-ftparchive, GnuPG and the push tool" \
  bash -c 'grep -q apt-ftparchive "$1" && grep -q GnuPG "$1" && grep -qi "openssh" "$1" && ! grep -qi "aws cli" "$1"' _ "$REPO_ROOT/docs/engines.md"
KH="$REPO_ROOT/packaging/release/github_known_hosts"
check "S-103f the committed GitHub known_hosts has the ed25519 host key" grep -Eq '^github\.com ssh-ed25519 AAAA' "$KH"
check "S-103f apt.sh uses strict host key checking and the committed file" \
  bash -c 'grep -q "StrictHostKeyChecking=yes" "$1" && grep -q github_known_hosts "$1" && ! grep -q "StrictHostKeyChecking=no\|accept-new" "$1"' _ "$APT"
check "S-103f apt.sh no longer uses object storage" bash -c '! grep -qiE "aws |r2|s3://" "$1"' _ "$APT"

# A throwaway passphrase protected key, never a real one.
GH="$TMP/gh"; mkdir -m 700 "$GH"
GNUPGHOME="$GH" gpg --batch --pinentry-mode loopback --passphrase "$PASS_SENT" \
  --quick-gen-key 'Kawaro release signing' ed25519 sign never >/dev/null 2>&1
SECKEY="$(GNUPGHOME="$GH" gpg --batch --pinentry-mode loopback --passphrase "$PASS_SENT" --armor --export-secret-keys 2>/dev/null)"
GNUPGHOME="$GH" gpg --batch --armor --export > "$TMP/pub.asc" 2>/dev/null
FPR="$(GNUPGHOME="$GH" gpg --with-colons --list-keys 2>/dev/null | awk -F: '/^fpr/{print $10; exit}')"
# A second key, to prove a mismatching public key is refused.
GH2="$TMP/gh2"; mkdir -m 700 "$GH2"
GNUPGHOME="$GH2" gpg --batch --passphrase '' --quick-gen-key 'Other key' ed25519 sign never >/dev/null 2>&1
GNUPGHOME="$GH2" gpg --batch --armor --export > "$TMP/other.asc" 2>/dev/null
gpg --dearmor < "$TMP/other.asc" > "$TMP/other-keyring.gpg" 2>/dev/null
check "S-103f the throwaway key exists" test -n "$FPR" -a -n "$SECKEY"

# Fixture DEBs, tiny.
mkdeb() { # mkdeb VERSION
  local d="$TMP/deb-$1"; mkdir -p "$d/DEBIAN" "$d/usr/bin"
  printf 'Package: kawaro\nVersion: %s\nArchitecture: amd64\nMaintainer: Kawaro\nDescription: fixture\n' "$1" > "$d/DEBIAN/control"
  printf '#!/bin/sh\necho %s\n' "$1" > "$d/usr/bin/kawaro"; chmod 755 "$d/usr/bin/kawaro"
  mkdir -p "$TMP/debs"
  dpkg-deb --root-owner-group --build "$d" "$TMP/debs/kawaro_$1_amd64.deb" >/dev/null
}
for v in 1.0.0 1.0.1 1.0.2 1.0.3; do mkdeb "$v"; done

# Stub: curl records and fails (nothing may be downloaded in these runs).
BIN="$TMP/bin"; mkdir -p "$BIN"
printf '#!/bin/sh\necho "curl $*" >> "$STUB_LOG"\nexit 22\n' > "$BIN/curl"
chmod +x "$BIN"/*

# The Pages repository: a local bare repository with the README commit a new
# GitHub repository starts with. BUCKET is a fresh checkout of it.
PAGES="$TMP/pages.git"
git init -q --bare -b main "$PAGES"
git clone -q "$PAGES" "$TMP/seed" 2>/dev/null
( cd "$TMP/seed" && git config user.name t && git config user.email t@t.invalid \
  && echo "# kawaro-apt" > README.md && git add README.md && git commit -q -m init && git push -q origin HEAD:main )
BUCKET="$TMP/bucket"; mkdir -p "$TMP/t"
snap() { rm -rf "$BUCKET"; git clone -q "$PAGES" "$BUCKET" 2>/dev/null; }
ncommits() { git -C "$PAGES" rev-list --count main; }
keyfile="$TMP/seckey.asc"; printf '%s\n' "$SECKEY" > "$keyfile"
DEPLOY_SENT='sentinel-deploy-key-9f2a'
export_env=(APT_SIGNING_PASSPHRASE="$PASS_SENT" APT_DEPLOY_KEY="$DEPLOY_SENT" APT_REPO=OWNER/kawaro-apt
  APT_REPO_URL="file://$PAGES" CANONICAL_REPO=OWNER/kawaro APT_PUBLIC_KEY_FILE="$TMP/pub.asc")

run() { # run LOG [VAR=VALUE ...] -- tag args   (APT_SIGNING_KEY always set unless NOKEY=1)
  local name="$1"; shift
  local envs=()
  while [ "$1" != "--" ]; do envs+=("$1"); shift; done
  shift
  : > "$TMP/$name.log"
  OUT="$(cd "$REPO_ROOT" && env "${export_env[@]}" APT_SIGNING_KEY="$SECKEY" "${envs[@]}" \
    PATH="$BIN:$PATH" TMPDIR="$TMP/t" STUB_LOG="$TMP/$name.log" \
    "$APT" "$@" 2>&1)"
  RC=$?
}
publish() { run "p$1" "APT_DEB_FILE=$TMP/debs/kawaro_$1_amd64.deb" -- "v$1"; }
leaks() { # leaks TEXT: true when no secret value is in it
  ! grep -qF -e "$PASS_SENT" -e "$DEPLOY_SENT" -e 'PRIVATE KEY' <<<"$1"
}

# Dry run: no secrets needed, nothing external, three ordered phases.
run dry NOKEY=1 APT_SIGNING_KEY= -- v1.0.0 --dry-run
check "S-103f dry run exits 0 without a key" test "$RC" -eq 0
check "S-103f dry run prints the plan line" grep -qxF 'DRYRUN apt: would publish 1.0.0' <<<"$OUT"
check "S-103f dry run says it commits one atomic change to the Pages repository over ssh" \
  bash -c 'grep -q "DRYRUN apt: .*one commit" <<<"$1" && grep -q "StrictHostKeyChecking=yes" <<<"$1" && grep -q github_known_hosts <<<"$1"' _ "$OUT"
check "S-103f dry run keeps the 3 newest versions" grep -q 'DRYRUN apt: .*3 newest' <<<"$OUT"
check "S-103f dry run made no external call" test ! -s "$TMP/dry.log"

# A dry run that has the signing key also checks it against the committed public key.
run dk -- v1.0.0 --dry-run
check "S-103f dry run with the key confirms the fingerprint match" \
  bash -c '[ "$1" -eq 0 ] && grep -qxF "DRYRUN apt: signing key matches the committed public key $2" <<<"$3"' _ "$RC" "$FPR" "$OUT"
check "S-103f ...and leaks nothing" leaks "$OUT"
run dkm APT_PUBLIC_KEY_FILE="$TMP/other.asc" -- v1.0.0 --dry-run
check "S-103f dry run with a mismatching public key fails" bash -c '[ "$1" -ne 0 ] && [ "$1" -ne 10 ]' _ "$RC"

# Clean SKIP, one line naming the variable.
run s1 APT_SIGNING_KEY= -- v1.0.0
check "S-103f no key: exit 10 and the exact SKIP line" \
  bash -c '[ "$1" -eq 10 ] && [ "$2" = "SKIP apt: APT_SIGNING_KEY not configured" ]' _ "$RC" "$OUT"
for v in APT_DEPLOY_KEY APT_REPO CANONICAL_REPO; do
  run s2 "$v=" -- v1.0.0
  check "S-103f missing $v: exit 10 naming it" \
    bash -c '[ "$1" -eq 10 ] && [ "$2" = "SKIP apt: $3 not configured" ]' _ "$RC" "$OUT" "$v"
done
run s3 APT_PUBLIC_KEY_FILE="$TMP/nonexistent.asc" -- v1.0.0
check "S-103f no committed public key: exit 10 naming the file" \
  bash -c '[ "$1" -eq 10 ] && grep -q "^SKIP apt: .*kawaro-archive.asc\|^SKIP apt: APT_PUBLIC_KEY_FILE" <<<"$2"' _ "$RC" "$OUT"
run s4 -- v1.0.0-beta.1
check "S-103f prerelease: exit 10 and SKIP line" \
  bash -c '[ "$1" -eq 10 ] && [ "$2" = "SKIP apt: prerelease tag" ]' _ "$RC" "$OUT"
run s5 -- 1.0
check "S-103f malformed tag is a failure, not a skip" bash -c '[ "$1" -ne 0 ] && [ "$1" -ne 10 ]' _ "$RC"
check "S-103f a skip made no external call" test ! -s "$TMP/s2.log"

# A public key that does not belong to the signing key is refused before any upload.
run mm APT_PUBLIC_KEY_FILE="$TMP/other.asc" "APT_DEB_FILE=$TMP/debs/kawaro_1.0.0_amd64.deb" -- v1.0.0
check "S-103f a mismatching committed public key fails" bash -c '[ "$1" -ne 0 ] && [ "$1" -ne 10 ]' _ "$RC"
check "S-103f ...and nothing was pushed" test "$(ncommits)" -eq 1
check "S-103f ...and the failure leaks nothing" leaks "$OUT"

# First real publication.
publish 1.0.0
snap
check "S-103f first publication is exactly one new commit" test "$(ncommits)" -eq 2
check "S-103f first publication exits 0" test "$RC" -eq 0
check "S-103f first publication prints PUBLISHED" grep -qxF 'PUBLISHED apt: 1.0.0' <<<"$OUT"
for f in dists/stable/InRelease dists/stable/Release dists/stable/Release.gpg \
  dists/stable/main/binary-amd64/Packages dists/stable/main/binary-amd64/Packages.gz \
  dists/stable/main/binary-amd64/Release pool/main/k/kawaro/kawaro_1.0.0_amd64.deb \
  kawaro-archive-keyring.gpg kawaro-archive.asc; do
  check "S-103f the layout has $f" test -s "$BUCKET/$f"
done
check "S-103f by-hash entries exist for Packages and Packages.gz" bash -c '
  d="$1/dists/stable/main/binary-amd64"; h1="$(sha256sum "$d/Packages" | cut -d" " -f1)"; h2="$(sha256sum "$d/Packages.gz" | cut -d" " -f1)"
  test -s "$d/by-hash/SHA256/$h1" && test -s "$d/by-hash/SHA256/$h2"' _ "$BUCKET"
R="$BUCKET/dists/stable/Release"
for l in 'Origin: Kawaro' 'Suite: stable' 'Codename: stable' 'Architectures: amd64' 'Components: main' 'Acquire-By-Hash: yes'; do
  check "S-103f Release has '$l'" grep -qxF "$l" "$R"
done
check "S-103f Release hashes Packages and Packages.gz" \
  bash -c 'grep -q "main/binary-amd64/Packages$" "$1" && grep -q "main/binary-amd64/Packages.gz$" "$1"' _ "$R"
check "S-103f Release does not list by-hash entries" bash -c '! grep -q by-hash "$1"' _ "$R"
P="$BUCKET/dists/stable/main/binary-amd64/Packages"
D="$BUCKET/pool/main/k/kawaro/kawaro_1.0.0_amd64.deb"
check "S-103f Packages indexes the DEB with its Filename and SHA256" bash -c '
  grep -qx "Package: kawaro" "$1" && grep -qx "Version: 1.0.0" "$1" &&
  grep -qx "Filename: pool/main/k/kawaro/kawaro_1.0.0_amd64.deb" "$1" &&
  grep -qx "SHA256: $(sha256sum "$2" | cut -d" " -f1)" "$1"' _ "$P" "$D"
check "S-103f Release.gpg verifies with gpgv and the published keyring" \
  gpgv --quiet --keyring "$BUCKET/kawaro-archive-keyring.gpg" "$BUCKET/dists/stable/Release.gpg" "$R" 2>/dev/null
check "S-103f InRelease verifies with gpgv and the published keyring" \
  gpgv --quiet --keyring "$BUCKET/kawaro-archive-keyring.gpg" "$BUCKET/dists/stable/InRelease" 2>/dev/null
check "S-103f the keyring holds the signing key" \
  bash -c 'gpg --show-keys --with-colons "$1" 2>/dev/null | grep -q "^fpr:::::::::$2:"' _ "$BUCKET/kawaro-archive-keyring.gpg" "$FPR"
check "S-103f the published armored key is the committed one" cmp -s "$BUCKET/kawaro-archive.asc" "$TMP/pub.asc"
cp "$R" "$TMP/Release.tampered"; printf 'x\n' >> "$TMP/Release.tampered"
check "S-103f a modified Release fails gpgv" bash -c '! gpgv --keyring "$1" "$2" "$3" >/dev/null 2>&1' _ \
  "$BUCKET/kawaro-archive-keyring.gpg" "$BUCKET/dists/stable/Release.gpg" "$TMP/Release.tampered"
check "S-103f another key does not verify" bash -c '! gpgv --keyring "$1" "$2" "$3" >/dev/null 2>&1' _ \
  "$TMP/other-keyring.gpg" "$BUCKET/dists/stable/Release.gpg" "$R"

# One atomic commit holds the pool object, the metadata and the Release files.
L="$TMP/p1.0.0.log"
check "S-103f the one commit holds pool, metadata and Release files together" bash -c '
  f="$(git -C "$1" show --name-only --format= HEAD)"
  grep -q "^pool/main/k/kawaro/kawaro_1.0.0_amd64.deb$" <<<"$f" && grep -q "binary-amd64/Packages.gz$" <<<"$f" && grep -q "^dists/stable/InRelease$" <<<"$f"' _ "$BUCKET"
check "S-103f the Pages tree has .nojekyll" test -e "$BUCKET/.nojekyll"
check "S-103f the commit has no personal identity" bash -c '! git -C "$1" log -1 --format="%an %ae %cn %ce" | grep -qE "@[A-Za-z0-9.-]+\.[a-z]{2,}"' _ "$BUCKET"
check "S-103f the push was never forced" bash -c '! grep -E "git .*push" "$1" | grep -qE -- "--force|-f |[[:space:]]\+"' _ "$APT"
check "S-103f no secret in output or recorded calls" bash -c '! grep -qF -e "$1" -e "$4" -e "PRIVATE KEY" "$2" "$3"' _ "$PASS_SENT" "$L" <(printf '%s' "$OUT") "$DEPLOY_SENT"
check "S-103f no secret was committed to the Pages repository" bash -c '! git -C "$1" grep -qF -e "$2" -e "$3" -e "PRIVATE KEY" $(git -C "$1" rev-list --all)' _ "$BUCKET" "$PASS_SENT" "$DEPLOY_SENT"
check "S-103f the temporary keyring and work dir are wiped" bash -c '[ -z "$(ls -A "$1")" ]' _ "$TMP/t"

# Idempotence.
before="$(ncommits)"
publish 1.0.0
check "S-103f a second run of the same tag exits 0 with UNCHANGED" \
  bash -c '[ "$1" -eq 0 ] && grep -qxF "UNCHANGED apt: 1.0.0" <<<"$2"' _ "$RC" "$OUT"
check "S-103f ...and pushes nothing, no new commit" test "$(ncommits)" = "$before"

# Older versions are kept, three at most.
publish 1.0.1; publish 1.0.2; snap
check "S-103f three versions are kept" bash -c 'ls "$1"/pool/main/k/kawaro/ | sort | tr "\n" " " | grep -qx "kawaro_1.0.0_amd64.deb kawaro_1.0.1_amd64.deb kawaro_1.0.2_amd64.deb "' _ "$BUCKET"
publish 1.0.3
snap
check "S-103f the fourth version evicts the oldest from the pool" \
  bash -c 'ls "$1"/pool/main/k/kawaro/ | sort | tr "\n" " " | grep -qx "kawaro_1.0.1_amd64.deb kawaro_1.0.2_amd64.deb kawaro_1.0.3_amd64.deb "' _ "$BUCKET"
check "S-103f Packages indexes exactly the three newest" bash -c '
  [ "$(grep -c "^Version:" "$1")" -eq 3 ] && ! grep -qx "Version: 1.0.0" "$1" && grep -qx "Version: 1.0.3" "$1"' _ "$P"
check "S-103f the old pool object is removed in the same commit as the new index" bash -c '
  f="$(git -C "$1" show --no-renames --name-status --format= HEAD)"
  grep -q "^D.pool/main/k/kawaro/kawaro_1.0.0_amd64.deb$" <<<"$f" && grep -q "^A.pool/main/k/kawaro/kawaro_1.0.3_amd64.deb$" <<<"$f" && grep -q "InRelease$" <<<"$f"' _ "$BUCKET"
check "S-103f the final signatures still verify" \
  gpgv --quiet --keyring "$BUCKET/kawaro-archive-keyring.gpg" "$BUCKET/dists/stable/InRelease" 2>/dev/null

# apt itself parses the signed index (when apt-get exists).
if command -v apt-get >/dev/null 2>&1 && { [ -d /usr/libexec/apt/methods ] || [ -d /usr/lib/apt/methods ]; }; then
  A="$TMP/apt"; mkdir -p "$A/state/lists/partial" "$A/cache/archives/partial" "$A/src"; : > "$A/status"
  mksrc() { # mksrc KEYRING
    cat > "$A/src/kawaro.sources" <<SRC
Types: deb
URIs: file://$BUCKET
Suites: stable
Components: main
Architectures: amd64
Signed-By: $1
SRC
  }
  aptq() { apt-get -o Dir::Etc::sourcelist=/dev/null -o Dir::Etc::sourceparts="$A/src" -o Dir::State="$A/state" \
    -o Dir::Cache="$A/cache" -o Dir::State::status="$A/status" -o APT::Architecture=amd64 -o Debug::NoLocking=1 "$@"; }
  mksrc "$BUCKET/kawaro-archive-keyring.gpg"
  aptout="$(aptq update 2>&1)"; arc=$?
  check "S-103f apt-get update accepts the signed repository" test "$arc" -eq 0
  check "S-103f apt-get update prints no warning" bash -c '! grep -q "^W:\|NO_PUBKEY\|not signed\|skipped OpenPGP" <<<"$1"' _ "$aptout"
  check "S-103f apt-cache sees kawaro 1.0.3 as the candidate" \
    bash -c 'apt-cache -o Dir::Etc::sourcelist=/dev/null -o Dir::Etc::sourceparts="$1/src" -o Dir::State="$1/state" -o Dir::Cache="$1/cache" -o Dir::State::status="$1/status" -o APT::Architecture=amd64 policy kawaro | grep -q "Candidate: 1.0.3"' _ "$A"
  mksrc "$TMP/pub.asc"
  check "S-103f the armored key also works as Signed-By" bash -c '! aptq update 2>&1 | grep -q "^W:\|^E:"' 
  mksrc "$GH2/pubring.kbx"
  check "S-103f a different key makes apt-get update fail closed" bash -c '! aptq update >/dev/null 2>&1' 
else
  echo "SKIP S-103f apt-get is not installed (index checked as files above)"
fi

# Docs.
check "S-103f the doc exists" test -f "$DOC"
for s in '/etc/apt/keyrings/' 'Signed-By:' 'signed-by=' '.sources' 'kawaro-archive-keyring.gpg' 'OWNER' \
  'APT_SIGNING_KEY' 'APT_SIGNING_PASSPHRASE' 'APT_DEPLOY_KEY' 'APT_REPO' 'github.io' 'passphrase' 'Kawaro release signing' 'Ubuntu' 'Debian' 'Kali' 'dists/stable/main/binary-amd64' 'pool/'; do
  check "S-103f the doc mentions $s" grep -qF -- "$s" "$DOC"
done
check "S-103f the doc has no owner action list and no object storage" \
  bash -c '! grep -qE "OWNER ACTIONS|R2|bucket" "$1"' _ "$DOC"
check "S-103f the doc documents the manual Debian and Kali inclusion path" \
  bash -c 'grep -qi "mentors" "$1" && grep -qi "manual" "$1"' _ "$DOC"
check "S-103f the doc never allows an unauthenticated install" \
  bash -c '! grep -qiE "trusted=yes|allow-unauthenticated|allow-insecure" "$1"' _ "$DOC"

# Privacy and dash lint over every file this part owns.
LONG_DASH="$(printf '\342\200\224\|\342\200\223')"
for f in "$APT" "$BUILD" "$DOC"; do
  check "S-103f no long dash in $(basename "$f")" bash -c '! grep -q "$2" "$1"' _ "$f" "$LONG_DASH"
  check "S-103f no email, home path or key block in $(basename "$f")" \
    bash -c '! grep -qE "[A-Za-z0-9._-]+@[A-Za-z0-9.-]+\.[a-z]{2,}|/home/|BEGIN [A-Z ]*PRIVATE KEY" "$1"' _ "$f"
done
finish
