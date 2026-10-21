#!/usr/bin/env bash
# S-018b AC6 on GNOME Shell (Mutter): no zwlr_layer_shell_v1, so the hive stays
# an xdg_toplevel as today, with no get_layer_surface and no protocol error.
# S-018c AC11: with SBARHIVE_DEBUG_FOCUS each show prints one
# "hive surface: window X Y W H" line, compared with the Shell.Eval frame.
# The daemon runs with WAYLAND_DEBUG=1 and shows the hive three times (no
# token, a token, drop mode); each open show is closed with Esc through
# Mutter's RemoteDesktop API (tests/drop/input_gnome.py).
# S-019 AC7: the window path prints no "hive anchor" line.
# Runs in a private headless gnome-shell with a throwaway HOME, XDG dirs,
# runtime dir and session bus. Never touches the live desktop or profile.
# Usage: tests/overlay/gnome.sh [target/release]
# Conditions go to check() in single quotes and are expanded by eval.
# shellcheck disable=SC2016,SC2034,SC2329
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
BIN="${1:-$ROOT/target/release}"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
OUT="${OVERLAY_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE SBARHIVE_SURFACE
  local log="$OUT/gnome"
  local FAILS=0
  check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }
  : >"$log-cli.log"
  # Writes the throwaway HOME's dconf through the private bus only.
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
  python3 "$ROOT/tests/drop/input_gnome.py" "$fifo" >"$log-input.log" 2>&1 &
  local input=$!
  exec 3>"$fifo"
  sleep 1
  # Mutter adds the virtual devices on the first event; plug them in now.
  echo "key Shift_L" >&3; sleep 10

  local W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/photo.png"
  local DLOG="$log-daemon.log"
  SBARHIVE_LANG=en SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$DLOG" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done

  local marks=() names=() geos=()
  show() { # show NAME COMMAND...
    local name=$1; shift
    names+=("$name"); marks+=("$(wc -l <"$DLOG")")
    "$@" >>"$log-cli.log" 2>&1
    local g=""
    for _ in $(seq 30); do g=$(hive_geo); [ -n "$g" ] && break; sleep 0.2; done
    geos+=("${g:-none}")
    echo "key Escape" >&3; sleep 1.5
  }
  show plain "$BIN/sbarhive" open "$W/photo.png"
  show token "$BIN/sbarhive" open --token sbh-test-token "$W/photo.png"
  show drop "$BIN/sbarhive" drop
  # Drop mode closes 10 s after showing without a drag entering it.
  sleep 10
  marks+=("$(wc -l <"$DLOG")")

  local R='-> '
  count() { grep -cE -- "$R$1" <<<"$2"; }
  local i
  for ((i = 0; i < ${#names[@]}; i++)); do
    local n=$((i + 1)) name=${names[i]}
    local seg upto; seg=$(sed -n "$((marks[i] + 1)),$((marks[i + 1]))p" "$DLOG")
    upto=$(head -n "${marks[i + 1]}" "$DLOG")
    echo "INFO show $n ($name): hive window ${geos[i]}"
    check "AC6 show $n ($name): the hive is a SbarHive window" '[ "${geos[i]}" != none ]'
    check "AC6 show $n ($name): the hive is an xdg_surface as today" \
      '[ "$(count "xdg_wm_base[#@][0-9]+\.get_xdg_surface\(" "$upto")" -ge 1 ]'
    check "AC6 show $n ($name): no get_layer_surface" \
      '[ "$(count "zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(" "$seg")" -eq 0 ]'
    # AC11: no Wayland client knows its own window position, so W H must be
    # the mapped frame's size within 1 px and X Y only have to lie on screen.
    local sl ns kind sx sy sw sh gx gy gw gh
    sl=$(grep -E '^hive surface: ' <<<"$seg"); ns=$(grep -c . <<<"$sl")
    read -r _ _ kind sx sy sw sh <<<"$(head -1 <<<"$sl")"
    read -r gx gy gw gh <<<"${geos[i]}"
    echo "INFO show $n ($name): ${ns} surface line(s): ${sl:-none}"
    check "AC11 show $n ($name): exactly one \"hive surface: window X Y W H\" line" \
      '[ "$ns" -eq 1 ] && grep -qE "^hive surface: window -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$" <<<"$sl"'
    check "AC11 show $n ($name): W H equal the SbarHive frame within 1 px, X Y on the 1280x800 screen" \
      '[ "${kind:-}" = window ] && [ -n "${gh:-}" ] && [ $((sw - gw)) -ge -1 ] && [ $((sw - gw)) -le 1 ] && [ $((sh - gh)) -ge -1 ] && [ $((sh - gh)) -le 1 ] && [ "$sx" -ge 0 ] && [ "$sy" -ge 0 ] && [ $((sx + sw)) -le 1280 ] && [ $((sy + sh)) -le 800 ]'
  done
  # S-019 AC7 (S-019c): the window path has no anchor; the hive stays
  # where it is today. A guard: it holds before and after S-019c.
  check "S-019 AC7: the window path prints no \"hive anchor\" line" '! grep -qE "^hive anchor" "$DLOG"'
  check "AC6: no get_layer_surface in the whole trace" \
    '[ "$(count "zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(" "$(cat "$DLOG")")" -eq 0 ]'
  check "AC6: no protocol error in the daemon log" '! grep -qE "wl_display[#@]1\.error\(|[Pp]rotocol error" "$DLOG"'

  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi

  echo quit >&3; exec 3>&-
  "$BIN/sbarhive" quit >/dev/null 2>&1
  sleep 0.5
  kill $d $input 2>/dev/null
  kill $shell 2>/dev/null
  timeout 5 tail --pid=$shell -f /dev/null || kill -9 $shell
}
export -f inner

echo "== gnome"
SANDBOX="$(mktemp -d)"
RUNTIME="$(mktemp -d)"
chmod 700 "$RUNTIME"
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates (portals, dconf) can reach the live desktop or the real profile.
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY=wayland-sbh-overlay-gnome \
  dbus-run-session -- bash -c inner 2>"$OUT/gnome-bus.log" | tee "$OUT/gnome.txt"
rm -rf "$SANDBOX" "$RUNTIME"
grep -q '^RESULT PASS' "$OUT/gnome.txt"
