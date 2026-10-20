#!/usr/bin/env bash
# S-014d AC12 (first half): from the shortcut to the first frame drawn under
# 150 ms with a ready daemon, median of 20 drop-mode shows. Runs
# crates/ui/tests/measure.sh RUNS IDLE with DROP=1 in its private headless
# KWin (1920x1080, scale 1, the layer path), and checks:
#   - measure.sh ran the first show and RUNS warm shows;
#   - every show was a drop-mode show: hive_bench prints "mode=drop" in each
#     show line (the contract of "show --drop"), and the bench daemon (with
#     SBARHIVE_DEBUG_FOCUS=1) prints one "hive surface: layer" line per show
#     and no "hive focus: true" line, because the drop-mode layer takes no
#     keyboard (S-018 AC13) while a normal show takes it;
#   - the median warm first_frame_ms is under 150 ms.
# The time from the key press to the `sbarhive drop` process (the
# compositor's spawn) is not in this number; tests/drop/*.sh shortcut cover
# that the key runs it (AC11). No WAYLAND_DEBUG: it would slow the frames.
# measure.sh's output folder is copied to tests/drop/out/ and removed.
# Usage: tests/drop/budget.sh [runs] [idle_seconds]   (defaults 20 1)
# shellcheck disable=SC2016
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
RUNS="${1:-20}"
IDLE="${2:-1}"
OUT="${DROP_OUT:-$HERE/out}"
mkdir -p "$OUT"
FAILS=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }

{
  log="$OUT/budget-drop"
  echo "== measure.sh $RUNS $IDLE (DROP=1)"
  env -u SBARHIVE_SURFACE -u SHOW_TOKEN -u PREVIEW DROP=1 SBARHIVE_DEBUG_FOCUS=1 \
    CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
    nice -n 19 "$ROOT/crates/ui/tests/measure.sh" "$RUNS" "$IDLE" >"$log.txt" 2>&1
  raw=$(sed -n 's/^raw output: //p' "$log.txt")
  dir=$(dirname "${raw:-/nonexistent/x}")
  if [ -n "$raw" ] && [ -d "$dir" ] && [ "$dir" != / ]; then
    cp "$dir/daemon.log" "$log-daemon.log" 2>/dev/null
    cp "$dir/result.txt" "$log-result.txt" 2>/dev/null
    case "$dir" in /tmp/tmp.*) rm -rf "$dir" ;; esac
  fi
  grep -E '^(== summary|first show|warm first)' "$log.txt" | sed 's/^/INFO /'

  shows=$(grep -cE '^(first|warm) ' "$log-result.txt" 2>/dev/null)
  warm=$(grep -c '^warm ' "$log-result.txt" 2>/dev/null)
  dropl=$(grep -E '^(first|warm) ' "$log-result.txt" 2>/dev/null | grep -cw 'mode=drop')
  median=$(sed -n 's/^warm first_frame_ms median \([0-9.]*\) n .*/\1/p' "$log.txt" | head -1)
  layer=$(cat "$log-daemon.log" "$log-result.txt" 2>/dev/null | grep -cE '^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$')
  window=$(cat "$log-daemon.log" "$log-result.txt" 2>/dev/null | grep -c '^hive surface: window')
  focus=$(cat "$log-daemon.log" "$log-result.txt" 2>/dev/null | grep -c '^hive focus: true')
  echo "INFO drop: shows ${shows:-0} (warm ${warm:-0}), mode=drop lines ${dropl:-0}, hive surface layer ${layer:-0} window ${window:-0}, hive focus: true ${focus:-0}, median ${median:-none} ms"
  check "AC12: measure.sh ran all $((RUNS + 1)) daemon shows" '[ "${warm:-0}" -eq "$RUNS" ] && [ "${shows:-0}" -eq $((RUNS + 1)) ]'
  check "AC12: every show line says mode=drop (hive_bench show --drop)" '[ "${shows:-0}" -gt 0 ] && [ "${dropl:-0}" -eq "${shows:-x}" ]'
  check "AC12: every show was a layer surface (one \"hive surface: layer\" line per show, no window line)" \
    '[ "${layer:-0}" -eq "${shows:-x}" ] && [ "${window:-0}" -eq 0 ]'
  check "AC12: no show took the keyboard (no \"hive focus: true\": drop mode, S-018 AC13)" '[ "${shows:-0}" -gt 0 ] && [ "${focus:-0}" -eq 0 ]'
  check "AC12: median warm first frame of drop-mode shows under 150 ms" \
    '[ -n "$median" ] && python3 -I -c "import sys; sys.exit(not float(sys.argv[1]) < 150)" "$median"'
  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
} | tee "$OUT/budget-drop-summary.txt"
grep -q '^RESULT PASS' "$OUT/budget-drop-summary.txt"
