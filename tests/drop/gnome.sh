#!/usr/bin/env bash
# S-014b drop receiver on GNOME Shell (Mutter), AC5, AC6, AC7. A GTK 4 window
# drags a PNG, `sbarhive drop` is run mid-drag (what the shortcut does), and
# the file is dropped: show 1 on the WEBP cell, show 2 on a transparent
# corner, show 3 on the WEBP cell with Shift held, show 4 on the centre cell.
# The daemon runs with WAYLAND_DEBUG=1 so the trace shows a wl_data_device
# per new surface. Runs in a private headless gnome-shell with a throwaway
# HOME, XDG dirs, runtime dir and session bus. Never touches the live
# desktop or profile.
# Usage: tests/drop/gnome.sh [smart|strict] [target/release]
#   Mutter focus-new-windows, default smart
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
POLICY=smart
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in smart|strict) POLICY="$a" ;; *) BIN="$a" ;; esac
done
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
OUT="${DROP_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT POLICY

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  # shellcheck source=lib.sh
  source "$HERE/lib.sh"
  OUT_LOG="$OUT/gnome-$POLICY"
  local log="$OUT_LOG"
  : >"$log-cli.log"
  # Writes the throwaway HOME's dconf through the private bus only.
  gsettings set org.gnome.desktop.wm.preferences focus-new-windows "$POLICY"
  # No first-login welcome dialog: it is system modal and blocks input.
  gsettings set org.gnome.shell welcome-dialog-last-shown-version '999'
  # --unsafe-mode only opens Shell.Eval to this test on the private bus.
  gnome-shell --headless --wayland --no-x11 --unsafe-mode --wayland-display "$WAYLAND_DISPLAY" \
    --virtual-monitor 1280x800 >"$log-shell.log" 2>&1 &
  local shell=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "gnome-shell did not start"; tail "$log-shell.log"; return 1; }
  sleep 3
  shell_eval() {
    gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
      --method org.gnome.Shell.Eval "$1" 2>&1
  }
  # GNOME starts in the overview; leave it, as a user would after logging in.
  for _ in $(seq 20); do
    shell_eval 'Main.overview.hide(); Main.overview.visible' | grep -q false && break
    sleep 0.5
  done

  hive_geo() {
    shell_eval 'global.get_window_actors().map(a => a.meta_window)
      .filter(w => w.get_title() === "SbarHive" && !w.minimized)
      .map(w => { const r = w.get_frame_rect(); return [r.x, r.y, r.width, r.height].join(" "); }).join("")' \
      | grep -o '[0-9-]\+ [0-9-]\+ [0-9]\+ [0-9]\+' | head -1
  }

  local fifo="$XDG_RUNTIME_DIR/input.fifo"; mkfifo "$fifo"
  python3 "$HERE/input_gnome.py" "$fifo" >"$log-input.log" 2>&1 &
  local input=$!
  exec 3>"$fifo"
  # `to` returns when the pointer is there; give each command time to run.
  ptr_move() { echo "to $1 $2" >&3; sleep 0.3; }
  ptr_down() { echo "down" >&3; sleep 0.1; }
  ptr_up() { echo "up" >&3; sleep 0.1; }
  shift_down() { echo "keydown Shift_L" >&3; sleep 0.1; }
  shift_up() { echo "keyup Shift_L" >&3; sleep 0.1; }
  sleep 1
  # Mutter adds the virtual devices on the first event; plug them in now.
  echo "key Shift_L" >&3; sleep 10

  W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  local i
  for i in one two three four; do cp "$ROOT/tests/fixtures/images/alpha.png" "$W/photo $i.png"; done
  NEXT="$XDG_RUNTIME_DIR/next"; printf '%s\n' "$W/photo one.png" >"$NEXT"
  DLOG="$log-daemon.log"; SLOG="$log-source.log"

  # English layout, so the WEBP cell is top right of the centre. No warm-up
  # show: the first drop-mode show is part of AC5.
  SBARHIVE_LANG=en WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$DLOG" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done

  GTK_A11Y=none python3 "$HERE/source.py" "$NEXT" >"$SLOG" 2>&1 &
  local src=$!
  sleep 4

  cycle 1 "$W/photo one.png" webp 0
  cycle 2 "$W/photo two.png" corner 0
  cycle 3 "$W/photo three.png" webp 1
  cycle 4 "$W/photo four.png" centre 0
  protocol_checks
  summary

  echo quit >&3; exec 3>&-
  "$BIN/sbarhive" quit >/dev/null 2>&1
  sleep 0.5
  kill $src $d $input 2>/dev/null
  kill $shell 2>/dev/null
  timeout 5 tail --pid=$shell -f /dev/null || kill -9 $shell
}
export -f inner

echo "== gnome $POLICY"
SANDBOX="$(mktemp -d)"
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates (portals, dconf) can reach the live desktop or the real profile.
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh-drop-gnome \
  dbus-run-session -- bash -c inner 2>"$OUT/gnome-$POLICY-bus.log" | tee "$OUT/gnome-$POLICY.txt"
grep -q '^RESULT PASS' "$OUT/gnome-$POLICY.txt"
