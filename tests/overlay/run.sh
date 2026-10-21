#!/usr/bin/env bash
# Runs the whole overlay suite (S-018), one script after another, each in its
# own private headless compositor, and prints one summary line per step:
#   docs.sh                      AC19, AC20 (no compositor)
#   kwin.sh                      layer mode (the default), then every other
#                                mode: window input token a11y drop scheme 1.5
#                                anchor (S-019c)
#   gnome.sh                     the window path on a headless GNOME Shell
#   focus.sh                     AC10, every level and case
#   budget.sh                    AC18 budgets (measure.sh 20 60, twice)
#   preview.sh                   AC27 first frame with a 1024 px thumbnail
# Build first: cargo build --release, plus
# cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml
# Each step's output goes to tests/overlay/out/run-<step>.txt. Exits 1 if any
# step fails. Usage: tests/overlay/run.sh
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="${OVERLAY_OUT:-$HERE/out}"
mkdir -p "$OUT"
FAILED=""
step() { # step NAME SCRIPT [ARGS...]
  local name=$1
  shift
  if "$@" >"$OUT/run-$name.txt" 2>&1; then
    echo "PASS $name"
  else
    echo "FAIL $name (see $OUT/run-$name.txt)"
    FAILED="$FAILED $name"
  fi
}

step docs "$HERE/docs.sh"
step kwin-layer "$HERE/kwin.sh"
for mode in window input token a11y drop scheme 1.5 anchor; do
  step "kwin-$mode" "$HERE/kwin.sh" "$mode"
done
step gnome "$HERE/gnome.sh"
step focus "$HERE/focus.sh"
step budget "$HERE/budget.sh"
step preview "$HERE/preview.sh"

if [ -z "$FAILED" ]; then echo "RESULT PASS"; else echo "RESULT FAIL:$FAILED"; fi
[ -z "$FAILED" ]
