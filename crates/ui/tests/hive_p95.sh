#!/usr/bin/env bash
# S-062 AC11: invoke to first frame of the hive stays under 150 ms at the 95th
# percentile over 20 warm shows, measured with the opaque plate present.
# Runs crates/ui/tests/measure.sh (private headless KWin) and reads the
# first_frame_ms of its warm lines.
#   crates/ui/tests/hive_p95.sh
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
if ! grep -q 'Tokens.hive-plate' "$HERE/../ui/hive.slint"; then
  echo "FAIL AC11: hive.slint has no plate (Tokens.hive-plate), nothing to measure"; exit 1
fi
OUTPUT="$("$HERE/measure.sh" 20 0 2>&1)" || { echo "$OUTPUT"; echo "FAIL AC11: measure.sh failed"; exit 1; }
RAW="$(printf '%s\n' "$OUTPUT" | sed -n 's/^raw output: //p')"
[ -f "$RAW" ] || { echo "FAIL AC11: no raw output"; exit 1; }
VALUES="$(grep '^warm ' "$RAW" | grep -o 'first_frame_ms=[0-9.]*' | cut -d= -f2 | sort -n)"
N="$(printf '%s\n' "$VALUES" | grep -c .)"
[ "$N" -ge 20 ] || { echo "FAIL AC11: $N warm samples, want 20"; exit 1; }
P95="$(printf '%s\n' "$VALUES" | awk -v n="$N" '{a[NR]=$1} END{i=int(0.95*n+0.999999); print a[i]}')"
echo "first_frame_ms p95 $P95 over $N shows"
awk -v p="$P95" 'BEGIN{exit !(p < 150)}' || { echo "FAIL AC11: p95 $P95 ms >= 150 ms"; exit 1; }
echo "ok AC11"
