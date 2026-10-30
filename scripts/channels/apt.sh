#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Channel: signed apt repository on S3 compatible storage, Cloudflare R2
# (S-103f, built on S-016a and S-016b). Run from the repository root.
#   scripts/channels/apt.sh vX.Y.Z [--dry-run]
# Environment (names only, values are never printed):
#   APT_SIGNING_KEY          armored private key (secret, imported into a
#                            temporary keyring that is wiped on exit)
#   APT_SIGNING_PASSPHRASE   passphrase of that key, if it has one
#   APT_R2_ACCESS_KEY_ID     object storage credentials, limited to the bucket
#   APT_R2_SECRET_ACCESS_KEY
#   APT_R2_ACCOUNT_ID        builds the endpoint https://ID.r2.cloudflarestorage.com
#   APT_R2_BUCKET            bucket name
#   CANONICAL_REPO           OWNER/NAME of this project (release URLs)
#   APT_PUBLIC_KEY_FILE      committed public key (default packaging/keys/kawaro-archive.asc);
#                            its fingerprint must equal the signing key's
#   APT_DEB_FILE             optional local DEB (default dist/, else the release asset)
#   SHA256SUMS_FILE          optional local SHA256SUMS used to check a downloaded DEB
# Exit 0 done, 10 clean SKIP (not configured, or prerelease), other = failure.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
dry=0 tag=""
for a in "$@"; do
  case "$a" in
    --dry-run) dry=1 ;;
    v*) tag="$a" ;;
    *) echo "usage: apt.sh vX.Y.Z [--dry-run]" >&2; exit 2 ;;
  esac
done
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || { echo "apt: bad tag '$tag'" >&2; exit 2; }
version="${tag#v}"
[[ "$tag" != *-* ]] || { echo "SKIP apt: prerelease tag"; exit 10; }

pubkey="${APT_PUBLIC_KEY_FILE:-$ROOT/packaging/keys/kawaro-archive.asc}"
missing=""
for v in APT_SIGNING_KEY APT_R2_ACCESS_KEY_ID APT_R2_SECRET_ACCESS_KEY APT_R2_ACCOUNT_ID APT_R2_BUCKET CANONICAL_REPO; do
  [ -n "${!v:-}" ] || { missing="$v"; break; }
done
if [ -z "$missing" ] && [ ! -s "$pubkey" ]; then missing="packaging/keys/kawaro-archive.asc"; fi
if [ -n "$missing" ]; then
  if [ "$dry" = 0 ]; then echo "SKIP apt: $missing not configured"; exit 10; fi
  echo "NOTE apt: $missing not configured, a real run would SKIP"
fi
repo="${CANONICAL_REPO:-OWNER/kawaro}"
bucket="${APT_R2_BUCKET:-BUCKET}"
debname="kawaro_${version}_amd64.deb"

if [ "$dry" = 1 ]; then
  echo "DRYRUN apt: would publish $version"
  echo "DRYRUN apt: phase 1 pool: upload pool/main/k/kawaro/$debname"
  echo "DRYRUN apt: phase 2 metadata: upload Packages, Packages.gz, binary-amd64 Release, by-hash entries, keyring"
  echo "DRYRUN apt: phase 3 Release files last: upload Release, Release.gpg, InRelease"
  echo "DRYRUN apt: then remove pool objects older than the 3 newest versions from s3://$bucket"
  exit 0
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-apt.XXXXXX")"
export GNUPGHOME="$work/gnupg"
# shellcheck disable=SC2329  # runs from the EXIT trap
cleanup() {
  gpgconf --kill all >/dev/null 2>&1 || true
  rm -rf -- "$work"
}
trap cleanup EXIT
mkdir -m 700 "$GNUPGHOME"

command -v aws >/dev/null 2>&1 || { echo "apt: the aws command line tool is required" >&2; exit 1; }

# The DEB.
deb="${APT_DEB_FILE:-}"
if [ -z "$deb" ] && [ -f "$ROOT/dist/$debname" ]; then deb="$ROOT/dist/$debname"; fi
if [ -z "$deb" ]; then
  deb="$work/$debname"
  curl -fsSL --retry 3 -o "$deb" "https://github.com/$repo/releases/download/$tag/$debname"
  sums="${SHA256SUMS_FILE:-}"
  if [ -z "$sums" ] && [ -f "$ROOT/dist/SHA256SUMS" ]; then sums="$ROOT/dist/SHA256SUMS"; fi
  if [ -z "$sums" ]; then
    sums="$work/SHA256SUMS"
    curl -fsSL --retry 3 -o "$sums" "https://github.com/$repo/releases/download/$tag/SHA256SUMS"
  fi
  want="$(awk -v n="$debname" '$2 == n || $2 == "*" n { print $1 }' "$sums")"
  [ -n "$want" ] && [ "$(sha256sum "$deb" | cut -d' ' -f1)" = "$want" ] \
    || { echo "apt: $debname does not match SHA256SUMS" >&2; exit 1; }
fi
[ "$(dpkg-deb -f "$deb" Version)" = "$version" ] || { echo "apt: the DEB version is not $version" >&2; exit 1; }

# The signing key lives only in this temporary keyring.
printf '%s\n' "$APT_SIGNING_KEY" | gpg --batch --import >/dev/null 2>&1 \
  || { echo "apt: APT_SIGNING_KEY could not be imported" >&2; exit 1; }
fpr="$(gpg --batch --with-colons --list-secret-keys 2>/dev/null | awk -F: '/^fpr/ { print $10; exit }')"
[ -n "$fpr" ] || { echo "apt: no secret key in APT_SIGNING_KEY" >&2; exit 1; }
pubfpr="$(gpg --batch --with-colons --show-keys "$pubkey" 2>/dev/null | awk -F: '/^fpr/ { print $10; exit }')"
[ "$pubfpr" = "$fpr" ] || { echo "apt: the committed public key does not match the signing key" >&2; exit 1; }
export APT_SIGN_FPR="$fpr"

# Storage access: credentials only through the environment, never arguments.
export AWS_ACCESS_KEY_ID="$APT_R2_ACCESS_KEY_ID" AWS_SECRET_ACCESS_KEY="$APT_R2_SECRET_ACCESS_KEY"
export AWS_DEFAULT_REGION=auto AWS_REQUEST_CHECKSUM_CALCULATION=when_required AWS_RESPONSE_CHECKSUM_VALIDATION=when_required
s3() { aws --endpoint-url "https://$APT_R2_ACCOUNT_ID.r2.cloudflarestorage.com" s3 "$@"; }
remote="s3://$bucket"

# Existing state: the pool (for retention) and the current Packages (for idempotence).
repo_dir="$work/repo"
mkdir -p "$repo_dir/pool"
s3 sync "$remote/pool/" "$repo_dir/pool/" --only-show-errors
poolfile="pool/main/k/kawaro/$debname"
if s3 cp "$remote/dists/stable/main/binary-amd64/Packages" "$work/Packages.old" --only-show-errors >/dev/null 2>&1 \
  && grep -qx "Version: $version" "$work/Packages.old" \
  && [ -f "$repo_dir/$poolfile" ] && cmp -s "$deb" "$repo_dir/$poolfile"; then
  echo "UNCHANGED apt: $version"
  exit 0
fi

pruned="$("$ROOT/packaging/apt/build-repo.sh" "$deb" "$repo_dir" --keep 3 | sed -n 's/^PRUNED //p')"
if [ ! -f "$repo_dir/$poolfile" ]; then
  echo "UNCHANGED apt: $version is older than the 3 newest versions kept"
  exit 0
fi

# Phase 1: pool. Phase 2: metadata. Phase 3: Release files last. Then prune.
up() { s3 cp "$repo_dir/$1" "$remote/$1" --only-show-errors; }
up "$poolfile"
idx="dists/stable/main/binary-amd64"
up "$idx/Packages"
up "$idx/Packages.gz"
up "$idx/Release"
for f in "$repo_dir/$idx/by-hash/SHA256/"*; do up "$idx/by-hash/SHA256/${f##*/}"; done
up kawaro-archive-keyring.gpg
up kawaro-archive.asc
up dists/stable/Release
up dists/stable/Release.gpg
up dists/stable/InRelease
for p in $pruned; do s3 rm "$remote/$p" --only-show-errors; done
echo "PUBLISHED apt: $version"
