#!/usr/bin/env bash
# Measures S-001 AC1 (IPC request to first frame) and AC2 (idle daemon RSS)
# with examples/hive_bench.rs, inside a private headless KWin with a throwaway
# HOME, its own runtime dir, Wayland socket and session bus. Never touches the
# live desktop.
#
# Usage: crates/ui/tests/measure.sh [runs] [idle_seconds]
#   CPUS=0,1  pins the compositor and the daemon to those cores (taskset).
# Prints one line per request, then the first show on its own and the median
# of the rest, then the RSS after the idle wait.
set -u
RUNS="${1:-20}"
IDLE="${2:-60}"
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
(cd "$ROOT" && cargo build --quiet --release -p sbarhive-ui --example hive_bench) || exit 2
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/release/examples/hive_bench"
OUT="$(mktemp -d)"
PIN=()
[ -n "${CPUS:-}" ] && PIN=(taskset -c "$CPUS")

inner() {
  unset DISPLAY XDG_ACTIVATION_TOKEN
  "${PIN[@]}" kwin_wayland --virtual --socket "$WAYLAND_DISPLAY" --width 1920 --height 1080 >"$OUT/kwin.log" 2>&1 &
  local cpid=$!
  for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "compositor did not start"; tail "$OUT/kwin.log"; return 1; }
  sleep 2

  echo "== cold start"
  for i in $(seq 5); do
    t0=$(date +%s%3N)
    line=$("${PIN[@]}" "$BIN" run 2>/dev/null | tr '\n' ' ')
    ff=$(grep -o 'first_frame_epoch_ms=[0-9]*' <<<"$line" | cut -d= -f2)
    echo "cold $i exec_to_frame_ms=$(( ${ff:-0} - t0 )) $line"
  done

  echo "== daemon"
  "${PIN[@]}" "$BIN" daemon 2>"$OUT/daemon.log" &
  local dpid=$!
  for _ in $(seq 50); do [ -S "$XDG_RUNTIME_DIR/sbarhive-ui-bench.sock" ] && break; sleep 0.1; done
  sleep 1
  echo "first $("$BIN" show 2>&1 | tr '\n' ' ')"
  "$BIN" ctl hide; sleep 0.5
  for i in $(seq "$RUNS"); do
    echo "warm $i $("$BIN" show 2>&1 | tr '\n' ' ')"
    "$BIN" ctl hide
    sleep 0.5
  done
  sleep "$IDLE"
  echo "rss_kb_after_idle=$(awk '/VmRSS/{print $2}' /proc/$dpid/status) idle_s=$IDLE"
  "$BIN" ctl quit
  kill $cpid 2>/dev/null
  timeout 5 tail --pid=$cpid -f /dev/null || kill -9 $cpid 2>/dev/null
}

export -f inner
export BIN OUT RUNS IDLE CPUS
SANDBOX="$(mktemp -d)"
RUNTIME="$(mktemp -d)"
chmod 700 "$RUNTIME"
env -u WAYLAND_DISPLAY -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE \
  -u XDG_CURRENT_DESKTOP -u XDG_SESSION_DESKTOP -u DESKTOP_SESSION \
  -u SWAYSOCK -u I3SOCK -u DBUS_SESSION_BUS_ADDRESS \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY=wayland-sbh-measure \
  dbus-run-session -- bash -c 'PIN=(); [ -n "${CPUS:-}" ] && PIN=(taskset -c "$CPUS"); inner' \
  | tee "$OUT/result.txt"
rm -rf "$SANDBOX" "$RUNTIME"

median() { sort -n | awk '{a[NR]=$1} END{if(NR) print a[int((NR+1)/2)], "n", NR; else print "none"}'; }
echo "== summary"
echo "cold exec_to_frame_ms median $(grep '^cold ' "$OUT/result.txt" | grep -o 'exec_to_frame_ms=[0-9]*' | cut -d= -f2 | median)"
echo "first show after daemon start: $(grep '^first ' "$OUT/result.txt" | cut -d' ' -f2-)"
echo "warm first_frame_ms median $(grep '^warm ' "$OUT/result.txt" | grep -o 'first_frame_ms=[0-9.]*' | cut -d= -f2 | median)"
echo "warm first_frame_ms max $(grep '^warm ' "$OUT/result.txt" | grep -o 'first_frame_ms=[0-9.]*' | cut -d= -f2 | sort -n | tail -1)"
grep '^rss_kb' "$OUT/result.txt"
echo "raw output: $OUT/result.txt"
