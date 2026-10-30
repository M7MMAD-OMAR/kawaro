#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Prints the Scoop manifest for a release on stdout.
#   packaging/scoop/generate.sh vX.Y.Z SHA256SUMS OWNER/REPO
# The hash comes from the ZIP line of the release SHA256SUMS. Exit 1 when the
# ZIP line is missing, 2 on bad usage.
set -euo pipefail
[ "$#" -eq 3 ] || { echo "usage: generate.sh vX.Y.Z SHA256SUMS OWNER/REPO" >&2; exit 2; }
tag="$1" sums="$2" repo="$3"
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "generate: bad tag $tag" >&2; exit 2; }
version="${tag#v}"
zip="kawaro-$version-windows-x64.zip"
hash="$(awk -v f="$zip" '{n=$2; sub(/^\*/, "", n)} n == f {print $1}' "$sums")"
[[ "$hash" =~ ^[0-9a-f]{64}$ ]] || { echo "generate: no valid $zip line in $sums" >&2; exit 1; }
sed -e "s|@VERSION@|$version|g" -e "s|@REPO@|$repo|g" -e "s|@HASH@|$hash|g" \
  "$(dirname "$0")/kawaro.json.tpl"
