#!/usr/bin/env bash
# Overlay spike on GNOME: no zwlr_layer_shell_v1, so the hive must fall back to
# the winit xdg_toplevel. Private gnome-shell --headless, throwaway HOME, XDG
# dirs, runtime dir and session bus. Never touches the live desktop.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="$HERE/out"; mkdir -p "$OUT"
BIN="$HERE/target/release"
[ -x "$BIN/overlay-spike" ] || { echo "build first: cargo build --release"; exit 2; }
export HERE OUT BIN

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  gnome-shell --headless --wayland --no-x11 --unsafe-mode --wayland-display "$WAYLAND_DISPLAY" \
    --virtual-monitor 1280x800 >"$OUT/gnome-shell.log" 2>&1 &
  local shell=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  sleep 3
  "$BIN/overlay-spike" globals >"$OUT/gnome-globals.txt" 2>&1
  echo "layer-shell: $(grep zwlr_layer_shell "$OUT/gnome-globals.txt" || echo absent)"
  SPIKE_EXIT_MS=2000 "$BIN/overlay-spike" hive >"$OUT/gnome-hive.log" 2>&1
  grep -E "unavailable|first frame|present" "$OUT/gnome-hive.log" | sed 's/^/    /'
  kill $shell 2>/dev/null
  timeout 5 tail --pid=$shell -f /dev/null || kill -9 $shell
}
export -f inner

SANDBOX="$(mktemp -d)"
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh-overlay-gnome \
  dbus-run-session -- bash -c inner 2>"$OUT/gnome-bus.log" | tee "$OUT/gnome.txt"
rm -rf "$SANDBOX"
