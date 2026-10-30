#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Channel: signed apt repository served by GitHub Pages (S-103f, built on
# S-016a and S-016b). The signed tree is one commit in a public Pages
# repository, pushed over ssh with a deploy key. Run from the repository root.
#   scripts/channels/apt.sh vX.Y.Z [--dry-run]
# Environment (names only, values are never printed):
#   APT_SIGNING_KEY          armored private key (secret, imported into a
#                            temporary keyring that is wiped on exit)
#   APT_SIGNING_PASSPHRASE   passphrase of that key, if it has one (optional)
#   APT_DEPLOY_KEY           private ssh key, a deploy key with write access to
#                            the Pages repository only (secret, written to a
#                            0600 file that an EXIT trap removes)
#   APT_REPO                 OWNER/NAME of the Pages repository (variable)
#   CANONICAL_REPO           OWNER/NAME of this project (release URLs)
#   APT_PUBLIC_KEY_FILE      committed public key (default packaging/keys/kawaro-archive.asc);
#                            its fingerprint must equal the signing key's
#   GITHUB_KNOWN_HOSTS       optional known_hosts path (default
#                            packaging/release/github_known_hosts, GitHub's
#                            published host keys; checked strictly)
#   APT_REPO_URL             optional remote override (tests use a local path)
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
known_hosts="${GITHUB_KNOWN_HOSTS:-$ROOT/packaging/release/github_known_hosts}"
for v in APT_SIGNING_KEY APT_DEPLOY_KEY APT_REPO CANONICAL_REPO; do
  [ -n "${!v:-}" ] || { missing="$v"; break; }
done
if [ -z "$missing" ] && [ ! -s "$pubkey" ]; then missing="packaging/keys/kawaro-archive.asc"; fi
if [ -z "$missing" ] && [ ! -s "$known_hosts" ]; then missing="packaging/release/github_known_hosts"; fi
if [ -n "$missing" ]; then
  if [ "$dry" = 0 ]; then echo "SKIP apt: $missing not configured"; exit 10; fi
  echo "NOTE apt: $missing not configured, a real run would SKIP"
fi
repo="${CANONICAL_REPO:-OWNER/kawaro}"
debname="kawaro_${version}_amd64.deb"
ssh_opts="-o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=${known_hosts/#$ROOT\//}"

work="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-apt.XXXXXX")"
export GNUPGHOME="$work/gnupg"
# shellcheck disable=SC2329  # runs from the EXIT trap
cleanup() {
  gpgconf --kill all >/dev/null 2>&1 || true
  rm -rf -- "$work"
}
trap cleanup EXIT
mkdir -m 700 "$GNUPGHOME"

# The signing key lives only in this temporary keyring. The committed public
# key must belong to it, or nothing is built (also checked by a dry run).
check_signing_key() {
  printf '%s\n' "$APT_SIGNING_KEY" | gpg --batch --import >/dev/null 2>&1 \
    || { echo "apt: APT_SIGNING_KEY could not be imported" >&2; exit 1; }
  fpr="$(gpg --batch --with-colons --list-secret-keys 2>/dev/null | awk -F: '/^fpr/ { print $10; exit }')"
  [ -n "$fpr" ] || { echo "apt: no secret key in APT_SIGNING_KEY" >&2; exit 1; }
  pubfpr="$(gpg --batch --with-colons --show-keys "$pubkey" 2>/dev/null | awk -F: '/^fpr/ { print $10; exit }')"
  [ "$pubfpr" = "$fpr" ] || { echo "apt: the committed public key does not match the signing key" >&2; exit 1; }
  export APT_SIGN_FPR="$fpr"
}

if [ "$dry" = 1 ]; then
  echo "DRYRUN apt: would publish $version"
  echo "DRYRUN apt: would build the signed tree (pool, Packages, Release, Release.gpg, InRelease, keyring) and push it as one commit to main of ${APT_REPO:-APT_REPO}, never forced"
  echo "DRYRUN apt: ssh options: $ssh_opts"
  echo "DRYRUN apt: pool keeps the 3 newest versions; an older one is removed in the same commit"
  if [ -n "${APT_SIGNING_KEY:-}" ] && [ -s "$pubkey" ]; then
    check_signing_key
    echo "DRYRUN apt: signing key matches the committed public key $fpr"
  fi
  exit 0
fi

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

check_signing_key

# Push access: the deploy key goes to a private file, host keys are strict.
umask 077
keyfile="$work/id_key"
printf '%s\n' "$APT_DEPLOY_KEY" >"$keyfile"
export GIT_SSH_COMMAND="ssh -i $keyfile -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$known_hosts"
export GIT_TERMINAL_PROMPT=0
export GIT_AUTHOR_NAME=kawaro-release-bot GIT_COMMITTER_NAME=kawaro-release-bot
export GIT_AUTHOR_EMAIL=
export GIT_COMMITTER_EMAIL=
ssh_user=git # the fixed GitHub ssh account name, not a person
url="${APT_REPO_URL:-ssh://${ssh_user}@github.com/$APT_REPO.git}"

# Existing state: the pool (for retention) and the current Packages (for idempotence).
repo_dir="$work/pages"
git clone --quiet "$url" "$repo_dir" 2>&1 | grep -v '^warning: You appear to have cloned an empty repository' || true
[ -d "$repo_dir/.git" ] || { echo "apt: clone of the Pages repository failed" >&2; exit 1; }
git -C "$repo_dir" checkout --quiet -B main
poolfile="pool/main/k/kawaro/$debname"
if [ -f "$repo_dir/dists/stable/main/binary-amd64/Packages" ] \
  && grep -qx "Version: $version" "$repo_dir/dists/stable/main/binary-amd64/Packages" \
  && [ -f "$repo_dir/$poolfile" ] && cmp -s "$deb" "$repo_dir/$poolfile"; then
  echo "UNCHANGED apt: $version"
  exit 0
fi

"$ROOT/packaging/apt/build-repo.sh" "$deb" "$repo_dir" --keep 3 >/dev/null
if [ ! -f "$repo_dir/$poolfile" ]; then
  echo "UNCHANGED apt: $version is older than the 3 newest versions kept"
  exit 0
fi
# Pages serves the files as they are; no Jekyll processing.
: >"$repo_dir/.nojekyll"

# One commit holds the pool, the metadata and the signed Release files, so a
# reader never sees a half published index.
git -C "$repo_dir" add -A
if git -C "$repo_dir" diff --cached --quiet; then
  echo "UNCHANGED apt: $version"
  exit 0
fi
git -C "$repo_dir" commit --quiet -m "kawaro: publish $version"
git -C "$repo_dir" push --quiet origin HEAD:main
echo "PUBLISHED apt: $version"
