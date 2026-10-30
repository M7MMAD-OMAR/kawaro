#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Writes PKGBUILD and .SRCINFO for kawaro-bin into OUTDIR.
#   packaging/aur/generate.sh vX.Y.Z SHA256SUMS OWNER/REPO OUTDIR
# sha256sums comes from the DEB line of the release SHA256SUMS. Exit 1 when
# that line is missing, 2 on bad usage. makepkg --printsrcinfo is not needed:
# both files come from the same two templates and the same three values.
set -euo pipefail
[ "$#" -eq 4 ] || { echo "usage: generate.sh vX.Y.Z SHA256SUMS OWNER/REPO OUTDIR" >&2; exit 2; }
tag="$1" sums="$2" repo="$3" out="$4"
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "generate: bad tag $tag" >&2; exit 2; }
version="${tag#v}"
deb="kawaro_${version}_amd64.deb"
sha="$(awk -v f="$deb" '{n=$2; sub(/^\*/, "", n)} n == f {print $1}' "$sums")"
[[ "$sha" =~ ^[0-9a-f]{64}$ ]] || { echo "generate: no valid $deb line in $sums" >&2; exit 1; }
here="$(dirname "$0")"
mkdir -p "$out"
for pair in PKGBUILD.tpl:PKGBUILD SRCINFO.tpl:.SRCINFO; do
  sed -e "s|@VERSION@|$version|g" -e "s|@REPO@|$repo|g" -e "s|@SHA@|$sha|g" \
    "$here/${pair%%:*}" >"$out/${pair##*:}"
done
