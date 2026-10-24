#!/usr/bin/env bash
# Regenerates design/brand/sbarhive.ico (16, 24, 32, 48, 64 and 256 pixel
# images) from design/brand/sbarhive.svg. Needs inkscape and ImageMagick (`magick`,
# else `convert`). A re-run gives an identical file.
#   scripts/windows/make-ico.sh
set -eu
cd "$(dirname "$0")/../.."
svg=design/brand/sbarhive.svg
out=design/brand/sbarhive.ico
if ! command -v inkscape >/dev/null 2>&1; then
  echo "missing tool: inkscape" >&2
  exit 2
fi
if command -v magick >/dev/null 2>&1; then
  im=magick
elif command -v convert >/dev/null 2>&1; then
  im=convert
else
  echo "missing tool: magick" >&2
  exit 2
fi
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
sizes="16 24 32 48 64 256"
files=()
for s in $sizes; do
  inkscape "$svg" --export-type=png --export-width="$s" --export-height="$s" \
    --export-filename="$tmp/$s.png" >/dev/null 2>&1
  files+=("$tmp/$s.png")
done
"$im" "${files[@]}" -strip "$tmp/out.ico"
mv -f "$tmp/out.ico" "$out"
