#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Signs the release SHA256SUMS with the apt archive key and uploads
# SHA256SUMS.asc and kawaro-archive.asc next to it (S-112, S-016 decision 8).
# Run only by the sign-checksums job of .github/workflows/packages.yml; no
# argument. Environment: GITHUB_REF_NAME, GITHUB_REPOSITORY, APT_SIGNING_KEY
# (armored private key), GH_TOKEN. Never run it by hand with a real key.
set -euo pipefail

if [ -z "${APT_SIGNING_KEY:-}" ]; then
  echo "SKIP sign-checksums: APT_SIGNING_KEY not configured"
  exit 0
fi

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PUB="$ROOT/packaging/keys/kawaro-archive.asc"
TAG="${GITHUB_REF_NAME:?GITHUB_REF_NAME is required}"
REPO="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"

GNUPGHOME="$(mktemp -d)"
export GNUPGHOME
chmod 700 "$GNUPGHOME"
WORK="$(mktemp -d)"
trap 'rm -rf "$GNUPGHOME" "$WORK"' EXIT

printf '%s\n' "$APT_SIGNING_KEY" | gpg --batch --quiet --import
want="$(gpg --batch --with-colons --show-keys "$PUB" | awk -F: '$1 == "fpr" {print $10; exit}')"
have="$(gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" {print $10; exit}')"
if [ -z "$want" ] || [ "$want" != "$have" ]; then
  echo "sign-checksums: the key fingerprint does not match packaging/keys/kawaro-archive.asc" >&2
  exit 1
fi

gh release download "$TAG" --pattern SHA256SUMS --dir "$WORK" -R "$REPO"
gpg --batch --yes --armor --pinentry-mode loopback --passphrase '' \
  --local-user "$want" --detach-sign --output "$WORK/SHA256SUMS.asc" "$WORK/SHA256SUMS"
gpg --batch --verify "$WORK/SHA256SUMS.asc" "$WORK/SHA256SUMS"
cp "$PUB" "$WORK/kawaro-archive.asc"
gh release upload "$TAG" "$WORK/SHA256SUMS.asc" "$WORK/kawaro-archive.asc" --clobber -R "$REPO"
echo "sign-checksums: uploaded SHA256SUMS.asc and kawaro-archive.asc for $TAG"
