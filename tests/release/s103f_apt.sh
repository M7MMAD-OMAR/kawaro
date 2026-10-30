#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103f: the apt repository channel. Offline: `aws` and `curl` are stubs, the
# "bucket" is a local directory, the key is generated here and thrown away.
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
check "S-103f engines.md records apt-ftparchive, GnuPG and the upload tool" \
  bash -c 'grep -q apt-ftparchive "$1" && grep -q GnuPG "$1" && grep -qi "aws cli" "$1"' _ "$REPO_ROOT/docs/engines.md"

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

# Stubs: aws maps s3://bucket/ to a directory, curl records and fails.
BIN="$TMP/bin"; mkdir -p "$BIN"
cat > "$BIN/aws" <<'STUB'
#!/usr/bin/env bash
echo "aws $*" >> "$STUB_LOG"
args=(); skip=0
for a in "$@"; do
  if [ "$skip" = 1 ]; then skip=0; continue; fi
  case "$a" in --endpoint-url) skip=1 ;; --*) ;; *) args+=("$a") ;; esac
done
[ "${args[0]}" = s3 ] || exit 2
map() { case "$1" in "s3://$STUB_BUCKET/"*) echo "$STUB_BUCKET_DIR/${1#"s3://$STUB_BUCKET/"}" ;; s3://*) exit 3 ;; *) echo "$1" ;; esac; }
case "${args[1]}" in
  sync) src="$(map "${args[2]}")"; dst="$(map "${args[3]}")"; mkdir -p "$dst"; [ ! -d "$src" ] || cp -a "$src/." "$dst/" ;;
  cp) src="$(map "${args[2]}")"; dst="$(map "${args[3]}")"; [ -f "$src" ] || exit 1; mkdir -p "$(dirname "$dst")"; cp "$src" "$dst" ;;
  rm) rm -f "$(map "${args[2]}")" ;;
  *) exit 2 ;;
esac
STUB
printf '#!/bin/sh\necho "curl $*" >> "$STUB_LOG"\nexit 22\n' > "$BIN/curl"
chmod +x "$BIN"/*

BUCKET="$TMP/bucket"; mkdir -p "$BUCKET" "$TMP/t"
keyfile="$TMP/seckey.asc"; printf '%s\n' "$SECKEY" > "$keyfile"
export_env=(APT_SIGNING_PASSPHRASE="$PASS_SENT" APT_R2_ACCESS_KEY_ID=ak-sentinel-77 APT_R2_SECRET_ACCESS_KEY=sk-sentinel-88
  APT_R2_ACCOUNT_ID=acct123 APT_R2_BUCKET=kawaro-apt CANONICAL_REPO=OWNER/kawaro APT_PUBLIC_KEY_FILE="$TMP/pub.asc")

run() { # run LOG [VAR=VALUE ...] -- tag args   (APT_SIGNING_KEY always set unless NOKEY=1)
  local name="$1"; shift
  local envs=()
  while [ "$1" != "--" ]; do envs+=("$1"); shift; done
  shift
  : > "$TMP/$name.log"
  OUT="$(cd "$REPO_ROOT" && env "${export_env[@]}" APT_SIGNING_KEY="$SECKEY" "${envs[@]}" \
    PATH="$BIN:$PATH" TMPDIR="$TMP/t" STUB_LOG="$TMP/$name.log" STUB_BUCKET=kawaro-apt STUB_BUCKET_DIR="$BUCKET" \
    "$APT" "$@" 2>&1)"
  RC=$?
}
publish() { run "p$1" "APT_DEB_FILE=$TMP/debs/kawaro_$1_amd64.deb" -- "v$1"; }
leaks() { # leaks TEXT: true when no secret value is in it
  ! grep -qF -e "$PASS_SENT" -e ak-sentinel-77 -e sk-sentinel-88 -e 'PRIVATE KEY' <<<"$1"
}

# Dry run: no secrets needed, nothing external, three ordered phases.
run dry NOKEY=1 APT_SIGNING_KEY= -- v1.0.0 --dry-run
check "S-103f dry run exits 0 without a key" test "$RC" -eq 0
check "S-103f dry run prints the plan line" grep -qxF 'DRYRUN apt: would publish 1.0.0' <<<"$OUT"
check "S-103f dry run lists the three upload phases in order" bash -c '
  a="$(grep -n "phase 1.*pool" <<<"$1" | cut -d: -f1)"; b="$(grep -n "phase 2.*metadata" <<<"$1" | cut -d: -f1)"
  c="$(grep -n "phase 3.*Release" <<<"$1" | cut -d: -f1)"
  [ -n "$a" ] && [ -n "$b" ] && [ -n "$c" ] && [ "$a" -lt "$b" ] && [ "$b" -lt "$c" ]' _ "$OUT"
check "S-103f dry run made no external call" test ! -s "$TMP/dry.log"

# Clean SKIP, one line naming the variable.
run s1 APT_SIGNING_KEY= -- v1.0.0
check "S-103f no key: exit 10 and the exact SKIP line" \
  bash -c '[ "$1" -eq 10 ] && [ "$2" = "SKIP apt: APT_SIGNING_KEY not configured" ]' _ "$RC" "$OUT"
for v in APT_R2_ACCESS_KEY_ID APT_R2_SECRET_ACCESS_KEY APT_R2_ACCOUNT_ID APT_R2_BUCKET CANONICAL_REPO; do
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
check "S-103f ...and nothing was uploaded" bash -c '! grep -q "s3 cp .* s3://" "$1"' _ "$TMP/mm.log"
check "S-103f ...and the failure leaks nothing" leaks "$OUT"

# First real publication.
publish 1.0.0
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

# Phase order in the recorded calls: pool first, Release files last.
L="$TMP/p1.0.0.log"
check "S-103f upload order is pool, then metadata, then Release files" bash -c '
  n() { grep -n "$2" "$1" | head -1 | cut -d: -f1; }
  a="$(n "$1" "s3 cp .*kawaro_1.0.0_amd64.deb")"; b="$(n "$1" "s3 cp .*Packages.gz")"
  c="$(n "$1" "s3 cp .*stable/Release s3")"; d="$(n "$1" "s3 cp .*Release.gpg")"; e="$(n "$1" "s3 cp .*InRelease")"
  [ -n "$a$b$c$d$e" ] && [ "$a" -lt "$b" ] && [ "$b" -lt "$c" ] && [ "$b" -lt "$d" ] && [ "$b" -lt "$e" ]
  last="$(grep -n "s3 cp" "$1" | tail -1)"; grep -q "InRelease\|Release.gpg\|/Release " <<<"$last"' _ "$L"
check "S-103f the R2 endpoint comes from the account variable" grep -q -- '--endpoint-url https://acct123.r2.cloudflarestorage.com' "$L"
check "S-103f no secret in output or recorded calls" bash -c '! grep -qF -e "$1" -e ak-sentinel-77 -e sk-sentinel-88 -e "PRIVATE KEY" "$2" "$3"' _ "$PASS_SENT" "$L" <(printf '%s' "$OUT")
check "S-103f the temporary keyring and work dir are wiped" bash -c '[ -z "$(ls -A "$1")" ]' _ "$TMP/t"

# Idempotence.
before="$(cd "$BUCKET" && find . -type f -print0 | sort -z | xargs -0 sha256sum)"
publish 1.0.0
check "S-103f a second run of the same tag exits 0 with UNCHANGED" \
  bash -c '[ "$1" -eq 0 ] && grep -qxF "UNCHANGED apt: 1.0.0" <<<"$2"' _ "$RC" "$OUT"
check "S-103f ...and uploads nothing" bash -c '! grep -q "s3 cp .* s3://\|s3 rm" "$1"' _ "$TMP/p1.0.0.log"
check "S-103f ...and the bucket is byte for byte the same" \
  bash -c '[ "$1" = "$(cd "$2" && find . -type f -print0 | sort -z | xargs -0 sha256sum)" ]' _ "$before" "$BUCKET"

# Older versions are kept, three at most.
publish 1.0.1; publish 1.0.2
check "S-103f three versions are kept" bash -c 'ls "$1"/pool/main/k/kawaro/ | sort | tr "\n" " " | grep -qx "kawaro_1.0.0_amd64.deb kawaro_1.0.1_amd64.deb kawaro_1.0.2_amd64.deb "' _ "$BUCKET"
publish 1.0.3
check "S-103f the fourth version evicts the oldest from the pool" \
  bash -c 'ls "$1"/pool/main/k/kawaro/ | sort | tr "\n" " " | grep -qx "kawaro_1.0.1_amd64.deb kawaro_1.0.2_amd64.deb kawaro_1.0.3_amd64.deb "' _ "$BUCKET"
check "S-103f Packages indexes exactly the three newest" bash -c '
  [ "$(grep -c "^Version:" "$1")" -eq 3 ] && ! grep -qx "Version: 1.0.0" "$1" && grep -qx "Version: 1.0.3" "$1"' _ "$P"
check "S-103f the old pool object is removed only after the Release files" bash -c '
  r="$(grep -n "s3 rm" "$1" | head -1 | cut -d: -f1)"; c="$(grep -n "s3 cp .*InRelease" "$1" | head -1 | cut -d: -f1)"
  [ -n "$r" ] && [ -n "$c" ] && [ "$r" -gt "$c" ]' _ "$TMP/p1.0.3.log"
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
  'OWNER ACTIONS' 'APT_SIGNING_KEY' 'APT_SIGNING_PASSPHRASE' 'APT_R2_ACCESS_KEY_ID' 'APT_R2_SECRET_ACCESS_KEY' \
  'APT_R2_ACCOUNT_ID' 'APT_R2_BUCKET' 'Kawaro release signing' 'Ubuntu' 'Debian' 'Kali' 'dists/stable/main/binary-amd64' 'pool/'; do
  check "S-103f the doc mentions $s" grep -qF -- "$s" "$DOC"
done
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
