#!/usr/bin/env bash
# S-018 AC18 (S-018f): the S-001 budgets on the layer path. Runs
# crates/ui/tests/measure.sh 20 60 twice in its private headless KWin
# (1920x1080, scale 1): once without a token and once with SHOW_TOKEN, and
# checks for each run:
#   - the hive was a layer surface in every show: the bench daemon runs with
#     SBARHIVE_DEBUG_FOCUS=1 and prints "hive surface: layer|window X Y W H"
#     per show (AC11); one layer line per show and no window line.
#     SBARHIVE_SURFACE is unset, so the default choice (AC1) is measured.
#     No WAYLAND_DEBUG: it would slow the frames being timed.
#   - the median warm first_frame_ms is under 150 ms;
#   - the daemon's RSS after 60 s idle is 40 MB or less, rounded down to whole
#     MB the way tests/e2e/run.sh rounds (int(VmRSS kB / 1024) <= 40).
# The first show after the daemon starts and the cold starts are printed as
# INFO only. measure.sh's output folder is copied to tests/overlay/out/ and
# removed from /tmp.
# Usage: tests/overlay/budget.sh [runs] [idle_seconds]   (defaults 20 60)
# shellcheck disable=SC2016
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
RUNS="${1:-20}"
IDLE="${2:-60}"
OUT="${OVERLAY_OUT:-$HERE/out}"
mkdir -p "$OUT"
FAILS=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }

run() { # run NAME [TOKEN]
  local name=$1 token=${2:-} log="$OUT/budget-$1"
  echo "== measure.sh $RUNS $IDLE ($name)"
  local env=(env -u SBARHIVE_SURFACE -u SHOW_TOKEN SBARHIVE_DEBUG_FOCUS=1 CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}")
  [ -n "$token" ] && env+=(SHOW_TOKEN="$token")
  "${env[@]}" "$ROOT/crates/ui/tests/measure.sh" "$RUNS" "$IDLE" >"$log.txt" 2>&1
  local raw; raw=$(sed -n 's/^raw output: //p' "$log.txt")
  local dir; dir=$(dirname "${raw:-/nonexistent/x}")
  if [ -n "$raw" ] && [ -d "$dir" ] && [ "$dir" != / ]; then
    cp "$dir/daemon.log" "$log-daemon.log" 2>/dev/null
    cp "$dir/result.txt" "$log-result.txt" 2>/dev/null
    case "$dir" in /tmp/tmp.*) rm -rf "$dir" ;; esac
  fi
  grep -E '^(== summary|cold exec|first show|warm first|rss_kb)' "$log.txt" | sed 's/^/INFO /'

  local shows warm median rss layer window
  # One "first" show plus RUNS warm shows.
  shows=$(grep -cE '^(first|warm) ' "$log-result.txt" 2>/dev/null)
  warm=$(grep -c '^warm ' "$log-result.txt" 2>/dev/null)
  median=$(sed -n 's/^warm first_frame_ms median \([0-9.]*\) n .*/\1/p' "$log.txt" | head -1)
  # measure.sh prints the RSS line twice (the run, then the summary).
  rss=$(sed -n 's/^rss_kb_after_idle=\([0-9]*\) .*/\1/p' "$log.txt" | head -1)
  # The bench daemon's stderr is daemon.log and its stdout is in result.txt.
  layer=$(cat "$log-daemon.log" "$log-result.txt" 2>/dev/null | grep -cE '^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$')
  window=$(cat "$log-daemon.log" "$log-result.txt" 2>/dev/null | grep -c '^hive surface: window')
  echo "INFO $name: shows ${shows:-0} (warm ${warm:-0}), hive surface lines layer ${layer:-0} window ${window:-0}, median ${median:-none} ms, rss ${rss:-none} kB"
  check "AC18 $name: measure.sh ran all $((RUNS + 1)) daemon shows" '[ "${warm:-0}" -eq "$RUNS" ] && [ "${shows:-0}" -eq $((RUNS + 1)) ]'
  check "AC18 $name: every show was a layer surface (one \"hive surface: layer\" line per show, no window line)" \
    '[ "${layer:-0}" -eq "${shows:-x}" ] && [ "${window:-0}" -eq 0 ]'
  check "AC18 $name: median warm first frame under 150 ms" \
    '[ -n "$median" ] && python3 -I -c "import sys; sys.exit(not float(sys.argv[1]) < 150)" "$median"'
  check "AC18 $name: RSS after $IDLE s idle 40 MB or less" '[ -n "$rss" ] && [ $((rss / 1024)) -le 40 ]'
}

: >"$OUT/budget.txt"
{
  run plain
  run token sbh-budget-token
  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
} | tee "$OUT/budget.txt"
grep -q '^RESULT PASS' "$OUT/budget.txt"
