#!/usr/bin/env bash
# Drop spike on KWin: a GTK 4 window starts a file drag, a global shortcut is
# pressed mid-drag, the drop window (spike) is started mid-drag, and the file
# is dropped on a cell. Runs in a private kwin_wayland --virtual with a
# throwaway HOME, XDG dirs, runtime dir and session bus. Never touches the
# live desktop or profile.
# Usage: spikes/drop/kwin.sh [LEVEL]   (focus stealing prevention 0..4, default 1)
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
LEVEL="${1:-1}"; MODE="l$LEVEL${FIRST:+-first}"
OUT="$HERE/out"; mkdir -p "$OUT"
[ -x "$HERE/target/release/drop-spike" ] || { echo "build first: cargo build --release"; exit 2; }
export HERE MODE OUT LEVEL

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  local log="$OUT/kwin-$MODE"
  mkdir -p "$XDG_CONFIG_HOME"
  printf '[Windows]\nFocusStealingPreventionLevel=%s\n' "$LEVEL" >"$XDG_CONFIG_HOME/kwinrc"
  # Lets the private test client use org_kde_kwin_fake_input in this
  # private compositor only.
  KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 QT_FORCE_STDERR_LOGGING=1 \
    QT_LOGGING_RULES='kwin_scripting.debug=true;js.debug=true' \
    kwin_wayland --virtual --no-lockscreen --socket "$WAYLAND_DISPLAY" --width 1280 --height 800 >"$log-kwin.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  sleep 2

  local n=0
  script() { # script TAG JS: run a KWin script, keep it loaded
    n=$((n + 1))
    local js="$XDG_RUNTIME_DIR/s$n.js"
    printf '%s\n' "$2" >"$js"
    local id
    id=$(gdbus call --session --dest org.kde.KWin --object-path /Scripting \
      --method org.kde.kwin.Scripting.loadScript "$js" "$1" | grep -o '[0-9]\+' | head -1)
    gdbus call --session --dest org.kde.KWin --object-path "/Scripting/Script$id" \
      --method org.kde.kwin.Script.run >/dev/null 2>&1
    sleep 0.4
  }
  # The same KGlobalAccel path the GlobalShortcuts portal backend uses.
  script sbh-shortcut 'print("SBH script loaded " + registerShortcut("SbhDrop", "SbarHive drop test", "Meta+Ctrl+F11", function () { print("SBH shortcut fired"); }));'
  gdbus call --session --dest org.kde.kglobalaccel --object-path /kglobalaccel --method org.kde.KGlobalAccel.allComponents >"$log-accel.txt" 2>&1

  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  "$HERE/target/release/kwin_input" <"$f" >"$log-input.log" 2>&1 &
  exec 3>"$f"
  local W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"; printf 'x\n' >"$W/photo one.png"
  GTK_A11Y=none python3 "$HERE/source.py" "$W/photo one.png" >"$log-source.log" 2>&1 &
  local src=$!
  local spike=
  start_spike() { DROP_SPIKE_EXIT=1 "$HERE/target/release/drop-spike" >"$log-spike.log" 2>&1 & spike=$!; }
  sleep 3
  # Control: the shortcut with no drag.
  echo "keydown 125" >&3; echo "keydown 29" >&3; echo "keydown 87" >&3; echo "sleep 50" >&3; echo "keyup 87" >&3; echo "keyup 29" >&3; echo "keyup 125" >&3
  sleep 0.5
  echo "shortcut fired with no drag: $(grep -c 'SBH shortcut fired' "$log-kwin.log")"
  # FIRST=1: the shortcut case "hive first, then drag": the spike maps
  # after the source and before the press.
  [ -n "${FIRST:-}" ] && { start_spike; sleep 2; }
  # Press on the source (top left area) and drag past GTK's threshold.
  echo "move 150 150" >&3; echo "sleep 200" >&3; echo "down" >&3
  for i in 1 2 3 4 5 6; do echo "move $((150 + i * 10)) $((150 + i * 5))" >&3; echo "sleep 50" >&3; done
  sleep 1
  echo "keydown 125" >&3; echo "keydown 29" >&3; echo "keydown 87" >&3; echo "sleep 50" >&3; echo "keyup 87" >&3; echo "keyup 29" >&3; echo "keyup 125" >&3
  sleep 0.5
  [ -z "${FIRST:-}" ] && start_spike
  sleep 2
  script sbh-geo 'workspace.windowList().forEach(function (w) { if (w.caption === "drop-spike") { var g = w.frameGeometry; print("SBH geo " + g.x + " " + g.y + " " + g.width + " " + g.height); } });'
  local geo; geo=$(grep -o 'SBH geo .*' "$log-kwin.log" | tail -1)
  script sbh-st 'var a = workspace.activeWindow; print("SBH focus " + (a ? a.caption : null) + " stack " + workspace.stackingOrder.map(function (w) { return w.caption; }).join(","));'
  echo "$(grep -o 'SBH focus.*' "$log-kwin.log" | tail -1)"
  echo "spike window: ${geo:-not found}"
  read -r _ _ gx gy gw gh <<<"$geo"
  if [ -n "${gx:-}" ]; then
    local cx=$((gx + gw / 2)) cy=$((gy + gh / 2))
    # Over the window but in a corner (no cell), then to the right cell (index 2 or so), then drop.
    echo "move $((gx + 5)) $((gy + 5))" >&3; echo "sleep 150" >&3
    echo "move $((cx + 50)) $((cy))" >&3; echo "sleep 150" >&3
    echo "move $((cx + 105)) $((cy + 2))" >&3; echo "sleep 300" >&3
  fi
  echo "up" >&3
  sleep 2
  exec 3>&-
  echo "shortcut fired in total: $(grep -c 'SBH shortcut fired' "$log-kwin.log")"
  echo "source: $(tr '\n' ' ' <"$log-source.log")"
  echo "spike:  $(tr '\n' ' ' <"$log-spike.log")"
  kill $src ${spike:-} 2>/dev/null
  kill $kwin 2>/dev/null
  timeout 5 tail --pid=$kwin -f /dev/null || kill -9 $kwin
}
export -f inner

SANDBOX="$(mktemp -d)"
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates can reach the live desktop or the real profile.
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh-drop-kwin \
  dbus-run-session -- bash -c inner 2>"$OUT/kwin-$MODE-bus.log" | tee "$OUT/kwin-$MODE.txt"
