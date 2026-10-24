#!/usr/bin/env bash
# Regenerates design/brand/sbarhive.ico (16, 24, 32, 48, 64 and 256 pixel
# images) from design/brand/sbarhive.svg. Needs inkscape and ImageMagick. A
# re-run gives an identical file.
#   scripts/windows/make-ico.sh
set -eu
cd "$(dirname "$0")/../.."
svg=design/brand/sbarhive.svg
out=design/brand/sbarhive.ico
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
sizes="16 24 32 48 64 256"
files=()
for s in $sizes; do
  inkscape "$svg" --export-type=png --export-width="$s" --export-height="$s" \
    --export-filename="$tmp/$s.png" >/dev/null 2>&1
  files+=("$tmp/$s.png")
done
magick "${files[@]}" -strip "$tmp/out.ico"
mv -f "$tmp/out.ico" "$out"
