#!/usr/bin/env bash
# S-014d AC12 (first half): from the shortcut to the show being ready under
# 150 ms with a ready daemon, median of 20 drop-mode shows (the first frame
# until S-021, the catcher's ready commit since S-021 AC8, below). Runs
# crates/ui/tests/measure.sh RUNS IDLE with DROP=1 in its private headless
# KWin (1920x1080, scale 1, the layer path), and checks:
#   - measure.sh ran the first show and RUNS warm shows;
#   - every show was a drop-mode show: hive_bench prints "mode=drop" in each
#     show line (the contract of "show --drop"), and the bench daemon (with
#     SBARHIVE_DEBUG_FOCUS=1) prints one "hive surface: layer" line per show
#     and no "hive focus: true" line, because the drop-mode layer takes no
#     keyboard (S-018 AC13) while a normal show takes it;
#   - the median warm catcher_ms is under 150 ms (first_frame_ms before
#     S-021 AC8).
# The time from the key press to the `sbarhive drop` process (the
# compositor's spawn) is not in this number; tests/drop/*.sh shortcut cover
# that the key runs it (AC11). No WAYLAND_DEBUG: it would slow the frames.
# measure.sh's output folder is copied to tests/drop/out/ and removed.
# S-019e AC20, mode "hold": the 150 ms budget of the long press path is
# timed from the end of the threshold, not from the press. measure.sh runs
# with DROP=1 HOLD=1 (hive_bench "show --drop --hold", the gate in
# sbarhive_core::hold with its default 400 ms), on the layer path, and:
#   - all RUNS + 1 shows ran, every line says mode=hold and hold_ms=400;
#   - in every line press_to_catcher_ms is catcher_ms plus the threshold:
#     B - A between 400 and 450 ms (the clock starts at the threshold's end,
#     neither at the press nor late; an instant show cannot pass);
#   - one "hive surface: layer" line per show, no "hive focus: true";
#   - the median warm catcher_ms is under 150 ms; the median
#     press_to_catcher_ms is printed as INFO (the threshold plus that
#     median). Before S-021 AC8 these were first_frame_ms and
#     press_to_frame_ms.
# S-021 AC8 (backlog/S-021-drop-only-on-drag.md, the maintainer's answer to
# question 1): a drop show on the layer path binds a hidden catcher and
# draws the hive only once a drag of files with an action enters it, so a
# bench show (no drag) has no first frame any more. Its reference is the
# catcher's ready commit: its first wl_surface.commit after the first
# configure of its zwlr_layer_surface_v1 (the commit before that configure
# carries no buffer; this one carries the catcher's transparent pixel), as
# catcher_ready in tests/drop/layer.sh. hive_bench prints, instead of
# first_frame_ms:
#   show --drop          "mode=drop catcher_ms=A", A from the request line to
#                        the catcher's ready commit;
#   show --drop --hold   "mode=hold hold_ms=400 catcher_ms=A
#                        press_to_catcher_ms=B", A from the threshold's end
#                        (the gate's show) to the catcher's ready commit, B
#                        from the request (B is the threshold plus A).
# Both modes then check, with the guards above: one "hive surface: layer"
# line per show, no "hive focus: true" line, no "hive reveal:" line (no drag
# enters, so the hive is never drawn), every line carries catcher_ms (and
# press_to_catcher_ms in hold mode, B - A from 400 to 450 ms), and the
# median warm catcher_ms is under 150 ms. measure.sh prints those medians.
# Usage: tests/drop/budget.sh [runs] [idle_seconds] [hold]   (defaults 20 1)
# shellcheck disable=SC2016
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
RUNS="${1:-20}"
IDLE="${2:-1}"
HOLDM="${3:-}"
OUT="${DROP_OUT:-$HERE/out}"
mkdir -p "$OUT"
FAILS=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }

if [ "$HOLDM" = hold ]; then
{
  log="$OUT/budget-hold"
  echo "== measure.sh $RUNS $IDLE (DROP=1 HOLD=1)"
  env -u SBARHIVE_SURFACE -u SHOW_TOKEN -u PREVIEW DROP=1 HOLD=1 SBARHIVE_DEBUG_FOCUS=1 \
    CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
    nice -n 19 "$ROOT/crates/ui/tests/measure.sh" "$RUNS" "$IDLE" >"$log.txt" 2>&1
  raw=$(sed -n 's/^raw output: //p' "$log.txt")
  dir=$(dirname "${raw:-/nonexistent/x}")
  if [ -n "$raw" ] && [ -d "$dir" ] && [ "$dir" != / ]; then
    cp "$dir/daemon.log" "$log-daemon.log" 2>/dev/null
    cp "$dir/result.txt" "$log-result.txt" 2>/dev/null
    case "$dir" in /tmp/tmp.*) rm -rf "$dir" ;; esac
  fi
  grep -E '^(== summary|first show|warm catcher|warm press)' "$log.txt" | sed 's/^/INFO /'
  lines=$(grep -E '^(first|warm) ' "$log-result.txt" 2>/dev/null)
  shows=$(grep -c . <<<"$lines")
  warm=$(grep -c '^warm ' <<<"$lines")
  holdl=$(grep -w 'mode=hold' <<<"$lines" | grep -w 'hold_ms=400' | grep -E '\bcatcher_ms=[0-9.]+' | grep -cE '\bpress_to_catcher_ms=[0-9.]+')
  # Lines where B - A is the threshold (400 to 450 ms).
  span=$(python3 -I -c '
import re, sys
ok = 0
for line in sys.stdin:
    a = re.search(r"\bcatcher_ms=([0-9.]+)", line)
    b = re.search(r"\bpress_to_catcher_ms=([0-9.]+)", line)
    if a and b and 400 <= float(b.group(1)) - float(a.group(1)) <= 450:
        ok += 1
print(ok)' <<<"$lines")
  median=$(sed -n 's/^warm catcher_ms median \([0-9.]*\) n .*/\1/p' "$log.txt" | head -1)
  total=$(sed -n 's/^warm press_to_catcher_ms median \([0-9.]*\) n .*/\1/p' "$log.txt" | head -1)
  both=$(cat "$log-daemon.log" "$log-result.txt" 2>/dev/null)
  layer=$(grep -cE '^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$' <<<"$both")
  window=$(grep -c '^hive surface: window' <<<"$both")
  focus=$(grep -c '^hive focus: true' <<<"$both")
  reveal=$(grep -c '^hive reveal:' <<<"$both")
  echo "INFO hold: shows ${shows:-0} (warm ${warm:-0}), mode=hold hold_ms=400 lines with catcher_ms and press_to_catcher_ms ${holdl:-0}, threshold span lines ${span:-0}, hive surface layer ${layer:-0} window ${window:-0}, hive focus: true ${focus:-0}, hive reveal ${reveal:-0}, median from the threshold ${median:-none} ms, median from the press ${total:-none} ms"
  check "S-019 AC20: measure.sh ran all $((RUNS + 1)) hold shows" '[ "${warm:-0}" -eq "$RUNS" ] && [ "${shows:-0}" -eq $((RUNS + 1)) ]'
  check "S-019 AC20, S-021 AC8: every show line says mode=hold and hold_ms=400 with catcher_ms and press_to_catcher_ms (hive_bench show --drop --hold)" '[ "${shows:-0}" -gt 0 ] && [ "${holdl:-0}" -eq "${shows:-x}" ]'
  check "S-019 AC20, S-021 AC8: in every show the time from the press is the threshold plus the time from its end (press_to_catcher_ms - catcher_ms 400 to 450)" \
    '[ "${shows:-0}" -gt 0 ] && [ "${span:-0}" -eq "${shows:-x}" ]'
  check "S-019 AC20: every show was a layer surface (one \"hive surface: layer\" line per show, no window line)" \
    '[ "${shows:-0}" -gt 0 ] && [ "${layer:-0}" -eq "${shows:-x}" ] && [ "${window:-0}" -eq 0 ]'
  check "S-019 AC20: no show took the keyboard (drop mode)" '[ "${shows:-0}" -gt 0 ] && [ "${focus:-0}" -eq 0 ]'
  check "S-021 AC8: no show was drawn (no \"hive reveal:\" line: no drag enters the bench's hidden catcher)" '[ "${shows:-0}" -gt 0 ] && [ "${reveal:-0}" -eq 0 ]'
  check "S-021 AC8: median warm catcher_ms of hold shows, from the end of the threshold to the catcher's ready commit, under 150 ms" \
    '[ -n "$median" ] && python3 -I -c "import sys; sys.exit(not float(sys.argv[1]) < 150)" "$median"'
  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
} | tee "$OUT/budget-hold-summary.txt"
grep -q '^RESULT PASS' "$OUT/budget-hold-summary.txt"
exit $?
fi

{
  log="$OUT/budget-drop"
  echo "== measure.sh $RUNS $IDLE (DROP=1)"
  env -u SBARHIVE_SURFACE -u SHOW_TOKEN -u PREVIEW -u HOLD DROP=1 SBARHIVE_DEBUG_FOCUS=1 \
    CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
    nice -n 19 "$ROOT/crates/ui/tests/measure.sh" "$RUNS" "$IDLE" >"$log.txt" 2>&1
  raw=$(sed -n 's/^raw output: //p' "$log.txt")
  dir=$(dirname "${raw:-/nonexistent/x}")
  if [ -n "$raw" ] && [ -d "$dir" ] && [ "$dir" != / ]; then
    cp "$dir/daemon.log" "$log-daemon.log" 2>/dev/null
    cp "$dir/result.txt" "$log-result.txt" 2>/dev/null
    case "$dir" in /tmp/tmp.*) rm -rf "$dir" ;; esac
  fi
  grep -E '^(== summary|first show|warm catcher)' "$log.txt" | sed 's/^/INFO /'

  shows=$(grep -cE '^(first|warm) ' "$log-result.txt" 2>/dev/null)
  warm=$(grep -c '^warm ' "$log-result.txt" 2>/dev/null)
  dropl=$(grep -E '^(first|warm) ' "$log-result.txt" 2>/dev/null | grep -w 'mode=drop' | grep -cE '\bcatcher_ms=[0-9.]+')
  median=$(sed -n 's/^warm catcher_ms median \([0-9.]*\) n .*/\1/p' "$log.txt" | head -1)
  both=$(cat "$log-daemon.log" "$log-result.txt" 2>/dev/null)
  layer=$(grep -cE '^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$' <<<"$both")
  window=$(grep -c '^hive surface: window' <<<"$both")
  focus=$(grep -c '^hive focus: true' <<<"$both")
  reveal=$(grep -c '^hive reveal:' <<<"$both")
  echo "INFO drop: shows ${shows:-0} (warm ${warm:-0}), mode=drop lines with catcher_ms ${dropl:-0}, hive surface layer ${layer:-0} window ${window:-0}, hive focus: true ${focus:-0}, hive reveal ${reveal:-0}, median ${median:-none} ms"
  check "AC12: measure.sh ran all $((RUNS + 1)) daemon shows" '[ "${warm:-0}" -eq "$RUNS" ] && [ "${shows:-0}" -eq $((RUNS + 1)) ]'
  check "AC12, S-021 AC8: every show line says mode=drop with catcher_ms (hive_bench show --drop)" '[ "${shows:-0}" -gt 0 ] && [ "${dropl:-0}" -eq "${shows:-x}" ]'
  check "AC12: every show was a layer surface (one \"hive surface: layer\" line per show, no window line)" \
    '[ "${layer:-0}" -eq "${shows:-x}" ] && [ "${window:-0}" -eq 0 ]'
  check "AC12: no show took the keyboard (no \"hive focus: true\": drop mode, S-018 AC13)" '[ "${shows:-0}" -gt 0 ] && [ "${focus:-0}" -eq 0 ]'
  check "S-021 AC8: no show was drawn (no \"hive reveal:\" line: no drag enters the bench's hidden catcher)" '[ "${shows:-0}" -gt 0 ] && [ "${reveal:-0}" -eq 0 ]'
  check "AC12, S-021 AC8: median warm catcher_ms of drop-mode shows, from the request to the catcher's ready commit, under 150 ms" \
    '[ -n "$median" ] && python3 -I -c "import sys; sys.exit(not float(sys.argv[1]) < 150)" "$median"'
  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
} | tee "$OUT/budget-drop-summary.txt"
grep -q '^RESULT PASS' "$OUT/budget-drop-summary.txt"
