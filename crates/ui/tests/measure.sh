#!/usr/bin/env bash
# Measures S-001 AC1 (IPC request to first frame) and AC2 (idle daemon RSS)
# with examples/hive_bench.rs, inside a private headless KWin with a throwaway
# HOME, its own runtime dir, Wayland socket and session bus. Never touches the
# live desktop.
#
# Usage: crates/ui/tests/measure.sh [runs] [idle_seconds]
#   CPUS=0,1  pins the compositor and the daemon to those cores (taskset).
#   SHOW_TOKEN=T  passes an activation token with every show, which takes the
#             path that builds a fresh hive window (S-013).
#   DROP=1    every show is a drop-mode show (S-014 AC12): hive_bench gets
#             "show --drop", which presents the hive the way `kawaro drop`
#             does (Hive::present_drop), and prints "mode=drop" in its line.
#   HOLD=1    S-019e AC20: every show is a drop show in hold mode, the way
#             the daemon makes one after the long press threshold:
#             hive_bench gets "show --drop --hold"; its daemon passes the
#             request through the long press gate (kawaro_core::hold,
#             threshold 400 ms) and shows the hive when the gate's timer
#             gives the show. The show line then carries "mode=hold",
#             "hold_ms=400", "catcher_ms=A" timed from the threshold's end
#             (the gate's show) to the catcher's ready commit, and
#             "press_to_catcher_ms=B" from the request to that commit
#             (B is the threshold plus A); see S-021 AC8 below.
#   S-021 AC8: a drop show on the layer path draws nothing until a drag
#             enters, so with DROP=1 (and HOLD=1) hive_bench prints
#             "catcher_ms=A" (to the catcher's ready commit, its first
#             commit after its first configure) instead of first_frame_ms,
#             and with HOLD=1 "press_to_catcher_ms=B"; the summary adds
#             their warm medians and the warm catcher_ms max.
#   PREVIEW=1 every show presents one selected file whose only thumbnail is a
#             1024 px "xx-large" one (S-018 AC27): the file and the thumbnail
#             are made in this script's sandbox (tests/overlay/thumbs.py), and
#             hive_bench gets "--file PATH" for its daemon and cold runs.
#   X11=1     S-114: no KWin. The shows go to the X server named by DISPLAY
#             (tests/x11/run.sh starts a private Xvfb with xfwm4 and calls this
#             with X11=1), through the window path (the X11 hive window).
# Prints one line per request, then the first show on its own and the median
# of the rest, then the RSS after the idle wait.
set -u
RUNS="${1:-20}"
IDLE="${2:-60}"
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
(cd "$ROOT" && cargo build --quiet --release -p kawaro-ui --example hive_bench) || exit 2
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/release/examples/hive_bench"
OUT="$(mktemp -d)"
PIN=()
[ -n "${CPUS:-}" ] && PIN=(taskset -c "$CPUS")

inner() {
  unset XDG_ACTIVATION_TOKEN
  local cpid=""
  if [ -z "${X11:-}" ]; then
    unset DISPLAY
    "${PIN[@]}" kwin_wayland --virtual --socket "$WAYLAND_DISPLAY" --width 1920 --height 1080 >"$OUT/kwin.log" 2>&1 &
    cpid=$!
    for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
    [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "compositor did not start"; tail "$OUT/kwin.log"; return 1; }
  fi
  sleep 2
  local files=()
  if [ -n "${PREVIEW:-}" ]; then
    mkdir -p "$HOME/preview"
    printf 'selected file\n' >"$HOME/preview/photo.png"
    python3 -I "$ROOT/tests/overlay/thumbs.py" "$HOME/preview/photo.png" "$XDG_CACHE_HOME" xx-large 1024 1024 noise \
      || { echo "preview thumbnail not written"; return 1; }
    files=(--file "$HOME/preview/photo.png")
  fi

  echo "== cold start"
  for i in $(seq 5); do
    t0=$(date +%s%3N)
    line=$("${PIN[@]}" "$BIN" run "${files[@]}" 2>/dev/null | tr '\n' ' ')
    ff=$(grep -o 'first_frame_epoch_ms=[0-9]*' <<<"$line" | cut -d= -f2)
    echo "cold $i exec_to_frame_ms=$(( ${ff:-0} - t0 )) $line"
  done

  echo "== daemon"
  "${PIN[@]}" "$BIN" daemon "${files[@]}" 2>"$OUT/daemon.log" &
  local dpid=$!
  for _ in $(seq 50); do [ -S "$XDG_RUNTIME_DIR/kawaro-ui-bench.sock" ] && break; sleep 0.1; done
  sleep 1
  local tok=(); [ -n "${SHOW_TOKEN:-}" ] && tok=(--token "$SHOW_TOKEN")
  [ -n "${DROP:-}${HOLD:-}" ] && tok+=(--drop)
  [ -n "${HOLD:-}" ] && tok+=(--hold)
  echo "first $("$BIN" show "${tok[@]}" 2>&1 | tr '\n' ' ')"
  "$BIN" ctl hide; sleep 0.5
  for i in $(seq "$RUNS"); do
    echo "warm $i $("$BIN" show "${tok[@]}" 2>&1 | tr '\n' ' ')"
    "$BIN" ctl hide
    sleep 0.5
  done
  sleep "$IDLE"
  echo "rss_kb_after_idle=$(awk '/VmRSS/{print $2}' /proc/$dpid/status) idle_s=$IDLE"
  "$BIN" ctl quit
  if [ -n "$cpid" ]; then
    kill "$cpid" 2>/dev/null
    timeout 5 tail --pid="$cpid" -f /dev/null || kill -9 "$cpid" 2>/dev/null
  fi
}

export -f inner
export BIN OUT RUNS IDLE CPUS SHOW_TOKEN PREVIEW DROP HOLD ROOT X11
SANDBOX="$(mktemp -d)"
RUNTIME="$(mktemp -d)"
chmod 700 "$RUNTIME"
# X11=1 keeps DISPLAY (the private X server of tests/x11/run.sh) and sets no
# Wayland socket, so the hive takes the X11 window path.
KEEP_DISPLAY=(-u DISPLAY)
WAYLAND_SOCKET_NAME=wayland-sbh-measure
if [ -n "${X11:-}" ]; then KEEP_DISPLAY=(); WAYLAND_SOCKET_NAME=""; fi
env -u WAYLAND_DISPLAY "${KEEP_DISPLAY[@]}" -u HYPRLAND_INSTANCE_SIGNATURE \
  -u XDG_CURRENT_DESKTOP -u XDG_SESSION_DESKTOP -u DESKTOP_SESSION \
  -u SWAYSOCK -u I3SOCK -u DBUS_SESSION_BUS_ADDRESS \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$RUNTIME" ${WAYLAND_SOCKET_NAME:+WAYLAND_DISPLAY=$WAYLAND_SOCKET_NAME} \
  dbus-run-session -- bash -c 'PIN=(); [ -n "${CPUS:-}" ] && PIN=(taskset -c "$CPUS"); inner' \
  | tee "$OUT/result.txt"
rm -rf "$SANDBOX" "$RUNTIME"

median() { sort -n | awk '{a[NR]=$1} END{if(NR) print a[int((NR+1)/2)], "n", NR; else print "none"}'; }
echo "== summary"
echo "cold exec_to_frame_ms median $(grep '^cold ' "$OUT/result.txt" | grep -o 'exec_to_frame_ms=[0-9]*' | cut -d= -f2 | median)"
echo "first show after daemon start: $(grep '^first ' "$OUT/result.txt" | cut -d' ' -f2-)"
echo "warm first_frame_ms median $(grep '^warm ' "$OUT/result.txt" | grep -o 'first_frame_ms=[0-9.]*' | cut -d= -f2 | median)"
echo "warm first_frame_ms max $(grep '^warm ' "$OUT/result.txt" | grep -o 'first_frame_ms=[0-9.]*' | cut -d= -f2 | sort -n | tail -1)"
echo "warm catcher_ms median $(grep '^warm ' "$OUT/result.txt" | grep -o '\bcatcher_ms=[0-9.]*' | cut -d= -f2 | median)"
echo "warm catcher_ms max $(grep '^warm ' "$OUT/result.txt" | grep -o '\bcatcher_ms=[0-9.]*' | cut -d= -f2 | sort -n | tail -1)"
[ -n "${HOLD:-}" ] && echo "warm press_to_catcher_ms median $(grep '^warm ' "$OUT/result.txt" | grep -o 'press_to_catcher_ms=[0-9.]*' | cut -d= -f2 | median)"
grep '^rss_kb' "$OUT/result.txt"
echo "raw output: $OUT/result.txt"
