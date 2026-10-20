#!/usr/bin/env bash
# Overlay spike on KWin: the real hive on a zwlr_layer_shell_v1 overlay, with
# focus follows mouse and the strictest focus stealing prevention, under a
# maximised GTK window. Private kwin_wayland --virtual, throwaway HOME, XDG
# dirs, runtime dir and session bus. Never touches the live desktop.
# Usage: spikes/overlay/kwin.sh [SCALE [POLICY [FSP]]]
#   SCALE output scale (1), POLICY FocusFollowsMouse or ClickToFocus, FSP 0..4 (4)
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
SCALE="${1:-1}"; POLICY="${2:-FocusFollowsMouse}"; FSP="${3:-4}"
OUT="$HERE/out"; mkdir -p "$OUT"
BIN="$HERE/target/release"
[ -x "$BIN/overlay-spike" ] || { echo "build first: cargo build --release"; exit 2; }
export HERE OUT BIN SCALE POLICY FSP

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  local tag="kwin-s$SCALE-$POLICY-fsp$FSP"
  mkdir -p "$XDG_CONFIG_HOME"
  # Focus follows mouse and focus stealing prevention Extreme (4).
  printf '[Windows]\nFocusPolicy=%s\nFocusStealingPreventionLevel=%s\n' "$POLICY" "$FSP" >"$XDG_CONFIG_HOME/kwinrc"
  # Lets shot.py (python3) use org.kde.KWin.ScreenShot2 in this private KWin only.
  mkdir -p "$XDG_DATA_HOME/applications"
  printf '[Desktop Entry]\nType=Application\nName=sbh shot\nExec=%s\nNoDisplay=true\nX-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2\n' "$(readlink -f "$(command -v python3)")" >"$XDG_DATA_HOME/applications/sbh-shot.desktop"
  kbuildsycoca6 --noincremental >/dev/null 2>&1
  KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 \
    kwin_wayland --virtual --no-lockscreen --socket "$WAYLAND_DISPLAY" --width 1280 --height 800 >"$OUT/$tag-kwin.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  sleep 2
  if [ "$SCALE" != 1 ]; then
    # kwin_wayland --virtual ignores --scale here; set it like System Settings does.
    timeout 10 kscreen-doctor -o >"$OUT/kwin-s$SCALE-outputs.txt" 2>&1
    local o=1
    timeout 10 kscreen-doctor "output.$o.scale.$SCALE" >>"$OUT/kwin-s$SCALE-outputs.txt" 2>&1; sleep 1
  fi
  "$BIN/overlay-spike" globals >"$OUT/$tag-globals.txt" 2>&1
  echo "layer-shell: $(grep zwlr_layer_shell "$OUT/$tag-globals.txt" || echo absent)"

  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  "$BIN/kwin_input" <"$f" >"$OUT/$tag-input.log" 2>&1 &
  exec 3>"$f"
  GTK_A11Y=none python3 "$HERE/other.py" "$XDG_RUNTIME_DIR/token" >"$OUT/$tag-other.log" 2>&1 &
  local other=$!
  sleep 2
  echo "step: other window up"
  # Focus the other window under the pointer first (focus follows mouse).
  echo "move 200 200" >&3; sleep 0.5
  local cx=640 cy=400
  # Screenshots run in the background: KWin answers only after its next repaint.
  shot() { (timeout 8 python3 "$HERE/shot.py" "$OUT/$tag-$1.png" >/dev/null 2>"$OUT/$tag-shot-$1.err" &); }
  ts() { echo "    step $1 t=$(date +%s.%3N | awk '{printf "%.3f", $1 % 100}')"; }
  key() { ts "key $1"; echo "keydown $1" >&3; echo "sleep 40" >&3; echo "keyup $1" >&3; }
  run() { # run NAME [ENV...]: start the hive and wait for it
    local name=$1; shift
    : >"$OUT/$tag-$name.log"
    env "$@" SPIKE_EXIT_MS=6000 "$BIN/overlay-spike" hive >"$OUT/$tag-$name.log" 2>&1 &
    spike=$!
    for _ in $(seq 50); do grep -q "first frame" "$OUT/$tag-$name.log" && break; sleep 0.1; done
    sleep 0.6
  }
  finish() { timeout 10 tail --pid=$spike -f /dev/null; kill -9 $spike 2>/dev/null; echo "--- $1"; grep -E "first frame|configured|hive [0-9]|focus|key |wl_keyboard|token|click outside|dismissed|chosen|stand-in|present" "$OUT/$tag-$1.log" | sed 's/^/    /'; echo "    other window: $(grep -c 'other: click' "$OUT/$tag-other.log") clicks so far, last active: $(grep 'other: active' "$OUT/$tag-other.log" | tail -1)"; }

  # 0. Esc right away, pointer still: does the overlay get the keyboard on map?
  run still
  sleep 0.5; key 1; sleep 0.5
  kill $spike 2>/dev/null; finish still

  # 0b. Same, with an activation token from a click in the other window.
  echo "move 100 700" >&3; sleep 0.3; echo "down" >&3; echo "up" >&3; sleep 0.5
  run token SPIKE_TOKEN="$(cat "$XDG_RUNTIME_DIR/token" 2>/dev/null)"
  sleep 0.5; key 1; sleep 0.5
  kill $spike 2>/dev/null; finish token

  # 1. Esc after roaming the pointer over the other window.
  run esc
  shot hive
  for p in "100 100" "1200 700" "300 650" "$((cx + 5)) $((cy + 5))" "50 400"; do echo "move $p" >&3; sleep 0.3; done
  key 1
  sleep 0.5; ts kill; kill $spike 2>/dev/null; finish esc

  # 1b. A normal winit window (settings stand-in) in the same process.
  run settings SPIKE_SETTINGS=1
  sleep 0.5; kill $spike 2>/dev/null; finish settings

  # 2. Click outside the hive: dismisses, must not reach the other window.
  local before; before=$(grep -c 'other: click' "$OUT/$tag-other.log")
  run outside
  echo "move 100 700" >&3; sleep 0.3; echo "down" >&3; echo "up" >&3; sleep 0.5
  kill $spike 2>/dev/null; finish outside
  echo "    clicks that reached the other window: $(( $(grep -c 'other: click' "$OUT/$tag-other.log") - before ))"

  # 3. Click on a ring-1 cell (right of the centre).
  run cell
  echo "move $((cx + 72)) $cy" >&3; sleep 0.3; echo "down" >&3; echo "up" >&3; sleep 0.5
  kill $spike 2>/dev/null; finish cell

  # 4. Keyboard only: Right arrow then Return, pointer parked on the other window.
  run keys
  echo "move 100 100" >&3; sleep 0.3; key 106; sleep 0.2; key 28; sleep 0.5
  kill $spike 2>/dev/null; finish keys

  # 5. Second present in the same process (the daemon path) for its timing.
  run again SPIKE_REPRESENT_MS=1000
  sleep 1.5; shot again; sleep 1
  echo "    RSS with the hive shown twice: $(awk '/^VmRSS:/ {print int($2 / 1024)}' /proc/$spike/status) MB"
  kill $spike 2>/dev/null; finish again

  # 6. Drop mode: the catcher takes no input, a click outside reaches the window below.
  before=$(grep -c 'other: click' "$OUT/$tag-other.log")
  run drop SPIKE_DROP=1
  echo "move 100 700" >&3; sleep 0.3; echo "down" >&3; echo "up" >&3; sleep 0.5
  kill $spike 2>/dev/null; finish drop
  echo "    clicks that reached the other window: $(( $(grep -c 'other: click' "$OUT/$tag-other.log") - before ))"

  # 7. Centre cell preview: a thumbnail written like Nautilus does, and a type icon.
  local W="$HOME/work"; mkdir -p "$W"
  ffmpeg -loglevel error -f lavfi -i testsrc=size=640x400:rate=1 -frames:v 1 "$W/صورة 1+&~,(x).png"
  printf '%%PDF-1.4\n' >"$W/report.pdf"
  python3 "$HERE/thumbs.py" "$W/صورة 1+&~,(x).png" >"$OUT/$tag-thumbs.txt" 2>&1
  "$BIN/overlay-spike" uri "$W/صورة 1+&~,(x).png" >>"$OUT/$tag-thumbs.txt"
  "$BIN/overlay-spike" thumb "$W/صورة 1+&~,(x).png" "$W/report.pdf" >>"$OUT/$tag-thumbs.txt" 2>&1
  echo "--- thumbnails"; sed 's/^/    /' "$OUT/$tag-thumbs.txt"
  for which in thumb icon; do
    local file="$W/صورة 1+&~,(x).png"; [ $which = icon ] && file="$W/report.pdf"
    SPIKE_EXIT_MS=5000 "$BIN/overlay-spike" preview "$file" "$W/report.pdf" >"$OUT/$tag-preview-$which.log" 2>&1 &
    spike=$!
    sleep 1.2; shot preview-$which; echo "move 640 $((300 + ${#which}))" >&3; sleep 1.2
    key 1; sleep 0.3
    timeout 10 tail --pid=$spike -f /dev/null; kill -9 $spike 2>/dev/null
    echo "--- preview $which"; grep -E "preview|first frame" "$OUT/$tag-preview-$which.log" | sed 's/^/    /'
  done

  exec 3>&-
  kill $other 2>/dev/null
  kill $kwin 2>/dev/null
  timeout 5 tail --pid=$kwin -f /dev/null || kill -9 $kwin
}
export -f inner

SANDBOX="$(mktemp -d)"
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh-overlay-kwin \
  dbus-run-session -- bash -c inner 2>"$OUT/kwin-s$SCALE-$POLICY-fsp$FSP-bus.log" | tee "$OUT/kwin-s$SCALE-$POLICY-fsp$FSP.txt"
rm -rf "$SANDBOX"
