#!/usr/bin/env bash
# Packs the Windows release build into the portable zip. Builds nothing and
# calls no cargo: run scripts/windows/build.sh first, with the same
# CARGO_TARGET_DIR (default as in build.sh).
#   scripts/windows/make-zip.sh
# Writes $CARGO_TARGET_DIR/dist/sbarhive-<version>-windows-x64.zip (also the
# last line on stdout), <version> being the workspace version. One folder
# sbarhive-<version>-windows-x64/ holds sbarhive.exe, sbarhived.exe,
# sbarhive.ico (when design/brand/sbarhive.ico exists), README-windows.txt,
# actions/*.toml and SHA256SUMS. Reproducible: fixed mtimes, sorted entries,
# fixed modes, no extra fields, so the result does not depend on the umask,
# the clock or the folder it runs from. Exit 2 when an input is missing.
set -eu
# shellcheck source=/dev/null
. "$(dirname "$0")/scope.sh"
root="$(cd "$(dirname "$0")/../.." && pwd)"
target="${CARGO_TARGET_DIR:-$SBARHIVE_WIN_TARGET_DEFAULT}"
rel="$target/x86_64-pc-windows-gnu/release"
version="$(awk '/^\[workspace.package\]/{p=1;next} /^\[/{p=0} p && /^version *=/{gsub(/"/,"",$3); print $3; exit}' "$root/Cargo.toml")"
name="sbarhive-$version-windows-x64"
for f in "$rel/sbarhive.exe" "$rel/sbarhived.exe"; do
  [ -f "$f" ] || { echo "make-zip: missing $f, run scripts/windows/build.sh first" >&2; exit 2; }
done
[ -f "$root/installer/windows/README-windows.txt" ] || { echo "make-zip: missing installer/windows/README-windows.txt" >&2; exit 2; }

stage="$(mktemp -d "${TMPDIR:-/tmp}/sbarhive-zip.XXXXXX")"
trap 'rm -rf -- "$stage"' EXIT
dir="$stage/$name"
mkdir -p "$dir/actions" "$target/dist"
install -m 0644 "$rel/sbarhive.exe" "$rel/sbarhived.exe" "$root/installer/windows/README-windows.txt" "$dir/"
[ ! -f "$root/design/brand/sbarhive.ico" ] || install -m 0644 "$root/design/brand/sbarhive.ico" "$dir/"
install -m 0644 "$root"/actions/*.toml "$dir/actions/"
(cd "$dir" && LC_ALL=C find . -type f | sed 's|^\./||' | sort | xargs sha256sum > ../sums.tmp)
mv "$stage/sums.tmp" "$dir/SHA256SUMS"
chmod 0644 "$dir/SHA256SUMS"
find "$stage" -exec touch -h -d '2026-01-01 00:00:00 UTC' {} +

out="$target/dist/$name.zip"
rm -f "$out"
(cd "$stage" && LC_ALL=C find "$name" -type f | sort | TZ=UTC zip -X -D -q -9 "$out" -@)
echo "$out"
