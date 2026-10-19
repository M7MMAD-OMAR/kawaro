#!/usr/bin/env bash
# Measures the hive inside a private headless compositor, never on the live desktop.
# Usage: measure.sh kwin|gnome [runs]
# Prints one line per run, then medians. Snapshots land in out/.
set -u
COMP="${1:?kwin or gnome}"
RUNS="${2:-10}"
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN="$HERE/target/release/hive-spike"
OUT="$HERE/out"
mkdir -p "$OUT"

inner() {
  unset DISPLAY XDG_ACTIVATION_TOKEN
  case "$COMP" in
    kwin)  kwin_wayland --virtual --socket "$WAYLAND_DISPLAY" --width 1920 --height 1080 >"$OUT/$COMP.log" 2>&1 & ;;
    gnome) gnome-shell --headless --wayland --no-x11 --wayland-display "$WAYLAND_DISPLAY" \
             --virtual-monitor 1920x1080 >"$OUT/$COMP.log" 2>&1 & ;;
  esac
  local cpid=$!
  for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "compositor did not start"; tail "$OUT/$COMP.log"; return 1; }
  sleep 2

  echo "== cold start ($COMP)"
  for i in $(seq "$RUNS"); do
    t0=$(date +%s%3N)
    line=$("$BIN" run --exit-after 2500 2>/dev/null | tr '\n' ' ')
    ff=$(grep -o 'first_frame_epoch_ms=[0-9]*' <<<"$line" | cut -d= -f2)
    echo "cold $i exec_to_frame_ms=$(( ${ff:-0} - t0 )) $line"
  done

  echo "== daemon show ($COMP)"
  "$BIN" daemon 2>"$OUT/$COMP-daemon.log" &
  local dpid=$!
  for _ in $(seq 50); do [ -S "$XDG_RUNTIME_DIR/sbarhive-spike.sock" ] && break; sleep 0.1; done
  sleep 1
  for i in $(seq "$RUNS"); do
    echo "warm $i $("$BIN" show | tr '\n' ' ')"
    "$BIN" ctl hide
    sleep 0.5
  done
  echo "rss_kb_after_idle=$(sleep 5; awk '/VmRSS/{print $2}' /proc/$dpid/status)"
  "$BIN" ctl quit

  for lang in en ar; do
    "$BIN" snapshot "$OUT/$TAG-$lang.png" --lang $lang
    "$BIN" snapshot "$OUT/$TAG-$lang-light.png" --lang $lang --light
  done
  kill $cpid 2>/dev/null
  timeout 5 tail --pid=$cpid -f /dev/null || kill -9 $cpid
}

export -f inner
TAG="$COMP-${SPIKE_RENDERER:-default}"
export COMP RUNS BIN OUT TAG
# Point the private session bus at the private compositor before it starts, so
# nothing it activates (portals) can reach the live desktop.
# A throwaway HOME keeps the private compositor away from the real profile:
# without it gnome-shell loads the owner's extensions and writes to their dconf.
SANDBOX="$(mktemp -d)"
HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh \
  dbus-run-session -- bash -c inner | tee "$OUT/$TAG.txt"

echo "== medians ($TAG)"
for key in exec_to_frame_ms first_frame_ms; do
  for kind in cold warm; do
    grep "^$kind " "$OUT/$TAG.txt" | grep -o "$key=[0-9.]*" | cut -d= -f2 | sort -n |
      awk -v k="$kind $key" '{a[NR]=$1} END{if(NR) print k, "median", a[int((NR+1)/2)], "n", NR}'
  done
done
for kind in cold warm; do
  echo "$kind focused_true $(grep "^$kind " "$OUT/$TAG.txt" | grep -c 'focused=true') of $(grep -c "^$kind " "$OUT/$TAG.txt")"
done
