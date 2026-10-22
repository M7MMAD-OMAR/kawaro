#!/usr/bin/env bash
# S-021 AC10 (backlog/S-021-drop-only-on-drag.md): the window path is
# unchanged. Runs tests/drop/gnome.sh smart and tests/overlay/gnome.sh as
# they are (each in its own private headless gnome-shell with a throwaway
# HOME, one after the other), then reads their daemon logs: both must print
# RESULT PASS, and neither daemon log may hold a "hive probe:" or
# "hive reveal:" line (those are printed on the layer path only).
# Never touches the live desktop: both scripts set up their own sandbox.
# Usage: tests/drop/window_path.sh [target/release]
# Conditions go to check() in single quotes and are expanded by eval.
# shellcheck disable=SC2016
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
BIN="${1:-$ROOT/target/release}"
DOUT="${DROP_OUT:-$HERE/out}"
OOUT="${OVERLAY_OUT:-$ROOT/tests/overlay/out}"
FAILS=0
check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }

DROP_OUT="$DOUT" "$HERE/gnome.sh" smart "$BIN" >/dev/null 2>&1
check "S-021 AC10: tests/drop/gnome.sh smart prints RESULT PASS" 'grep -q "^RESULT PASS" "$DOUT/gnome-smart.txt"'
OVERLAY_OUT="$OOUT" "$ROOT/tests/overlay/gnome.sh" "$BIN" >/dev/null 2>&1
check "S-021 AC10: tests/overlay/gnome.sh prints RESULT PASS" 'grep -q "^RESULT PASS" "$OOUT/gnome.txt"'
for l in "$DOUT/gnome-smart-daemon.log" "$OOUT/gnome-daemon.log"; do
  n=$(grep -cE '^hive (probe|reveal):' "$l" 2>/dev/null); [ -n "$n" ] || n=none
  echo "INFO S-021 AC10: $l: $n probe or reveal lines"
  check "S-021 AC10: no \"hive probe:\" or \"hive reveal:\" line in $(basename "$l") (window path)" '[ -s "$l" ] && [ "$n" = 0 ]'
done
if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
[ "$FAILS" -eq 0 ]
