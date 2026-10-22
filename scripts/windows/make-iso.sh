#!/usr/bin/env bash
# Packs the Windows cross build into an ISO for the test machine. Builds
# nothing: run scripts/windows/build.sh and
# `cargo test --no-run --target x86_64-pc-windows-gnu -p sbarhive-core
# -p sbarhive-engines -p sbarhive-cli` first, with the same CARGO_TARGET_DIR.
#   scripts/windows/make-iso.sh
# CARGO_TARGET_DIR defaults to /home/sbarah/.cache/sbarhive-win-target. The
# ISO is $CARGO_TARGET_DIR/sbarhive-win-tests.iso (genisoimage -J -R) with
#   bin/sbarhive.exe  the release build
#   tests/*.exe       the test programs from debug/deps
#   run-tests.cmd     runs every tests\*.exe on Windows
#   SHA256SUMS        one line per .exe
# A deps/*.exe that is the same file as an uplifted debug/*.exe is a binary
# cargo built for the integration tests, not a test program, and is left out.
# Stale hashes from an older build are not told apart: clean deps/ if any.
set -eu
target="${CARGO_TARGET_DIR:-/home/sbarah/.cache/sbarhive-win-target}"
win="$(cd "$(dirname "$0")" && pwd)"
rel="$target/x86_64-pc-windows-gnu/release/sbarhive.exe"
debug="$target/x86_64-pc-windows-gnu/debug"
iso="$target/sbarhive-win-tests.iso"
[ -f "$rel" ] || { echo "make-iso: no $rel, run build.sh first" >&2; exit 2; }

tests=()
for f in "$debug"/deps/*.exe; do
  [ -f "$f" ] || continue
  for u in "$debug"/*.exe; do
    [ -f "$u" ] && [ "$f" -ef "$u" ] && continue 2
  done
  tests+=("$f")
done
[ ${#tests[@]} -gt 0 ] || { echo "make-iso: no test programs in $debug/deps" >&2; exit 2; }

sums="$(mktemp)"
trap 'rm -f -- "$sums"' EXIT
grafts=("bin/sbarhive.exe=$rel")
(cd "$(dirname "$rel")" && sha256sum sbarhive.exe | sed 's#  #  bin/#') > "$sums"
for f in "${tests[@]}"; do
  n="$(basename "$f")"
  grafts+=("tests/$n=$f")
  (cd "$debug/deps" && sha256sum "$n" | sed 's#  #  tests/#') >> "$sums"
done
grafts+=("run-tests.cmd=$win/run-tests.cmd" "SHA256SUMS=$sums")

rm -f -- "$iso.tmp"
genisoimage -quiet -J -R -V SBARHIVE_TESTS -graft-points -o "$iso.tmp" "${grafts[@]}"
mv -f -- "$iso.tmp" "$iso"
echo "iso: $iso (${#tests[@]} test programs)"
