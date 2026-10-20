#!/usr/bin/env bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034
# S-014b drop receiver on KWin (AC5, AC6, AC7), on the layer path since
# S-018d (S-018 AC13, AC14). A GTK 4 window drags a PNG,
# `sbarhive drop` is run mid-drag (what the shortcut does), and the file is
# dropped: show 1 on the WEBP cell, show 2 on a transparent corner, show 3 on
# the WEBP cell with Shift held, show 4 on the centre cell. The daemon runs
# with WAYLAND_DEBUG=1 so the trace shows a wl_data_device per new surface.
# S-014c (AC8, AC9, AC10): show 5 drags a PDF and drops it on a corner; every
# show's cells at enter must equal `sbarhive actions` for its file. In show 1
# ScreenShot2 shots are taken while the hive waits for the drag (the centre
# must show the waiting text) and while the drag is over the WEBP cell (it
# must be highlighted with color.light.accent, a cell across the centre not),
# and the hive's box of each is compared with the owner approved references
# in tests/drop/snapshots/ (drop-en-light-waiting.png, drop-en-light-over.png).
# Candidates are always written to the output folder; a missing reference is
# a FAIL until the owner approves one and it is copied there. The daemon runs
# light (SBARHIVE_COLOR_SCHEME) with reduced motion (SBARHIVE_REDUCED_MOTION),
# so the highlight is drawn at once and the shots are stable.
# Runs in a private kwin_wayland --virtual with a throwaway HOME, XDG dirs,
# runtime dir and session bus. Never touches the live desktop or profile.
# S-014d (AC11), mode "shortcut": Super+Z is bound the way the KDE docs tell
# users to (System Settings, Shortcuts, Add New, Command: a .desktop entry in
# XDG_DATA_HOME/applications and a [services] entry in kglobalshortcutsrc,
# launched by the kglobalacceld inside this private KWin) to a wrapper that
# marks the time and execs `sbarhive drop`. Three drags each press Super+Z
# mid-drag and drop on the WEBP cell, then Esc is pressed mid-drag over the
# hive and before the hive (tests/drop/lib.sh, shortcut_shows). The five
# S-014b and S-014c shows are not run in this mode.
# Usage: tests/drop/kwin.sh [level=N] [shortcut] [target/release]
#   level: focus stealing prevention 0..4, default 1 (Low)
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
LEVEL=1
MODE=""
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in level=*) LEVEL="${a#level=}" ;; shortcut) MODE=shortcut ;; *) BIN="$a" ;; esac
done
TAG="kwin-l$LEVEL${MODE:+-$MODE}"
INPUT="$HERE/kwin-input/target/release/kwin-input"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
[ -x "$INPUT" ] || { echo "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"; exit 2; }
OUT="${DROP_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT LEVEL INPUT MODE TAG

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  # shellcheck source=lib.sh
  source "$HERE/lib.sh"
  OUT_LOG="$OUT/$TAG"
  local log="$OUT_LOG"
  : >"$log-cli.log"
  mkdir -p "$XDG_CONFIG_HOME"
  # KWin's scale effect fades new surfaces in over 200 ms; the snapshots
  # must show the hive as drawn, not a frame of that fade.
  printf '[Windows]\nFocusStealingPreventionLevel=%s\n\n[Plugins]\nscaleEnabled=false\n' "$LEVEL" >"$XDG_CONFIG_HOME/kwinrc"
  # Lets shot.py (python3) use org.kde.KWin.ScreenShot2 in this private KWin
  # only, as tests/overlay/kwin.sh does.
  mkdir -p "$XDG_DATA_HOME/applications"
  printf '[Desktop Entry]\nType=Application\nName=sbh shot\nExec=%s\nNoDisplay=true\nX-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2\n' \
    "$(readlink -f "$(command -v python3)")" >"$XDG_DATA_HOME/applications/sbh-shot.desktop"
  # S-014d AC11: a KDE custom command shortcut, as the docs describe it.
  SHORTCUT_MARKS="$XDG_RUNTIME_DIR/shortcut-marks"
  SHORTCUT_WRAP="$XDG_RUNTIME_DIR/sbh-drop"
  if [ "$MODE" = shortcut ]; then
    printf '#!/bin/sh\ndate -u +%%T.%%6N >>"%s"\nexec "%s" drop >>"%s" 2>&1\n' \
      "$SHORTCUT_MARKS" "$BIN/sbarhive" "$log-cli.log" >"$SHORTCUT_WRAP"
    chmod +x "$SHORTCUT_WRAP"
    printf '[Desktop Entry]\nExec=%s\nName=SbarHive drop\nNoDisplay=true\nStartupNotify=false\nType=Application\nX-KDE-GlobalAccel-CommandShortcut=true\n' \
      "$SHORTCUT_WRAP" >"$XDG_DATA_HOME/applications/net.local.sbarhive-drop.desktop"
    printf '[services][net.local.sbarhive-drop.desktop]\n_launch=Meta+Z\n' >"$XDG_CONFIG_HOME/kglobalshortcutsrc"
  fi
  kbuildsycoca6 --noincremental >/dev/null 2>&1
  # KWIN_WAYLAND_NO_PERMISSION_CHECKS lets the test's own input client use
  # org_kde_kwin_fake_input in this private compositor only.
  KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 QT_FORCE_STDERR_LOGGING=1 \
    QT_LOGGING_RULES='kwin_scripting.debug=true;js.debug=true' \
    kwin_wayland --virtual --no-lockscreen --socket "$WAYLAND_DISPLAY" --width 1280 --height 800 >"$log-kwin.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "kwin did not start"; tail "$log-kwin.log"; return 1; }
  sleep 2

  # S-018d: on KWin the hive is a layer surface (S-018 AC4), and the layer
  # window KWin lists is the full screen catcher, not the hive. The hive's
  # rectangle comes from the daemon's "hive surface: layer X Y W H" line
  # (S-018 AC11, printed once per show with SBARHIVE_DEBUG_FOCUS), and the
  # hive is shown while there are more such lines than layer surface
  # destroys in the trace.
  hive_geo() {
    local shown gone
    shown=$(grep -cE '^hive surface: layer ' "$DLOG")
    gone=$(grep -cE -- '-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\(' "$DLOG")
    [ "$shown" -gt "$gone" ] || return 0
    grep -E '^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$' "$DLOG" | tail -1 | cut -d' ' -f4-
  }

  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  "$INPUT" <"$f" >"$log-input.log" 2>&1 &
  exec 3>"$f"
  ptr_move() { echo "move $1 $2" >&3; }
  ptr_down() { echo "down" >&3; }
  ptr_up() { echo "up" >&3; }
  shift_down() { echo "keydown 42" >&3; }
  shift_up() { echo "keyup 42" >&3; }

  W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  local i
  for i in one two three four; do cp "$ROOT/tests/fixtures/images/alpha.png" "$W/photo $i.png"; done
  cp "$ROOT/tests/fixtures/docs/one-page.pdf" "$W/report five.pdf"
  NEXT="$XDG_RUNTIME_DIR/next"; printf '%s\n' "$W/photo one.png" >"$NEXT"
  DLOG="$log-daemon.log"; SLOG="$log-source.log"

  # English layout, so the WEBP cell is top right of the centre. No warm-up
  # show: the first drop-mode show is part of AC5.
  SBARHIVE_LANG=en SBARHIVE_COLOR_SCHEME=light SBARHIVE_REDUCED_MOTION=1 \
    SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$DLOG" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done

  GTK_A11Y=none python3 "$HERE/source.py" "$NEXT" >"$SLOG" 2>&1 &
  local src=$!
  sleep 3

  # S-014c AC9, AC10: screenshots in show 1 (the PNG, English, light).
  local tokens="$ROOT/design/tokens.json" px="$HERE/pixels.py"
  # tests/overlay/shot.py captures at its D-Bus call but writes the PNG only
  # after a 5 s join on its reader thread (and can then hang on exit), which
  # would run into the 10 s enter timeout (AC3). So each shot starts in the
  # background, the drag waits 1.5 s with the hive unchanged, and the pixels
  # are checked by shot_checks after the shows.
  SHOTS=()
  shoot() { # shoot NAME X Y W H
    local full="$OUT_LOG-shot-$1.png"
    rm -f "$full" "$OUT/drop-en-light-$1.png"
    timeout 10 python3 -I -W ignore::DeprecationWarning "$ROOT/tests/overlay/shot.py" "$full" >>"$log-shot.log" 2>&1 &
    SHOTS+=("$!")
    sleep 1.5
  }
  snap_check() { # snap_check AC NAME
    local ref="$HERE/snapshots/drop-en-light-$2.png" cand="$OUT/drop-en-light-$2.png" d
    if [ ! -f "$cand" ]; then
      echo "FAIL $1: drop-en-light-$2.png: no screenshot taken"; FAILS=$((FAILS + 1))
    elif [ ! -f "$ref" ]; then
      echo "FAIL $1: no approved reference $ref (candidate: $cand, needs the owner's approval)"; FAILS=$((FAILS + 1))
    else
      d=$(python3 -I "$px" diff "$cand" "$ref")
      echo "INFO $1 drop-en-light-$2.png differs from the reference in $d of its pixels"
      check "$1: drop-en-light-$2.png matches the approved reference (0.1 % or less)" \
        'python3 -I -c "import sys; sys.exit(0 if float(sys.argv[1]) <= 0.001 else 1)" "$d" 2>/dev/null'
    fi
  }
  count() { python3 -I "$px" count "$1" "$2" "$3" "$4" "$5" "$tokens" light "$6" "${7:-8}"; }
  local geo1=""
  on_shown() {
    [ "$1" = 1 ] || return 0
    geo1="$2 $3 $4 $5"
    shoot waiting "$2" "$3" "$4" "$5"
  }
  on_over() {
    [ "$1" = 1 ] || return 0
    shoot over "$2" "$3" "$4" "$5"
  }
  shot_checks() {
    local p; for p in "${SHOTS[@]}"; do wait "$p"; done
    local gx gy gw gh; read -r gx gy gw gh <<<"${geo1:-0 0 0 0}"
    local cx=$((gx + gw / 2)) cy=$((gy + gh / 2)) dx dy ink hot cold cold2 n
    for n in waiting over; do
      [ -s "$OUT_LOG-shot-$n.png" ] && [ "$gw" -gt 0 ] &&
        python3 -I "$px" crop "$OUT_LOG-shot-$n.png" "$gx" "$gy" "$gw" "$gh" "$OUT/drop-en-light-$n.png"
    done
    read -r dx dy <<<"$(webp_offset)"
    # AC10: the waiting text, in the centre hexagon's middle (72 x 48 around
    # the hive's centre), before the drag enters. Guard first: the cells
    # arrive on enter, so a shot taken before it has no surface-2 or accent
    # in the WEBP cell's box. A late shot would show the selection text that
    # enter sets, and must not pass as the waiting text.
    local w="$OUT_LOG-shot-waiting.png" early="" late2="" late3=""
    if [ -s "$w" ]; then
      late2=$(count "$w" $((cx + dx - 28)) $((cy + dy - 28)) 56 56 surface-2)
      late3=$(count "$w" $((cx + dx - 28)) $((cy + dy - 28)) 56 56 accent)
      [ "${late2:-1}" -le 10 ] && [ "${late3:-1}" -le 10 ] && early=1
    fi
    echo "INFO AC10 show 1: WEBP box in the waiting shot: surface-2 ${late2:-none}, accent ${late3:-none}"
    check "AC10 show 1: the waiting shot was taken before enter (no cells yet)" '[ "$early" = 1 ]'
    [ "$early" = 1 ] && ink=$(count "$w" $((cx - 36)) $((cy - 24)) 72 48 text-primary 40)
    echo "INFO AC10 show 1: ${ink:-no} text-primary pixels in the centre before the drag enters"
    check "AC10 show 1: the waiting hive's centre shows text (text-primary ink) before enter" '[ "${ink:-0}" -ge 15 ]'
    snap_check AC10 waiting
    # AC9: 56 x 56 boxes around the WEBP cell (under the drag) and around the
    # ring 1 cell across the centre from it, axial (-1, 1).
    local o="$OUT_LOG-shot-over.png"
    if [ -s "$o" ]; then
      hot=$(count "$o" $((cx + dx - 28)) $((cy + dy - 28)) 56 56 accent)
      cold=$(count "$o" $((cx - dx - 28)) $((cy - dy - 28)) 56 56 accent)
      cold2=$(count "$o" $((cx - dx - 28)) $((cy - dy - 28)) 56 56 surface-2)
    fi
    echo "INFO AC9 show 1: accent pixels under the drag ${hot:-none}; across the centre accent ${cold:-none}, surface-2 ${cold2:-none}"
    check "AC9 show 1: the cell under the drag is filled with color.light.accent" '[ "${hot:-0}" -ge 200 ]'
    check "AC9 show 1: a cell not under the drag is not highlighted (surface-2, no accent)" \
      '[ -n "$cold" ] && [ "$cold" -le 10 ] && [ "${cold2:-0}" -ge 200 ]'
    snap_check AC9 over
  }

  if [ "$MODE" = shortcut ]; then
    # Meta (125) + Z (44) and Esc (1), evdev codes, through fake_input.
    press_shortcut() { echo "keydown 125" >&3; echo "keydown 44" >&3; echo "sleep 50" >&3; echo "keyup 44" >&3; echo "keyup 125" >&3; }
    press_esc() { echo "keydown 1" >&3; echo "sleep 30" >&3; echo "keyup 1" >&3; }
    HIVE_DESTROY_RE='-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\('
    ESC_NO_KEYBOARD=1
    local comps; comps=$(gdbus call --session --dest org.kde.kglobalaccel --object-path /kglobalaccel \
      --method org.kde.KGlobalAccel.allComponents 2>&1)
    check "AC11 setup: kglobalacceld lists the custom command shortcut component" 'grep -q sbarhive_drop <<<"$comps"'
    shortcut_shows
    summary
  else
  cycle 1 "$W/photo one.png" webp 0
  cycle 2 "$W/photo two.png" corner 0
  cycle 3 "$W/photo three.png" webp 1
  cycle 4 "$W/photo four.png" centre 0
  cycle 5 "$W/report five.pdf" corner 0
  protocol_checks
  enter_cells_checks
  shot_checks
  # S-018d (AC13, AC14): the layer path's drop checks, per show.
  # shellcheck source=layer.sh
  source "$HERE/layer.sh"
  for ((i = 0; i + 1 < ${#MARKS[@]}; i++)); do
    layer_drop_show $((i + 1)) "${MARKS[i]}" "${MARKS[i + 1]}" 1
  done
  summary
  fi

  exec 3>&-
  "$BIN/sbarhive" quit >/dev/null 2>&1
  sleep 0.5
  kill $src $d 2>/dev/null
  kill $kwin 2>/dev/null
  timeout 5 tail --pid=$kwin -f /dev/null || kill -9 $kwin
}
export -f inner

echo "== kwin level $LEVEL${MODE:+ $MODE}"
SANDBOX="$(mktemp -d)"
RUNDIR="$(mktemp -d)"
trap 'rm -rf "$SANDBOX" "$RUNDIR"' EXIT
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates can reach the live desktop or the real profile.
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$RUNDIR" WAYLAND_DISPLAY=wayland-sbh-drop-kwin \
  dbus-run-session -- bash -c inner 2>"$OUT/$TAG-bus.log" | tee "$OUT/$TAG.txt"
grep -q '^RESULT PASS' "$OUT/$TAG.txt"
