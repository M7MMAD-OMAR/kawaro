#!/usr/bin/env bash
# Builds the per-user MSI from the Windows release build with wixl (msitools,
# a build tool only). Calls no cargo: run scripts/windows/build.sh first, with
# the same CARGO_TARGET_DIR (default
# as in build.sh).
#   scripts/windows/make-msi.sh
# Writes $CARGO_TARGET_DIR/dist/kawaro-<version>-windows-x64.msi (also the
# last line on stdout). The ProductVersion is the workspace version without a
# prerelease part (MSI versions are numeric). Exit 2 when an input is missing.
set -eu
# shellcheck source=/dev/null
. "$(dirname "$0")/scope.sh"
root="$(cd "$(dirname "$0")/../.." && pwd)"
target="${CARGO_TARGET_DIR:-$KAWARO_WIN_TARGET_DEFAULT}"
rel="$target/x86_64-pc-windows-gnu/release"
version="$(awk '/^\[workspace.package\]/{p=1;next} /^\[/{p=0} p && /^version *=/{gsub(/"/,"",$3); print $3; exit}' "$root/Cargo.toml")"
for f in "$rel/kawaro.exe" "$rel/kawarod.exe"; do
  [ -f "$f" ] || { echo "make-msi: missing $f, run scripts/windows/build.sh first" >&2; exit 2; }
done
command -v wixl >/dev/null || { echo "make-msi: wixl not found, install msitools" >&2; exit 2; }
mkdir -p "$target/dist"
out="$target/dist/kawaro-$version-windows-x64.msi"
rm -f "$out"
defs=(-D "Version=${version%%[-+]*}" -D "Release=$rel" -D "Actions=$root/actions")
# wixl has no <?ifdef?>, so the optional icon block is kept or cut here.
wxs="$(mktemp "${TMPDIR:-/tmp}/kawaro-msi.XXXXXX")"
trap 'rm -f -- "$wxs"' EXIT
if [ -f "$root/design/brand/kawaro.ico" ]; then
  defs+=(-D "Ico=$root/design/brand/kawaro.ico")
  grep -v '^<?\(ifdef Ico\|endif\)?>$' "$root/installer/windows/kawaro.wxs" > "$wxs"
else
  sed '/^<?ifdef Ico?>$/,/^<?endif?>$/d' "$root/installer/windows/kawaro.wxs" > "$wxs"
fi
wixl -v -a x64 "${defs[@]}" -o "$out" "$wxs"
echo "$out"
