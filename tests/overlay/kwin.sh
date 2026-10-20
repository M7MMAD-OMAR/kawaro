#!/usr/bin/env bash
# S-018b: the hive as a zwlr_layer_shell_v1 overlay on KWin (AC4, AC5), and
# the window fallback when forced (AC6). The daemon runs with WAYLAND_DEBUG=1
# over a full screen GTK window (other.py) and shows the hive five times: no
# token, a token, drop mode, after a language change, after a theme change.
# Each show is closed with Esc through org_kde_kwin_fake_input. The trace is
# cut per show and the layer requests counted; a ScreenShot2 frame before and
# during the first show gives the box of the hive's pixels.
# Runs in a private kwin_wayland --virtual with a throwaway HOME, XDG dirs,
# runtime dir and session bus. Never touches the live desktop or profile.
# Usage: tests/overlay/kwin.sh [window] [target/release]
#   window  runs the daemon with SBARHIVE_SURFACE=window (AC6)
# Conditions go to check() in single quotes and are expanded by eval.
# shellcheck disable=SC2016,SC2034,SC2329
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
MODE=layer
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in window) MODE=window ;; *) BIN="$a" ;; esac
done
INPUT="$ROOT/tests/drop/kwin-input/target/release/kwin-input"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
[ -x "$INPUT" ] || { echo "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"; exit 2; }
OUT="${OVERLAY_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT MODE INPUT

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE SBARHIVE_SURFACE
  local log="$OUT/kwin-$MODE"
  local FAILS=0
  check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }
  : >"$log-cli.log"
  mkdir -p "$XDG_CONFIG_HOME/sbarhive"
  printf '[Windows]\nFocusStealingPreventionLevel=1\n' >"$XDG_CONFIG_HOME/kwinrc"
  # English and light to start with; shows 4 and 5 change them.
  printf 'language = "en"\ntheme = "light"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"
  # Lets shot.py (python3) use org.kde.KWin.ScreenShot2 in this private KWin only.
  mkdir -p "$XDG_DATA_HOME/applications"
  printf '[Desktop Entry]\nType=Application\nName=sbh shot\nExec=%s\nNoDisplay=true\nX-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2\n' \
    "$(readlink -f "$(command -v python3)")" >"$XDG_DATA_HOME/applications/sbh-shot.desktop"
  kbuildsycoca6 --noincremental >/dev/null 2>&1
  # KWIN_WAYLAND_NO_PERMISSION_CHECKS lets the test's own input client and
  # shot.py use fake input and ScreenShot2 in this private compositor only.
  KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 \
    kwin_wayland --virtual --no-lockscreen --socket "$WAYLAND_DISPLAY" --width 1280 --height 800 >"$log-kwin.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "kwin did not start"; tail "$log-kwin.log"; return 1; }
  sleep 2

  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  "$INPUT" <"$f" >"$log-input.log" 2>&1 &
  exec 3>"$f"
  GTK_A11Y=none python3 "$HERE/other.py" "$XDG_RUNTIME_DIR/token" >"$log-other.log" 2>&1 &
  local other=$!
  sleep 2
  echo "move 200 200" >&3; sleep 0.5

  local W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/photo.png"
  local DLOG="$log-daemon.log"
  local env=()
  [ "$MODE" = window ] && env=(SBARHIVE_SURFACE=window)
  env "${env[@]}" WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$DLOG" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done

  shot() { timeout 10 python3 "$HERE/shot.py" "$1" >/dev/null 2>>"$log-shot.err"; }
  shot "$log-before.png"

  local marks=() names=()
  esc() { echo "keydown 1" >&3; echo "sleep 40" >&3; echo "keyup 1" >&3; }
  show() { # show NAME COMMAND...: one show, closed with Esc
    local name=$1; shift
    names+=("$name"); marks+=("$(wc -l <"$DLOG")")
    "$@" >>"$log-cli.log" 2>&1
    sleep 1.5
    [ "$name" = plain ] && shot "$log-shown.png"
    esc; sleep 1
  }
  show plain "$BIN/sbarhive" open "$W/photo.png"
  show token "$BIN/sbarhive" open --token sbh-test-token "$W/photo.png"
  show drop "$BIN/sbarhive" drop
  # Drop mode may have no keyboard (S-018d): it closes 10 s after showing
  # without a drag entering it (S-014a AC3).
  sleep 10
  printf 'language = "ar"\ntheme = "light"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"; sleep 3
  show language "$BIN/sbarhive" open "$W/photo.png"
  printf 'language = "ar"\ntheme = "dark"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"; sleep 3
  show theme "$BIN/sbarhive" open "$W/photo.png"
  marks+=("$(wc -l <"$DLOG")")

  # Requests print as "-> interface#id.request(...)" (wayland-rs) or with "@"
  # (libwayland). Events print without the arrow.
  local R='-> '
  count() { grep -cE -- "$R$1" <<<"$2"; }
  local trace; trace=$(cat "$DLOG")
  local xdg_all layer_all
  xdg_all=$(count 'xdg_wm_base[#@][0-9]+\.get_xdg_surface\(' "$trace")
  layer_all=$(count 'zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$trace")
  echo "INFO whole trace: get_xdg_surface $xdg_all, get_layer_surface $layer_all"

  local i
  for ((i = 0; i < ${#names[@]}; i++)); do
    local n=$((i + 1)) name=${names[i]}
    local seg; seg=$(sed -n "$((marks[i] + 1)),$((marks[i + 1]))p" "$DLOG")
    local gl gl3 anc anc_bad ez ez_bad kb kb_bad sub xs
    gl=$(count 'zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$seg")
    gl3=$(count 'zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(.*, 3, "sbarhive"\)' "$seg")
    anc=$(count 'zwlr_layer_surface_v1[#@][0-9]+\.set_anchor\(15\)' "$seg")
    anc_bad=$(count 'zwlr_layer_surface_v1[#@][0-9]+\.set_anchor\([^1]|zwlr_layer_surface_v1[#@][0-9]+\.set_anchor\(1[^5]' "$seg")
    ez=$(count 'zwlr_layer_surface_v1[#@][0-9]+\.set_exclusive_zone\(-1\)' "$seg")
    ez_bad=$(count 'zwlr_layer_surface_v1[#@][0-9]+\.set_exclusive_zone\([^-]' "$seg")
    kb=$(count 'zwlr_layer_surface_v1[#@][0-9]+\.set_keyboard_interactivity\(1\)' "$seg")
    kb_bad=$(count 'zwlr_layer_surface_v1[#@][0-9]+\.set_keyboard_interactivity\([^1]' "$seg")
    sub=$(count 'wl_subcompositor[#@][0-9]+\.get_subsurface\(' "$seg")
    xs=$(count 'xdg_wm_base[#@][0-9]+\.get_xdg_surface\(' "$seg")
    echo "INFO show $n ($name): get_layer_surface $gl (layer 3 \"sbarhive\" $gl3), set_anchor(15) $anc, set_exclusive_zone(-1) $ez, set_keyboard_interactivity(1) $kb, get_subsurface $sub, get_xdg_surface $xs"
    if [ "$MODE" = layer ]; then
      check "AC5 show $n ($name): exactly one get_layer_surface" '[ "$gl" -eq 1 ]'
      check "AC4 show $n ($name): the layer surface is on layer 3 (overlay) with namespace sbarhive" '[ "$gl3" -eq 1 ]'
      check "AC4 show $n ($name): anchors 15 (all edges)" '[ "$anc" -ge 1 ] && [ "$anc_bad" -eq 0 ]'
      check "AC4 show $n ($name): exclusive_zone -1" '[ "$ez" -ge 1 ] && [ "$ez_bad" -eq 0 ]'
      # Drop mode gives the catcher no keyboard (AC13, S-018d), so only the
      # four other shows are held to keyboard 1 here.
      [ "$name" != drop ] && check "AC4 show $n ($name): keyboard_interactivity 1 (exclusive)" '[ "$kb" -ge 1 ] && [ "$kb_bad" -eq 0 ]'
      check "AC4 show $n ($name): exactly one get_subsurface (the hive in the catcher)" '[ "$sub" -eq 1 ]'
      check "AC5 show $n ($name): no get_xdg_surface" '[ "$xs" -eq 0 ]'
    else
      check "AC6 show $n ($name): no get_layer_surface with SBARHIVE_SURFACE=window" '[ "$gl" -eq 0 ]'
      # The first show can reuse the toplevel winit made when the daemon
      # started (tests/drop/lib.sh), so search up to the end of this show.
      local upto; upto=$(head -n "${marks[i + 1]}" "$DLOG")
      check "AC6 show $n ($name): the hive is an xdg_surface as today" '[ "$(count "xdg_wm_base[#@][0-9]+\.get_xdg_surface\(" "$upto")" -ge 1 ]'
    fi
  done
  if [ "$MODE" = layer ]; then
    check "AC4/AC5: no get_xdg_surface in the whole trace (no hive toplevel, not even a hidden one)" '[ "$xdg_all" -eq 0 ]'
  else
    check "AC6: no get_layer_surface in the whole trace" '[ "$layer_all" -eq 0 ]'
    check "AC6: get_xdg_surface in the trace" '[ "$xdg_all" -ge 1 ]'
  fi
  check "AC$([ "$MODE" = layer ] && echo 4 || echo 6): no protocol error in the daemon log" \
    '! grep -qE "wl_display[#@]1\.error\(|[Pp]rotocol error" "$DLOG"'

  # The box of the pixels the first show changed: the hive's opaque pixels,
  # centred within 1 px on 1280x800 (640, 400), no larger than the hive's
  # 393 x 362 (S-018 AC2), so a title bar or an opaque catcher fails it.
  local box="none"
  if [ -s "$log-before.png" ] && [ -s "$log-shown.png" ]; then
    box=$(python3 -I "$HERE/boxdiff.py" "$log-before.png" "$log-shown.png" 8 2>&1)
  fi
  echo "INFO hive pixel box: $box"
  local bx by bw bh; read -r bx by bw bh <<<"$box"
  if [ "$MODE" = layer ]; then
    check "AC4: screenshot box centred within 1 px" \
      '[ -n "${bh:-}" ] && python3 -I -c "import sys; x,y,w,h=map(int,sys.argv[1:]); sys.exit(not (abs(x+w/2-640)<=1 and abs(y+h/2-400)<=1))" "$bx" "$by" "$bw" "$bh"'
    check "AC4: screenshot box no larger than the hive (no title bar, transparent catcher)" \
      '[ -n "${bh:-}" ] && [ "$bw" -ge 300 ] && [ "$bw" -le 394 ] && [ "$bh" -ge 280 ] && [ "$bh" -le 363 ]'
  fi

  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi

  exec 3>&-
  "$BIN/sbarhive" quit >/dev/null 2>&1
  sleep 0.5
  kill $other $d 2>/dev/null
  kill $kwin 2>/dev/null
  timeout 5 tail --pid=$kwin -f /dev/null || kill -9 $kwin
}
export -f inner

echo "== kwin $MODE"
SANDBOX="$(mktemp -d)"
RUNTIME="$(mktemp -d)"
chmod 700 "$RUNTIME"
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates can reach the live desktop or the real profile.
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY=wayland-sbh-overlay-kwin \
  dbus-run-session -- bash -c inner 2>"$OUT/kwin-$MODE-bus.log" | tee "$OUT/kwin-$MODE.txt"
rm -rf "$SANDBOX" "$RUNTIME"
grep -q '^RESULT PASS' "$OUT/kwin-$MODE.txt"
