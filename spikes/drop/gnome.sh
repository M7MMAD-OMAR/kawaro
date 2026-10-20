#!/usr/bin/env bash
# Drop spike on GNOME Shell (Mutter): a GTK 4 window starts a file drag, a
# global accelerator is pressed mid-drag, the drop window (spike) is started
# mid-drag, and the file is dropped on a cell. Runs in a private headless
# gnome-shell with a throwaway HOME, XDG dirs, runtime dir and session bus.
# Never touches the live desktop or profile.
# Usage: spikes/drop/gnome.sh [smart|strict]   (Mutter focus-new-windows)
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="$HERE/out"; mkdir -p "$OUT"
POLICY="${1:-smart}"
TAG="$POLICY${FIRST:+-first}"
[ -x "$HERE/target/release/drop-spike" ] || { echo "build first: cargo build --release"; exit 2; }
export HERE OUT POLICY TAG

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  local log="$OUT/gnome-$TAG"
  gsettings set org.gnome.desktop.wm.preferences focus-new-windows "$POLICY"
  # No first-login welcome dialog: it is system modal and blocks input.
  gsettings set org.gnome.shell welcome-dialog-last-shown-version '999'
  # --unsafe-mode only opens Shell.Eval to this test on the private bus.
  gnome-shell --headless --wayland --no-x11 --unsafe-mode --wayland-display "$WAYLAND_DISPLAY" \
    --virtual-monitor 1280x800 >"$log-shell.log" 2>&1 &
  local shell=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  sleep 3
  shell_eval() {
    gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
      --method org.gnome.Shell.Eval "$1" 2>&1
  }
  for _ in $(seq 20); do
    shell_eval 'Main.overview.hide(); Main.overview.visible' | grep -q false && break
    sleep 0.5
  done
  # A Mutter accelerator, the mechanism gnome-shell uses for keybindings it
  # grabs for other processes (MetaDisplay::accelerator-activated). Allowed in
  # the normal action mode only, like the default for grabbed accelerators.
  shell_eval 'const Meta = imports.gi.Meta; const Shell = imports.gi.Shell;
    const a = global.display.grab_accelerator("<Super><Control>F11", Meta.KeyBindingFlags.NONE);
    Main.wm.allowKeybinding(Meta.external_binding_name_for_action(a), Shell.ActionMode.NORMAL);
    global.display.connect("accelerator-activated", (d, act) => { if (act === a) log("SBH shortcut fired mode=" + Main.actionMode); });
    a' >"$log-accel.txt"

  local fifo="$XDG_RUNTIME_DIR/input.fifo"; mkfifo "$fifo"
  python3 "$HERE/input_gnome.py" "$fifo" >"$log-input.log" 2>&1 &
  local input=$!
  exec 3>"$fifo"
  sleep 1
  # Mutter adds the virtual devices on the first event; plug them in now.
  echo "key Shift_L" >&3; sleep 10

  local W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"; printf 'x\n' >"$W/photo one.png"
  GTK_A11Y=none python3 "$HERE/source.py" "$W/photo one.png" >"$log-source.log" 2>&1 &
  local src=$!
  sleep 4
  echo "action mode before: $(shell_eval 'Main.actionMode')"
  echo "key Super_L+Control_L+F11" >&3; sleep 1
  echo "shortcut fired with no drag: $(grep -c 'SBH shortcut fired' "$log-shell.log")"
  local spike=
  # FIRST=1: "hive first, then drag": the spike maps after the source and
  # before the press.
  if [ -n "${FIRST:-}" ]; then DROP_SPIKE_EXIT=1 "$HERE/target/release/drop-spike" >"$log-spike.log" 2>&1 & spike=$!; sleep 3; fi
  echo "to 150 150" >&3; sleep 1; echo "down" >&3; sleep 0.3
  echo "to 220 190" >&3; sleep 1
  echo "key Super_L+Control_L+F11" >&3; sleep 1
  echo "action mode mid-drag: $(shell_eval 'Main.actionMode')"
  if [ -z "${FIRST:-}" ]; then DROP_SPIKE_EXIT=1 "$HERE/target/release/drop-spike" >"$log-spike.log" 2>&1 & spike=$!; fi
  sleep 3
  local geo; geo=$(shell_eval 'global.get_window_actors().map(a => a.meta_window)
    .filter(w => w.get_title() === "drop-spike").map(w => { const r = w.get_frame_rect(); return [r.x, r.y, r.width, r.height].join(" "); }).join("")')
  geo=$(grep -o '[0-9]\+ [0-9]\+ [0-9]\+ [0-9]\+' <<<"$geo")
  echo "focus: $(shell_eval 'global.display.focus_window?.get_title()') stack: $(shell_eval 'global.display.sort_windows_by_stacking(global.display.list_all_windows()).map(w => w.get_title()).join(",")')"
  echo "spike window: ${geo:-not found} (all: $(shell_eval 'JSON.stringify(global.get_window_actors().map(a => a.meta_window).map(w => [w.get_title(), w.get_frame_rect().x, w.get_frame_rect().y]))'))"
  read -r gx gy gw gh <<<"$geo"
  if [ -n "${gx:-}" ]; then
    local cx=$((gx + gw / 2)) cy=$((gy + gh / 2))
    echo "to $((gx + 5)) $((gy + 5))" >&3; sleep 1
    echo "to $((cx + 105)) $((cy + 2))" >&3; sleep 1
  fi
  echo "up" >&3
  sleep 2
  echo "shortcut fired in total: $(grep -c 'SBH shortcut fired' "$log-shell.log") ($(grep -o 'SBH shortcut fired.*' "$log-shell.log" | tr '\n' ' '))"
  echo "source: $(tr '\n' ' ' <"$log-source.log")"
  echo "spike:  $(tr '\n' ' ' <"$log-spike.log")"
  echo quit >&3; exec 3>&-
  kill $src $spike $input 2>/dev/null
  kill $shell 2>/dev/null
  timeout 5 tail --pid=$shell -f /dev/null || kill -9 $shell
}
export -f inner

SANDBOX="$(mktemp -d)"
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates (portals, dconf) can reach the live desktop or the real profile.
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh-drop-gnome \
  dbus-run-session -- bash -c inner 2>"$OUT/gnome-$TAG-bus.log" | tee "$OUT/gnome-$TAG.txt"
