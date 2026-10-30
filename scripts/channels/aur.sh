#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Channel: AUR package kawaro-bin (S-103e). Run from the repository root.
#   scripts/channels/aur.sh vX.Y.Z [--dry-run]
# Environment (names only, values are never printed):
#   AUR_SSH_KEY       private SSH key registered on the AUR account
#   CANONICAL_REPO    OWNER/NAME of this project (release URLs)
#   SHA256SUMS_FILE   optional local SHA256SUMS (default dist/SHA256SUMS,
#                     else downloaded from the release)
#   AUR_KNOWN_HOSTS   optional known_hosts path (default packaging/aur/known_hosts,
#                     committed after the owner verified the AUR host keys)
#   AUR_REPO_URL      optional remote override (tests use a local path)
# Exit 0 done, 10 clean SKIP (not configured, or prerelease), other = failure.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
dry=0 tag=""
for a in "$@"; do
  case "$a" in
    --dry-run) dry=1 ;;
    v*) tag="$a" ;;
    *) echo "usage: aur.sh vX.Y.Z [--dry-run]" >&2; exit 2 ;;
  esac
done
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || { echo "aur: bad tag '$tag'" >&2; exit 2; }
version="${tag#v}"
[[ "$tag" != *-* ]] || { echo "SKIP aur: prerelease tag"; exit 10; }

known_hosts="${AUR_KNOWN_HOSTS:-$ROOT/packaging/aur/known_hosts}"
missing=""
if [ -z "${AUR_SSH_KEY:-}" ]; then missing="AUR_SSH_KEY"
elif [ -z "${CANONICAL_REPO:-}" ]; then missing="CANONICAL_REPO"
elif [ ! -s "$known_hosts" ]; then missing="packaging/aur/known_hosts"
fi
if [ -n "$missing" ]; then
  if [ "$dry" = 0 ]; then echo "SKIP aur: $missing not configured"; exit 10; fi
  echo "NOTE aur: $missing not configured, a real run would SKIP"
fi
repo="${CANONICAL_REPO:-OWNER/kawaro}"

work="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-aur.XXXXXX")"
trap 'rm -rf -- "$work"' EXIT

sums="${SHA256SUMS_FILE:-}"
if [ -z "$sums" ] && [ -f "$ROOT/dist/SHA256SUMS" ]; then sums="$ROOT/dist/SHA256SUMS"; fi
if [ -z "$sums" ]; then
  sums="$work/SHA256SUMS"
  curl -fsSL --retry 3 -o "$sums" "https://github.com/$repo/releases/download/$tag/SHA256SUMS"
fi
"$ROOT/packaging/aur/generate.sh" "$tag" "$sums" "$repo" "$work/out"

ssh_opts="-o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$known_hosts"
if [ "$dry" = 1 ]; then
  echo "DRYRUN aur: would publish $version"
  echo "DRYRUN aur: would push PKGBUILD and .SRCINFO to master of kawaro-bin, no force, ssh options: $ssh_opts"
  exit 0
fi

umask 077
keyfile="$work/id_key"
printf '%s\n' "$AUR_SSH_KEY" >"$keyfile"
export GIT_SSH_COMMAND="ssh -i $keyfile $ssh_opts"
export GIT_TERMINAL_PROMPT=0
export GIT_AUTHOR_NAME=kawaro-release-bot GIT_COMMITTER_NAME=kawaro-release-bot
export GIT_AUTHOR_EMAIL=
export GIT_COMMITTER_EMAIL=

url="${AUR_REPO_URL:-ssh://aur@aur.archlinux.org/kawaro-bin.git}"
git clone --quiet "$url" "$work/aur" 2>&1 | grep -v '^warning: You appear to have cloned an empty repository' || true
[ -d "$work/aur/.git" ] || { echo "aur: clone failed" >&2; exit 1; }
git -C "$work/aur" checkout --quiet -B master
cp "$work/out/PKGBUILD" "$work/out/.SRCINFO" "$work/aur/"
git -C "$work/aur" add PKGBUILD .SRCINFO
if git -C "$work/aur" diff --cached --quiet; then
  echo "UNCHANGED aur: $version"
  exit 0
fi
git -C "$work/aur" commit --quiet -m "kawaro-bin $version-1"
git -C "$work/aur" push --quiet origin master
echo "PUBLISHED aur: $version"
