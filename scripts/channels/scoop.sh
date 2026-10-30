#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Channel: Scoop bucket (S-103c). Run from the repository root.
#   scripts/channels/scoop.sh vX.Y.Z [--dry-run]
# Environment (names only, values are never printed):
#   SCOOP_DEPLOY_KEY    private ssh key, a deploy key with write access to the
#                       bucket repository only (secret, written to a 0600 file
#                       that an EXIT trap removes)
#   BUCKET_REPO         OWNER/NAME of the bucket repository
#   CANONICAL_REPO      OWNER/NAME of this project (release URLs)
#   SHA256SUMS_FILE     optional local SHA256SUMS (default dist/SHA256SUMS,
#                       else downloaded from the release)
#   SCOOP_BUCKET_URL    optional clone URL override (tests use a local path)
#   GITHUB_KNOWN_HOSTS  optional known_hosts path (default
#                       packaging/release/github_known_hosts, GitHub's published
#                       host keys; checked strictly)
# Exit 0 done, 10 clean SKIP (not configured, or prerelease), other = failure.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
dry=0 tag=""
for a in "$@"; do
  case "$a" in
    --dry-run) dry=1 ;;
    v*) tag="$a" ;;
    *) echo "usage: scoop.sh vX.Y.Z [--dry-run]" >&2; exit 2 ;;
  esac
done
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || { echo "scoop: bad tag '$tag'" >&2; exit 2; }
version="${tag#v}"
[[ "$tag" != *-* ]] || { echo "SKIP scoop: prerelease tag"; exit 10; }

missing=""
known_hosts="${GITHUB_KNOWN_HOSTS:-$ROOT/packaging/release/github_known_hosts}"
for v in SCOOP_DEPLOY_KEY BUCKET_REPO CANONICAL_REPO; do
  [ -n "${!v:-}" ] || { missing="$v"; break; }
done
if [ -z "$missing" ] && [ ! -s "$known_hosts" ]; then missing="packaging/release/github_known_hosts"; fi
if [ -n "$missing" ]; then
  [ "$dry" = 0 ] || { echo "NOTE scoop: $missing not configured, a real run would SKIP"; }
  if [ "$dry" = 0 ]; then echo "SKIP scoop: $missing not configured"; exit 10; fi
fi
repo="${CANONICAL_REPO:-OWNER/kawaro}"

work="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-scoop.XXXXXX")"
trap 'rm -rf -- "$work"' EXIT

sums="${SHA256SUMS_FILE:-}"
if [ -z "$sums" ] && [ -f "$ROOT/dist/SHA256SUMS" ]; then sums="$ROOT/dist/SHA256SUMS"; fi
if [ -z "$sums" ]; then
  sums="$work/SHA256SUMS"
  curl -fsSL --retry 3 -o "$sums" "https://github.com/$repo/releases/download/$tag/SHA256SUMS"
fi
"$ROOT/packaging/scoop/generate.sh" "$tag" "$sums" "$repo" >"$work/kawaro.json"
"$ROOT/packaging/scoop/validate.sh" "$work/kawaro.json" "$sums"

ssh_opts="-o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=${known_hosts/#$ROOT\//}"
if [ "$dry" = 1 ]; then
  echo "DRYRUN scoop: would publish $version"
  echo "DRYRUN scoop: would write bucket/kawaro.json to ${BUCKET_REPO:-BUCKET_REPO} by ssh push, ssh options: $ssh_opts"
  exit 0
fi

ssh_user=git # the fixed GitHub ssh account name, not a person
url="${SCOOP_BUCKET_URL:-ssh://${ssh_user}@github.com/$BUCKET_REPO.git}"
# The deploy key is written to a private file in the work dir, removed on exit.
umask 077
keyfile="$work/id_key"
printf '%s\n' "$SCOOP_DEPLOY_KEY" >"$keyfile"
export GIT_SSH_COMMAND="ssh -i $keyfile -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$known_hosts"
export GIT_TERMINAL_PROMPT=0
export GIT_AUTHOR_NAME=kawaro-release-bot GIT_COMMITTER_NAME=kawaro-release-bot
export GIT_AUTHOR_EMAIL=
export GIT_COMMITTER_EMAIL=

git clone --quiet "$url" "$work/bucket" 2>&1 | grep -v '^warning: You appear to have cloned an empty repository' || true
[ -d "$work/bucket/.git" ] || { echo "scoop: clone of the bucket repository failed" >&2; exit 1; }
mkdir -p "$work/bucket/bucket"
cp "$work/kawaro.json" "$work/bucket/bucket/kawaro.json"
git -C "$work/bucket" add bucket/kawaro.json
if git -C "$work/bucket" diff --cached --quiet; then
  echo "UNCHANGED scoop: $version"
  exit 0
fi
git -C "$work/bucket" commit --quiet -m "kawaro: update to $version"
git -C "$work/bucket" push --quiet origin HEAD
echo "PUBLISHED scoop: $version"
