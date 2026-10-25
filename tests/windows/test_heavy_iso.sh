#!/usr/bin/env bash
# S-044b, heavy test: make-iso.sh on the real cross build output in the
# default target dir. Run after tests/windows/test_heavy.sh (it needs a fresh
# release kawaro.exe and the Windows test programs), only when
# `pgrep -a cargo` is empty. Its one cargo call (T, incremental, so it only
# prints the current test programs) runs inside the 6G scope:
#   bash tests/windows/test_heavy_iso.sh
# The real cache holds more .exe files than test programs: the CLI binary that
# `cargo test` builds for the integration tests (a second kawaro-<hash>.exe,
# hardlinked to debug/kawaro.exe) and any stale hashes. /tests/ on the ISO
# must hold exactly the test programs T lists.
# The sh -c bodies below take their values as arguments, on purpose.
# shellcheck disable=SC2016
set -u
# shellcheck source=tests/windows/lib.sh
. "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT" || exit 2
SCOPE=(systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%)
REL="$WIN_TARGET_DEFAULT/x86_64-pc-windows-gnu/release/kawaro.exe"
ISO="$WIN_TARGET_DEFAULT/kawaro-win-tests.iso"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-winiso.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT

ac9_real_make_iso_carries_exactly_the_test_programs_t_lists () {
  echo "== AC9: make-iso.sh on the real build output"
  local rc want got list sums
  "${SCOPE[@]}" env CARGO_TARGET_DIR="$WIN_TARGET_DEFAULT" cargo test --no-run -j 2 \
    --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines -p kawaro-cli \
    > "$WORK/t.out" 2>&1; rc=$?
  assert_eq "AC9 setup: T exit code" "$rc" "0"
  grep -oE '\(([^()]*\.exe)\)' "$WORK/t.out" | tr -d '()' | xargs -r -n1 basename | sort -u > "$WORK/want"
  check "AC9 setup: T lists test programs" test -s "$WORK/want"
  check "AC9 setup: release kawaro.exe exists (run test_heavy.sh first)" test -f "$REL"
  if [ ! -x "$WIN/make-iso.sh" ]; then fail "AC9: missing or not executable $WIN/make-iso.sh"; return; fi
  (unset CARGO_TARGET_DIR CI; "$WIN/make-iso.sh") > "$WORK/iso.out" 2>&1; rc=$?
  assert_eq "AC9: exit code" "$rc" "0"
  if [ ! -f "$ISO" ]; then fail "AC9: no $ISO"; tail -n 20 "$WORK/iso.out" >&2; return; fi
  list="$(isoinfo -J -f -i "$ISO" 2>/dev/null)"
  grep -E '^/tests/.*\.exe$' <<<"$list" | sed 's#^/tests/##' | sort > "$WORK/got"
  want="$(wc -l < "$WORK/want")"; got="$(wc -l < "$WORK/got")"
  echo "  AC9: test programs from T $want, /tests/*.exe on the ISO $got"
  assert_eq "AC9: /tests/*.exe on the ISO are exactly the test programs T lists" \
    "$(diff "$WORK/want" "$WORK/got" | grep -E '^[<>]' | tr '\n' ' ')" ""
  check "AC9: lists /bin/kawaro.exe" grep -qx /bin/kawaro.exe <<<"$list"
  check "AC9: lists /run-tests.cmd" grep -qx /run-tests.cmd <<<"$list"
  assert_eq "AC9: /bin/kawaro.exe is the release build" \
    "$(isoinfo -J -x /bin/kawaro.exe -i "$ISO" 2>/dev/null | sha256sum | cut -d' ' -f1)" \
    "$(sha256sum "$REL" | cut -d' ' -f1)"
  sums="$(isoinfo -J -x /SHA256SUMS -i "$ISO" 2>/dev/null)"
  assert_eq "AC9: SHA256SUMS lines equal the executables on the ISO" \
    "$(grep -c . <<<"$sums")" "$(grep -ciE '\.exe$' <<<"$list")"
}

if pgrep -x cargo >/dev/null || pgrep -x cargo-mutants >/dev/null; then
  echo "another cargo run is active; run this when pgrep -a cargo is empty" >&2
  exit 2
fi
ac9_real_make_iso_carries_exactly_the_test_programs_t_lists
finish "S-044b windows ISO (heavy)"
