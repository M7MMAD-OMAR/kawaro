#!/usr/bin/env bash
# S-018 AC27 (S-018h): the first frame budget with a preview. Runs
# PREVIEW=1 crates/ui/tests/measure.sh 20 IDLE in its private headless KWin
# (1920x1080, scale 1): every show presents one selected file whose only
# thumbnail is a 1024 px "xx-large" one, made in measure.sh's sandbox cache.
# Checks:
#   - all 21 daemon shows ran (one "first" and 20 warm);
#   - every show drew the thumbnail, downscaled before Slint: the bench daemon
#     runs with KAWARO_DEBUG_FOCUS=1 and the hive prints one line per show
#     with files, "hive preview: thumb W H" (the pixel size handed to Slint)
#     or "hive preview: icon NAME"; here 21 thumb lines, no icon line, and
#     W and H between 1 and 128 (the square inscribed in the centre hexagon is
#     about 53 px at scale 1; the exact bound is in ui_tests);
#   - the median warm first_frame_ms is under 150 ms.
# The memory half of AC27 (RSS after 20 shows) is in tests/e2e/run.sh; the
# RSS here is INFO only. No WAYLAND_DEBUG: it would slow the timed frames.
# Usage: tests/overlay/preview.sh [runs] [idle_seconds]   (defaults 20 5)
# shellcheck disable=SC2016
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
RUNS="${1:-20}"
IDLE="${2:-5}"
OUT="${OVERLAY_OUT:-$HERE/out}"
mkdir -p "$OUT"
FAILS=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }

log="$OUT/preview"
{
  echo "== PREVIEW=1 measure.sh $RUNS $IDLE"
  env -u KAWARO_SURFACE -u SHOW_TOKEN PREVIEW=1 KAWARO_DEBUG_FOCUS=1 \
    CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
    "$ROOT/crates/ui/tests/measure.sh" "$RUNS" "$IDLE" >"$log.txt" 2>&1
  raw=$(sed -n 's/^raw output: //p' "$log.txt")
  dir=$(dirname "${raw:-/nonexistent/x}")
  if [ -n "$raw" ] && [ -d "$dir" ] && [ "$dir" != / ]; then
    cp "$dir/daemon.log" "$log-daemon.log" 2>/dev/null
    cp "$dir/result.txt" "$log-result.txt" 2>/dev/null
    case "$dir" in /tmp/tmp.*) rm -rf "$dir" ;; esac
  fi
  grep -E '^(preview thumbnail|cold exec|first show|warm first|rss_kb)' "$log.txt" | sed 's/^/INFO /'

  shows=$(grep -cE '^(first|warm) ' "$log-result.txt" 2>/dev/null)
  warm=$(grep -c '^warm ' "$log-result.txt" 2>/dev/null)
  median=$(sed -n 's/^warm first_frame_ms median \([0-9.]*\) n .*/\1/p' "$log.txt" | head -1)
  thumbs=$(cat "$log-daemon.log" 2>/dev/null | grep -cE '^hive preview: thumb [0-9]+ [0-9]+$')
  icons=$(cat "$log-daemon.log" 2>/dev/null | grep -c '^hive preview: icon')
  big=$(cat "$log-daemon.log" 2>/dev/null | awk '/^hive preview: thumb [0-9]+ [0-9]+$/ { if ($4 < 1 || $5 < 1 || $4 > 128 || $5 > 128) n++ } END { print n + 0 }')
  sizes=$(cat "$log-daemon.log" 2>/dev/null | sed -n 's/^hive preview: thumb //p' | sort | uniq -c | tr '\n' ';')
  echo "INFO shows ${shows:-0} (warm ${warm:-0}), preview lines thumb ${thumbs:-0} icon ${icons:-0} (sizes: ${sizes:-none}), median ${median:-none} ms"
  check "AC27: measure.sh ran all $((RUNS + 1)) daemon shows" '[ "${warm:-0}" -eq "$RUNS" ] && [ "${shows:-0}" -eq $((RUNS + 1)) ]'
  check "AC27: every show drew the xx-large thumbnail (one \"hive preview: thumb W H\" line per show, no icon line)" \
    '[ "${thumbs:-0}" -eq "${shows:-x}" ] && [ "${icons:-0}" -eq 0 ]'
  check "AC24/AC27: the 1024 px thumbnail reached Slint downscaled (W and H 1 to 128)" \
    '[ "${thumbs:-0}" -gt 0 ] && [ "${big:-1}" -eq 0 ]'
  check "AC27: median warm first frame under 150 ms with the preview" \
    '[ -n "$median" ] && python3 -I -c "import sys; sys.exit(not float(sys.argv[1]) < 150)" "$median"'
  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
} | tee "$OUT/preview-result.txt"
grep -q '^RESULT PASS' "$OUT/preview-result.txt"
