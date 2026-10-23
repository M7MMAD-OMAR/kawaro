#!/usr/bin/env bash
# S-044a, heavy tests: the real Windows build (commands S and T of the story).
# Run only when `pgrep -a cargo` is empty, from outside any memory scope
# (a plain terminal or the Claude app scope), because build.sh and check.sh
# enter `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` by
# themselves; this script wraps its own cargo calls (T, metadata) in that
# scope too:
#   bash tests/windows/test_heavy.sh
# The sh -c bodies below take their values as arguments, on purpose.
# shellcheck disable=SC2016
set -u
# shellcheck source=tests/windows/lib.sh
. "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT" || exit 2
SCOPE=(systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%)
EXE="$WIN_TARGET_DEFAULT/x86_64-pc-windows-gnu/release/sbarhive.exe"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/sbarhive-winheavy.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT

ac1_real_build_sh_runs_in_a_6g_scope_and_produces_sbarhive_exe () {
  echo "== AC1: real build.sh (S)"
  if [ ! -x "$WIN/build.sh" ]; then fail "AC1: missing or not executable $WIN/build.sh"; return; fi
  # The scope's cgroup disappears when the build ends, so memory.max is read
  # as soon as the first line arrives, while the build still runs.
  mkfifo "$WORK/out"
  (unset CARGO_TARGET_DIR CI; "$WIN/build.sh" > "$WORK/out" 2> "$WORK/build.err"; echo $? > "$WORK/rc") &
  local first mm rc
  exec 3< "$WORK/out"
  IFS= read -r first <&3
  mm="$(cat "/sys/fs/cgroup${first#cgroup: }/memory.max" 2>/dev/null || echo gone)"
  cat <&3 > "$WORK/build.out"
  exec 3<&-
  wait
  rc="$(cat "$WORK/rc")"
  assert_eq "AC1: exit code" "$rc" "0"
  check "AC1: first line is 'cgroup: <path>' (got [$first])" grep -qE '^cgroup: /.+' <<<"$first"
  assert_eq "AC1: memory.max of the printed cgroup, read during the build" "$mm" "$SIX_G"
  check "AC1: $EXE exists" test -f "$EXE"
  check "AC1: sbarhive.exe is a PE32+ x86-64 Windows executable" \
    sh -c 'file -b "$1" | grep -q "PE32+ executable.*x86-64"' _ "$EXE"
  if [ "$rc" != 0 ]; then tail -n 30 "$WORK/build.err" >&2; fi
}

ac2_check_dlls_passes_on_the_real_sbarhive_exe () {
  echo "== AC2: check-dlls.sh on the real sbarhive.exe"
  if [ ! -x "$WIN/check-dlls.sh" ]; then fail "AC2: missing or not executable $WIN/check-dlls.sh"; return; fi
  local out rc
  out="$("$WIN/check-dlls.sh" "$EXE" 2>&1)"; rc=$?
  assert_eq "AC2 real: exit code (output [$out])" "$rc" "0"
  check "AC2 real: line 'dll check: ok <name> <N> imports'" \
    grep -qE '^dll check: ok [^ ]+ [0-9]+ imports$' <<<"$out"
}

ac3_real_check_sh_exits_0 () {
  echo "== AC3: real check.sh"
  if [ ! -x "$WIN/check.sh" ]; then fail "AC3: missing or not executable $WIN/check.sh"; return; fi
  local rc
  (unset CI; "$WIN/check.sh") > "$WORK/check.out" 2>&1; rc=$?
  assert_eq "AC3: exit code" "$rc" "0"
  if [ "$rc" != 0 ]; then tail -n 40 "$WORK/check.out" >&2; fi
}

ac5_windows_test_binaries_build_one_exe_per_test_target () {
  echo "== AC5: cargo test --no-run for Windows (T)"
  local rc want got
  "${SCOPE[@]}" env CARGO_TARGET_DIR="$WIN_TARGET_DEFAULT" cargo test --no-run -j 2 \
    --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines -p sbarhive-cli \
    > "$WORK/t.out" 2>&1; rc=$?
  assert_eq "AC5: exit code" "$rc" "0"
  "${SCOPE[@]}" cargo metadata --no-deps --format-version 1 > "$WORK/meta.json" 2>/dev/null
  want="$(python3 -I -c '
import json, sys
m = json.load(open(sys.argv[1]))
pk = {"sbarhive-core", "sbarhive-engines", "sbarhive-cli"}
print(sum(1 for p in m["packages"] if p["name"] in pk
          for t in p["targets"]
          if t.get("test") and set(t["kind"]) & {"lib", "bin", "test"}))
' "$WORK/meta.json")"
  got="$(grep -oE '\(([^()]*\.exe)\)' "$WORK/t.out" | sort -u | wc -l)"
  echo "  AC5: test targets $want, listed .exe files $got"
  check "AC5: some test target exists" test "${want:-0}" -gt 0
  assert_eq "AC5: listed .exe files equal test targets" "$got" "$want"
  if [ "$rc" != 0 ]; then grep -E '^error' "$WORK/t.out" | sort | uniq -c | head -n 20 >&2; fi
}

if [ "$(cat "/sys/fs/cgroup$(own_cgroup)/memory.max" 2>/dev/null)" != max ]; then
  echo "run outside a memory scope: build.sh and check.sh enter their own" >&2
  exit 2
fi
if pgrep -x cargo >/dev/null || pgrep -x cargo-mutants >/dev/null; then
  echo "another cargo run is active; run this when pgrep -a cargo is empty" >&2
  exit 2
fi
ac1_real_build_sh_runs_in_a_6g_scope_and_produces_sbarhive_exe
ac2_check_dlls_passes_on_the_real_sbarhive_exe
ac3_real_check_sh_exits_0
ac5_windows_test_binaries_build_one_exe_per_test_target
finish "S-044a windows build (heavy)"
