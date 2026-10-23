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
# S-019f (AC14, AC15): show 6 drags thumbed.png, whose thumbnail is seeded
# in the sandbox's XDG_CACHE_HOME (tests/overlay/thumbs.py), onto the
# centre cell; every show's "hive preview" lines are checked by
# preview_checks (lib.sh): "icon image" for the PNGs, "icon pdf" for show 5,
# "thumb W H" for show 6, one per enter and none before it. The pixels of
# the preview are checked on KWin only (tests/drop/kwin.sh, show 13).
# S-014e2 (AC18 part c, AC20 for GNOME, AC21), tests/drop/portal_real.sh:
#   mode "portal": the fake GlobalShortcuts portal (shortcuts_portal.py ok)
#   on the private bus, the installed portal still blocked; a PNG drag, the
#   fake's Activated, the hive a centred window with the waiting text, the
#   drop on WEBP before Deactivated.
#   mode "portal-real": the installed xdg-desktop-portal with
#   xdg-desktop-portal-gnome on this private bus only, XDG_CURRENT_DESKTOP
#   GNOME; the backend's dialog accepted by an injected click; Super+Z held
#   through RemoteDesktop: AC20 (a), (b), (c), then a second daemon on the
#   same profile (AC21).
# S-014f3 (AC15), tests/drop/token.sh and tests/drop/portal_real.sh:
#   "smart portal" also runs the five fake token cases of AC15 (a) on the
#   window path; "strict portal-real" (focus-new-windows strict) runs AC15
#   (b), three Super+Z held drags and one with hold_ms 2000, each checked
#   for the portal's token in the trace and the daemon's focus window, and
#   the no-token control (c), instead of AC20 and AC21.
# Usage: tests/drop/gnome.sh [smart|strict] [shortcut|portal|portal-real] [target/release]
#   Mutter focus-new-windows, default smart
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
# shellcheck source=../no_portal.sh
source "$ROOT/tests/no_portal.sh"
POLICY=smart
MODE=""
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in smart|strict) POLICY="$a" ;; shortcut | portal | portal-real) MODE="$a" ;; *) BIN="$a" ;; esac
done
TAG="gnome-$POLICY${MODE:+-$MODE}"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
OUT="${DROP_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT POLICY MODE TAG

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  # S-014e2: the daemon sees the desktop the real portal was started for.
  [ "$MODE" = portal-real ] && export XDG_CURRENT_DESKTOP=GNOME XDG_SESSION_TYPE=wayland
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
  THUMBED="$W/thumbed.png"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$THUMBED"
  python3 -I "$ROOT/tests/overlay/thumbs.py" "$THUMBED" "$XDG_CACHE_HOME" large 128 128 photo >"$log-thumb.log" 2>&1 \
    || echo "INFO S-019 AC15: thumbs.py failed: $(cat "$log-thumb.log")"
  NEXT="$XDG_RUNTIME_DIR/next"; printf '%s\n' "$W/photo one.png" >"$NEXT"
  DLOG="$log-daemon.log"; SLOG="$log-source.log"

  local d src=""
  if [ "$MODE" = portal ] || [ "$MODE" = portal-real ]; then
    # S-014e2 hooks for tests/drop/portal_real.sh.
    key_down() { echo "keydown Super_L" >&3; sleep 0.05; echo "keydown z" >&3; }
    key_up() { echo "keyup z" >&3; sleep 0.05; echo "keyup Super_L" >&3; }
    hold_motion() { :; }
    # shell_str JS: the string JS evaluates to, decoded from Shell.Eval's
    # reply (a GVariant tuple holding the JSON of the result).
    shell_str() {
      shell_eval "$1" | python3 -I -c 'import ast, json, sys
t = sys.stdin.read().strip().replace("(true,", "(True,", 1).replace("(false,", "(False,", 1)
try:
    ok, v = ast.literal_eval(t)
except (ValueError, SyntaxError):
    sys.exit()
if ok and v:
    print(json.loads(v))'
    }
    dialog_now() {
      shell_str "global.get_window_actors().map(a => a.meta_window)
        .filter(w => w.get_pid() !== $d && w.get_pid() !== ${src:-0} && !w.minimized && w.showing_on_its_workspace())
        .map(w => { const r = w.get_frame_rect(); return [r.x, r.y, r.width, r.height].join(' ') + '|' + w.get_pid() + '|' + w.get_wm_class() + '|' + w.get_title(); })
        .filter(s => !s.startsWith('0 0 0 0'))[0] || ''" | grep .
    }
    dialog_raise() {
      shell_eval "global.get_window_actors().map(a => a.meta_window).filter(w => w.get_pid() === $1)
        .forEach(w => w.activate(global.get_current_time())); true" >/dev/null
    }
    accept_click() { echo "to $(($1 + $3 - 39)) $(($2 + 22))" >&3; sleep 0.5; echo down >&3; sleep 0.1; echo up >&3; }
    accept_key() { echo "key Return" >&3; }
    watch_start() {
      shell_eval "global._sbhAdded = []; global.display.connect('window-created', (dp, w) => global._sbhAdded.push([w.get_pid(), w])); true" >/dev/null
    }
    watch_read() {
      shell_str "global._sbhAdded.map(([p, w]) => { let c = '', t = ''; try { c = w.get_wm_class(); t = w.get_title(); } catch (e) {} return p + '|' + c + '|' + t; }).join(String.fromCharCode(10))" | grep .
    }
    hive_shows() { grep -cE '^hive surface: window ' <<<"$1"; }
    # S-014f3: the PID of the window that has the keyboard focus.
    focus_pid() {
      shell_str "(() => { const w = global.display.focus_window; return w ? String(w.get_pid()) : 'none'; })()" | grep . || echo none
    }
    token_motion() { :; }
    token_settle() { wait_gone 12 || true; sleep 1; }
    screen_middle() {
      shell_str "(() => { const r = global.workspace_manager.get_active_workspace().get_work_area_for_monitor(0); return Math.round(r.x + r.width / 2) + ' ' + Math.round(r.y + r.height / 2); })()"
    }
    screen_shot() {
      gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell/Screenshot \
        --method org.gnome.Shell.Screenshot.Screenshot false false "$1" >/dev/null 2>&1
    }
    # shellcheck source=portal_real.sh
    source "$HERE/portal_real.sh"
    portal_setup
    portal_start_daemon first
  else
  # English layout, so the WEBP cell is top right of the centre. No warm-up
  # show: the first drop-mode show is part of AC5.
  SBARHIVE_LANG=en SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$DLOG" 2>&1 &
  d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
  fi

  GTK_A11Y=none python3 "$HERE/source.py" "$NEXT" >"$SLOG" 2>&1 &
  src=$!
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
  elif [ "$MODE" = portal ]; then
    fake_window_case
    # S-014f3, AC15 (a) on the window path.
    # shellcheck source=token.sh
    source "$HERE/token.sh"
    token_cases GNOME
    portal_teardown
  elif [ "$MODE" = portal-real ]; then
    if [ "$POLICY" = strict ]; then
      ac15_real_run
    else
      real_portal_run GNOME xdg-desktop-portal-gnome
    fi
    portal_teardown
  else
  cycle 1 "$W/photo one.png" webp 0
  cycle 2 "$W/photo two.png" corner 0
  cycle 3 "$W/photo three.png" webp 1
  cycle 4 "$W/photo four.png" centre 0
  cycle 5 "$W/report five.pdf" corner 0
  cycle 6 "$THUMBED" centre 0
  protocol_checks
  enter_cells_checks
  preview_checks
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
DESK=()
if [ "$MODE" = portal-real ]; then
  # S-014e2: the installed portal on this private bus, for GNOME.
  DESK=(XDG_CURRENT_DESKTOP=GNOME XDG_SESSION_TYPE=wayland)
else
  no_real_portal "$SANDBOX"
fi
if [ "$MODE" = portal ] || [ "$MODE" = portal-real ]; then
  # The daemon registers (no portal_shortcut = false), hold_ms 400, and the
  # shipped .desktop file is installed in the throwaway data directory.
  mkdir -p "$SANDBOX/.config/sbarhive" "$SANDBOX/.local/share/applications"
  printf 'hold_ms = 400\n' >"$SANDBOX/.config/sbarhive/config.toml"
  cp "$ROOT/packaging/io.github.m7mmad_omar.SbarHive.desktop" "$SANDBOX/.local/share/applications/"
fi
RUNDIR="$(mktemp -d)"
trap 'fusermount3 -u "$RUNDIR/doc" 2>/dev/null; rm -rf "$SANDBOX" "$RUNDIR"' EXIT
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates (portals, dconf) can reach the live desktop or the real profile.
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP "${DESK[@]}" \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$RUNDIR" WAYLAND_DISPLAY=wayland-sbh-drop-gnome \
  dbus-run-session -- bash -c inner 2>"$OUT/$TAG-bus.log" | tee "$OUT/$TAG.txt"
grep -q '^RESULT PASS' "$OUT/$TAG.txt"
