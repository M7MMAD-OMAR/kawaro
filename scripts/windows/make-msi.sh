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
ico="$root/design/brand/kawaro.ico"
# The icon is required (S-085 D6): the Start shortcut and the menu use it.
[ -f "$ico" ] || { echo "make-msi: missing $ico, the kawaro.ico icon is required" >&2; exit 2; }
command -v wixl >/dev/null || { echo "make-msi: wixl not found, install msitools" >&2; exit 2; }
mkdir -p "$target/dist"
out="$target/dist/kawaro-$version-windows-x64.msi"
rm -f "$out"
tail="$("$root/scripts/windows/guid-tail.sh" "$version" "$rel/kawaro.exe" "$rel/kawarod.exe" "$ico" "$root"/actions/*.toml)"
defs=(-D "Version=${version%%[-+]*}" -D "GuidTail=$tail" -D "Release=$rel" -D "Actions=$root/actions" -D "Ico=$ico")
wixl -v -a x64 "${defs[@]}" -o "$out" "$root/installer/windows/kawaro.wxs"
"$root/scripts/windows/msi-fixup.sh" "$out" "$version"
echo "$out"
