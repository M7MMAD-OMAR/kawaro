#!/usr/bin/env bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034
# S-014b drop receiver on GNOME Shell (Mutter), AC5, AC6, AC7. A GTK 4 window
# drags a PNG, `sbarhive drop` is run mid-drag (what the shortcut does), and
# the file is dropped: show 1 on the WEBP cell, show 2 on a transparent
# corner, show 3 on the WEBP cell with Shift held, show 4 on the centre cell.
# The daemon runs with WAYLAND_DEBUG=1 so the trace shows a wl_data_device
# per new surface. S-014c (AC8): show 5 drags a PDF and drops it on a
# corner; every show's cells at enter must equal `sbarhive actions` for its
# file (the daemon runs with SBARHIVE_DEBUG_FOCUS=1 for its diagnostic lines).
# The AC9 and AC10 screenshots are taken on KWin only (tests/drop/kwin.sh):
# both paths draw the same Slint component (S-018 AC7).
# Runs in a private headless gnome-shell with a throwaway
# HOME, XDG dirs, runtime dir and session bus. Never touches the live
# desktop or profile.
# S-014d (AC11), mode "shortcut": Super+Z is grabbed in this private shell
# with global.display.grab_accelerator in the normal action mode (the path
# the GlobalShortcuts portal backend and gnome-shell's own keybindings take,
# ADR 0003 result 4) and spawns a wrapper that marks the time and execs
# `sbarhive drop`. Custom shortcuts through gsd-media-keys are out of scope
# (S-014). Three drags each press Super+Z mid-drag and drop on the WEBP
# cell, then Esc is pressed mid-drag over the hive and before the hive
# (tests/drop/lib.sh, shortcut_shows). The five S-014b and S-014c shows are
# not run in this mode.
# Usage: tests/drop/gnome.sh [smart|strict] [shortcut] [target/release]
#   Mutter focus-new-windows, default smart
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
POLICY=smart
MODE=""
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in smart|strict) POLICY="$a" ;; shortcut) MODE=shortcut ;; *) BIN="$a" ;; esac
done
TAG="gnome-$POLICY${MODE:+-$MODE}"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
OUT="${DROP_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT POLICY MODE TAG

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  # shellcheck source=lib.sh
  source "$HERE/lib.sh"
  OUT_LOG="$OUT/$TAG"
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
      .filter(w => w.get_title() === "SbarHive" && !w.minimized && w.showing_on_its_workspace())
      .filter(w => { const r = w.get_frame_rect(); return r.width > 0 && r.height > 0; })
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
  cp "$ROOT/tests/fixtures/docs/one-page.pdf" "$W/report five.pdf"
  NEXT="$XDG_RUNTIME_DIR/next"; printf '%s\n' "$W/photo one.png" >"$NEXT"
  DLOG="$log-daemon.log"; SLOG="$log-source.log"

  # English layout, so the WEBP cell is top right of the centre. No warm-up
  # show: the first drop-mode show is part of AC5.
  SBARHIVE_LANG=en SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$DLOG" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done

  GTK_A11Y=none python3 "$HERE/source.py" "$NEXT" >"$SLOG" 2>&1 &
  local src=$!
  sleep 4

  if [ "$MODE" = shortcut ]; then
    make_shortcut_wrap
    local grab
    grab=$(shell_eval "const Meta = imports.gi.Meta; const Shell = imports.gi.Shell; const GLib = imports.gi.GLib;
      const a = global.display.grab_accelerator('<Super>z', Meta.KeyBindingFlags.NONE);
      Main.wm.allowKeybinding(Meta.external_binding_name_for_action(a), Shell.ActionMode.NORMAL);
      global.display.connect('accelerator-activated', (d, act) => {
        if (act === a) GLib.spawn_async(null, ['$SHORTCUT_WRAP'], null, GLib.SpawnFlags.DEFAULT, null);
      });
      a")
    echo "INFO AC11 setup: grab_accelerator <Super>z: $grab"
    check "AC11 setup: Super+Z grabbed in the normal action mode" 'grep -qE "true, .[1-9][0-9]*" <<<"$grab"'
    press_shortcut() {
      echo "key Super_L+z" >&3; sleep 0.3
      local ov; ov=$(shell_eval 'Main.overview.visible')
      check "AC11: Super+Z mid-drag does not open the overview" 'grep -q false <<<"$ov"'
    }
    press_esc() { echo "key Escape" >&3; }
    HIVE_DESTROY_RE='-> xdg_toplevel[#@][0-9]+\.destroy\('
    ESC_NO_KEYBOARD=0
    shortcut_shows
  else
  cycle 1 "$W/photo one.png" webp 0
  cycle 2 "$W/photo two.png" corner 0
  cycle 3 "$W/photo three.png" webp 1
  cycle 4 "$W/photo four.png" centre 0
  cycle 5 "$W/report five.pdf" corner 0
  protocol_checks
  enter_cells_checks
  fi
  summary

  echo quit >&3; exec 3>&-
  "$BIN/sbarhive" quit >/dev/null 2>&1
  sleep 0.5
  kill $src $d $input 2>/dev/null
  kill $shell 2>/dev/null
  timeout 5 tail --pid=$shell -f /dev/null || kill -9 $shell
}
export -f inner

echo "== gnome $POLICY${MODE:+ $MODE}"
SANDBOX="$(mktemp -d)"
RUNDIR="$(mktemp -d)"
trap 'rm -rf "$SANDBOX" "$RUNDIR"' EXIT
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates (portals, dconf) can reach the live desktop or the real profile.
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$RUNDIR" WAYLAND_DISPLAY=wayland-sbh-drop-gnome \
  dbus-run-session -- bash -c inner 2>"$OUT/$TAG-bus.log" | tee "$OUT/$TAG.txt"
grep -q '^RESULT PASS' "$OUT/$TAG.txt"
