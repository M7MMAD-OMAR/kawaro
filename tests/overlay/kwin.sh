#!/usr/bin/env bash
# S-018b and S-018c on KWin: the hive as a zwlr_layer_shell_v1 overlay.
# The daemon runs with WAYLAND_DEBUG=1 and SBARHIVE_DEBUG_FOCUS=1 over a full
# screen GTK window (other.py); input comes from org_kde_kwin_fake_input
# (tests/drop/kwin-input). The trace is cut per show with line marks.
# Modes (one private compositor each):
#   layer (default)  five shows: no token, a token, drop mode, after a language
#                    change, after a theme change; each closed with Esc. AC4,
#                    AC5 (layer requests per show), AC11 (diagnostic lines
#                    against the trace and a ScreenShot2 box).
#   window           the same shows with SBARHIVE_SURFACE=window (AC6, AC11),
#                    plus a click on the GTK window: focus loss still closes
#                    the hive on the window path (AC9).
#   input            AC8 and AC9, KWin focus follows mouse at level Low: a
#                    click on the ring-1 cell right of the centre, a click on
#                    the catcher at (100, 700), Right then Enter with the
#                    pointer over the GTK window, Esc after the pointer roams
#                    over 5 points, and `sbarhive settings` while shown.
#   token            AC10: three shows with `open --token T<n>`, each sends
#                    xdg_activation_v1.activate(T<n>, <catcher>) and closes
#                    with Esc; a fourth show without a token sends none.
#   a11y             AC12: a fake org.a11y.Bus (a11y.py) on the private bus
#                    with ScreenReaderEnabled=true makes the next show an
#                    xdg_surface; after PropertiesChanged to false the next
#                    show is a layer again.
#   drop             AC13 (S-018d): three `sbarhive drop` shows with no drag,
#                    each closed by the 10 s timeout (no keyboard, so no Esc).
#                    The catcher takes no keyboard and has an empty input
#                    region, a click at (100, 700) in show 1 reaches the GTK
#                    window once and the hive stays, and each show requests
#                    one wl_data_device after get_layer_surface with every
#                    earlier one released first (tests/drop/layer.sh).
#   scheme           AC15 (S-018e): a fake org.freedesktop.portal.Desktop
#                    (portal.py) serves the Settings color-scheme on the
#                    private bus. With theme = "auto": 1 makes the centre
#                    cell color.dark.surface-3, SettingChanged to 2 while the
#                    hive is hidden makes the next show light, and back to 1
#                    dark. theme = "light" or "dark" in config.toml, and then
#                    SBARHIVE_COLOR_SCHEME (daemon restarted with it), win
#                    over the portal. Every show is a layer.
#   1.5              AC17 (S-018e): a reference show at scale 1, then a
#                    second private KWin at scale 1.5 (1920x1200 physical,
#                    1280x800 logical; the scale is seeded in
#                    kwinoutputconfig.json and read back with kscreen-doctor):
#                    preferred_scale(180) before the first configure, the
#                    hive buffer 590 x 543 (AC2), destination 393 x 362, and
#                    the screenshot box 1.5 times the scale 1 box within 1 px.
# The layer mode also checks AC16 (S-018e): every wl_pointer.enter on the
# catcher or the hive is followed by wp_cursor_shape_device_v1.set_shape(
# <serial>, 1); each show moves the pointer over the catcher and the hive.
# And AC18 (S-018f): each hide frees that show's buffers (buffers.py): every
# wl_buffer alive at the show's zwlr_layer_surface_v1.destroy is destroyed by
# the client then, before the compositor releases it, and no wl_buffer or
# wl_shm_pool of the show is left.
# Runs in a private kwin_wayland --virtual with a throwaway HOME, XDG dirs,
# runtime dir and session bus. Never touches the live desktop or profile.
# Usage: tests/overlay/kwin.sh [window|input|token|a11y|drop|scheme|1.5] [target/release]
# Conditions go to check() in single quotes and are expanded by eval.
# shellcheck disable=SC2016,SC2034,SC2329
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
MODE=layer
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in window|input|token|a11y|drop|scheme|1.5) MODE=$a ;; *) BIN="$a" ;; esac
done
INPUT="$ROOT/tests/drop/kwin-input/target/release/kwin-input"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
[ -x "$INPUT" ] || { echo "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"; exit 2; }
OUT="${OVERLAY_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT MODE INPUT

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE SBARHIVE_SURFACE SBARHIVE_COLOR_SCHEME
  # The 1.5 mode runs twice: PHASE=ref at scale 1, then PHASE=main at 1.5.
  local log="$OUT/kwin-$MODE${PHASE:+-$PHASE}"
  local FAILS=0
  check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }
  : >"$log-cli.log"
  mkdir -p "$XDG_CONFIG_HOME/sbarhive"
  if [ "$MODE" = input ]; then
    # AC9: keyboard exclusivity under focus follows mouse, level Low.
    printf '[Windows]\nFocusPolicy=FocusFollowsMouse\nFocusStealingPreventionLevel=1\n' >"$XDG_CONFIG_HOME/kwinrc"
  else
    printf '[Windows]\nFocusStealingPreventionLevel=1\n' >"$XDG_CONFIG_HOME/kwinrc"
  fi
  # English and light to start with; shows 4 and 5 of layer/window change them.
  # The scheme mode follows the system (AC15) until it changes the theme.
  if [ "$MODE" = scheme ]; then
    printf 'language = "en"\ntheme = "auto"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"
  else
    printf 'language = "en"\ntheme = "light"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"
  fi
  # Lets shot.py (python3) use org.kde.KWin.ScreenShot2 in this private KWin only.
  mkdir -p "$XDG_DATA_HOME/applications"
  printf '[Desktop Entry]\nType=Application\nName=sbh shot\nExec=%s\nNoDisplay=true\nX-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2\n' \
    "$(readlink -f "$(command -v python3)")" >"$XDG_DATA_HOME/applications/sbh-shot.desktop"
  kbuildsycoca6 --noincremental >/dev/null 2>&1

  # AC12: the fake accessibility bus owns org.a11y.Bus before anything else on
  # the private bus can activate the real at-spi-bus-launcher.
  local a11y="" a11yf="$XDG_RUNTIME_DIR/a11y.fifo"
  if [ "$MODE" = a11y ]; then
    mkfifo "$a11yf"
    python3 -I -W ignore::DeprecationWarning "$HERE/a11y.py" "$a11yf" true >"$log-a11y.log" 2>&1 &
    a11y=$!
    exec 4>"$a11yf"
    timeout 10 gdbus wait --session org.a11y.Bus || echo "INFO fake org.a11y.Bus did not appear"
  fi

  # AC15: the fake portal owns org.freedesktop.portal.Desktop before KWin and
  # other.py start, so nothing activates the real xdg-desktop-portal here.
  local portal="" portalf="$XDG_RUNTIME_DIR/portal.fifo"
  if [ "$MODE" = scheme ]; then
    mkfifo "$portalf"
    python3 -I -W ignore::DeprecationWarning "$HERE/portal.py" "$portalf" 1 >"$log-portal.log" 2>&1 &
    portal=$!
    exec 5>"$portalf"
    timeout 10 gdbus wait --session org.freedesktop.portal.Desktop || echo "INFO fake portal did not appear"
  fi

  # AC17: scale 1.5 on a 1920x1200 virtual output is 1280x800 logical, the
  # AC2 case. `kscreen-doctor output.Virtual-0.scale.1.5` exits 0 and changes
  # nothing in headless KWin 6.7.5 (no kde_output_configuration request), so
  # the scale is seeded in KWin's own output config and read back below.
  local kw=1280 kh=800
  if [ "$MODE" = 1.5 ] && [ "${PHASE:-}" = main ]; then
    kw=1920; kh=1200
    cat >"$XDG_CONFIG_HOME/kwinoutputconfig.json" <<'JSON'
[{"name": "outputs", "data": [{"connectorName": "Virtual-0", "mode": {"width": 1920, "height": 1200, "refreshRate": 60000}, "scale": 1.5, "transform": "Normal"}]},
 {"name": "setups", "data": [{"lidClosed": false, "outputs": [{"enabled": true, "outputIndex": 0, "position": {"x": 0, "y": 0}, "priority": 0}]}]}]
JSON
  fi

  # KWIN_WAYLAND_NO_PERMISSION_CHECKS lets the test's own input client and
  # shot.py use fake input and ScreenShot2 in this private compositor only.
  KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 \
    kwin_wayland --virtual --no-lockscreen --socket "$WAYLAND_DISPLAY" --width "$kw" --height "$kh" >"$log-kwin.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "kwin did not start"; tail "$log-kwin.log"; kill $kwin $a11y $portal 2>/dev/null; return 1; }
  sleep 2
  local outputs=""
  if [ "$MODE" = 1.5 ]; then
    outputs=$(KSCREEN_BACKEND=KWayland timeout 10 kscreen-doctor -o 2>&1 | sed 's/\x1b\[[0-9;]*m//g' | grep -E 'Geometry|Scale' | tr -s ' \t\n' ' ')
    echo "INFO kscreen-doctor: $outputs"
  fi

  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  "$INPUT" <"$f" >"$log-input.log" 2>&1 &
  exec 3>"$f"
  local OLOG="$log-other.log"
  GTK_A11Y=none python3 "$HERE/other.py" "$XDG_RUNTIME_DIR/token" >"$OLOG" 2>&1 &
  local other=$!
  sleep 2
  echo "move 200 200" >&3; sleep 0.5

  local W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/photo.png"
  local DLOG="$log-daemon.log"
  local env=()
  [ "$MODE" = window ] && env=(SBARHIVE_SURFACE=window)
  env "${env[@]}" SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$DLOG" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done

  shot() { timeout 10 python3 "$HERE/shot.py" "$1" >/dev/null 2>>"$log-shot.err"; }

  # Input helpers. Evdev codes: 1 Esc, 28 Enter, 106 Right.
  key() { echo "keydown $1" >&3; echo "sleep 40" >&3; echo "keyup $1" >&3; }
  esc() { key 1; }
  click() { echo "move $1 $2" >&3; echo "sleep 150" >&3; echo "down" >&3; echo "sleep 40" >&3; echo "up" >&3; }
  lines() { wc -l <"$1"; }

  # Deadlines in wall time.
  now_ms() { echo $(( ${EPOCHREALTIME/./} / 1000 )); }
  wait_file() { # wait_file PATH SECONDS
    local end=$(( $(now_ms) + $2 * 1000 ))
    while [ "$(now_ms)" -lt "$end" ]; do [ -s "$1" ] && return 0; sleep 0.1; done
    return 1
  }
  mime() { file --mime-type -b "$1" 2>/dev/null; }
  mime_of_ext() { case "$1" in jpg) echo image/jpeg ;; webp) echo image/webp ;; avif) echo image/avif ;; png) echo image/png ;; ico) echo image/vnd.microsoft.icon ;; pdf) echo application/pdf ;; *) echo "?" ;; esac; }
  # Visible output names in the work folder (temporary files of a running
  # conversion are renamed away, so only names that appear count).
  nfiles() { find "$W" -maxdepth 1 -type f ! -name '.*' -printf '%f\n' | sort | tr '\n' ' '; }
  new_files() { comm -13 <(tr ' ' '\n' <<<"$1" | sed '/^$/d') <(tr ' ' '\n' <<<"$2" | sed '/^$/d'); }

  # Requests print as "-> interface#id.request(...)" (wayland-rs) or with "@"
  # (libwayland). Events print without the arrow.
  local R='-> '
  count() { grep -cE -- "$R$1" <<<"$2"; }
  evcount() { grep -cE -- "^\[[0-9:.]+\] $1" <<<"$2"; }
  between() { sed -n "$(($1 + 1)),$2p" "$DLOG"; }
  # "[HH:MM:SS.micro]" of a trace line to milliseconds.
  ts_ms() { sed -E 's/^\[([0-9]+):([0-9]+):([0-9]+)\.([0-9]{3}).*/\1 \2 \3 \4/' <<<"$1" | { read -r h m s ms; echo $(( (10#$h * 3600 + 10#$m * 60 + 10#$s) * 1000 + 10#$ms )); }; }
  # The catcher of a show: the wl_surface given to get_layer_surface.
  catcher_of() { grep -oE -- "$R"'zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$1" | grep -oE 'wl_surface[#@][0-9]+$' | head -1; }
  layer_destroys() { count 'zwlr_layer_surface_v1[#@][0-9]+\.destroy\(' "$1"; }
  surface_lines() { grep -E '^hive surface: ' <<<"$1"; }
  no_protocol_error() { ! grep -qE "wl_display[#@]1\.error\(|[Pp]rotocol error" "$DLOG"; }

  # Window-local position of the ring-1 cell right of the centre, axial (1, 0),
  # from the shipped layout rule (crates/ui/src/layout.rs, geometry.rs): cell
  # radius hex.size.md / sqrt(3), spacing radius + gap / sqrt(3), x offset
  # spacing * sqrt(3), y offset 0. The hive is centred (AC2, AC4): (640, 400).
  local RIGHT_DX
  RIGHT_DX=$(python3 -I - "$ROOT/design/tokens.json" <<'PY'
import json, math, sys
t = json.load(open(sys.argv[1]))["hex"]
s = t["size"]["md"] / math.sqrt(3) + t["gap"] / math.sqrt(3)
print(round(s * math.sqrt(3)))
PY
)

  local marks=() names=() omarks=()
  begin() { names+=("$1"); marks+=("$(lines "$DLOG")"); omarks+=("$(lines "$OLOG")"); }
  finish() { marks+=("$(lines "$DLOG")"); omarks+=("$(lines "$OLOG")"); }
  seg_of() { between "${marks[$1]}" "${marks[$1 + 1]}"; }
  clicks_of() { sed -n "$((omarks[$1] + 1)),$((omarks[$1 + 1]))p" "$OLOG" | grep -c 'other: click'; }

  case "$MODE" in
  layer | window)
    # The first shot is taken before any show.
    shot "$log-before.png"
    show() { # show NAME COMMAND...: one show, closed with Esc
      local name=$1; shift
      begin "$name"
      "$@" >>"$log-cli.log" 2>&1
      sleep 1.5
      [ "$name" = plain ] && shot "$log-shown.png"
      # AC16: the pointer enters the catcher, then the hive, then the catcher
      # again (after the screenshot, so the hover does not change it).
      if [ "$MODE" = layer ] && [ "$name" != drop ]; then
        echo "move 201 201" >&3; sleep 0.3
        echo "move 640 400" >&3; sleep 0.3
        echo "move 200 200" >&3; sleep 0.3
      fi
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
    local nshows=${#names[@]}
    if [ "$MODE" = window ]; then
      # AC9 window path guard: a click on the GTK window takes the focus from
      # the hive, and focus loss still closes it there (S-001 AC4, S-014 AC3).
      begin focusloss
      "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1
      sleep 1.5
      click 100 700; sleep 1.5
    fi
    finish

    local trace; trace=$(cat "$DLOG")
    local xdg_all layer_all
    xdg_all=$(count 'xdg_wm_base[#@][0-9]+\.get_xdg_surface\(' "$trace")
    layer_all=$(count 'zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$trace")
    echo "INFO whole trace: get_xdg_surface $xdg_all, get_layer_surface $layer_all"

    local i
    for ((i = 0; i < nshows; i++)); do
      local n=$((i + 1)) name=${names[i]}
      local seg; seg=$(seg_of "$i")
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

      # AC11: one "hive surface: KIND X Y W H" line per show, and the focus
      # lines follow the keyboard events of the trace. A leave with a nil
      # surface (KWin sends it after the client destroyed the catcher) is not
      # counted: only leaves of a live surface must print "hive focus: false".
      local sl ns kind sx sy sw sh
      sl=$(surface_lines "$seg"); ns=$(grep -c . <<<"$sl")
      read -r _ _ kind sx sy sw sh <<<"$(head -1 <<<"$sl")"
      echo "INFO show $n ($name): ${ns} surface line(s): ${sl:-none}"
      local ft ff ke kl
      ft=$(grep -c '^hive focus: true$' <<<"$seg"); ff=$(grep -c '^hive focus: false$' <<<"$seg")
      ke=$(evcount 'wl_keyboard[#@][0-9]+\.enter\(' "$seg"); kl=$(evcount 'wl_keyboard[#@][0-9]+\.leave\([0-9]+, wl_surface' "$seg")
      echo "INFO show $n ($name): hive focus true $ft false $ff, wl_keyboard enter $ke leave $kl"

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
        check "AC11 show $n ($name): exactly one \"hive surface: layer X Y W H\" line" \
          '[ "$ns" -eq 1 ] && grep -qE "^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$" <<<"$sl"'
        # 1280x800 at scale 1, hive 393 x 362 (AC2): (444, 219).
        check "AC11 show $n ($name): the layer line is 444 219 393 362 (AC2 on 1280x800, scale 1)" \
          '[ "${kind:-}" = layer ] && [ "${sx:-}" = 444 ] && [ "${sy:-}" = 219 ] && [ "${sw:-}" = 393 ] && [ "${sh:-}" = 362 ]'
        check "AC11 show $n ($name): X Y equal the subsurface set_position in the trace" \
          '[ -n "${sy:-}" ] && grep -qE -- "${R}wl_subsurface[#@][0-9]+\.set_position\($sx, $sy\)" <<<"$seg"'
        check "AC11 show $n ($name): W H equal a wp_viewport set_destination in the trace" \
          '[ -n "${sh:-}" ] && grep -qE -- "${R}wp_viewport[#@][0-9]+\.set_destination\($sw, $sh\)" <<<"$seg"'
        check "AC11 show $n ($name): one \"hive focus: true\" per wl_keyboard.enter and one false per leave" \
          '[ "$ft" -eq "$ke" ] && [ "$ff" -eq "$kl" ]'
        [ "$name" != drop ] && check "AC11 show $n ($name): the hive got the keyboard (hive focus: true)" '[ "$ft" -ge 1 ]'
      else
        check "AC6 show $n ($name): no get_layer_surface with SBARHIVE_SURFACE=window" '[ "$gl" -eq 0 ]'
        # The first show can reuse the toplevel winit made when the daemon
        # started (tests/drop/lib.sh), so search up to the end of this show.
        local upto; upto=$(head -n "${marks[i + 1]}" "$DLOG")
        check "AC6 show $n ($name): the hive is an xdg_surface as today" '[ "$(count "xdg_wm_base[#@][0-9]+\.get_xdg_surface\(" "$upto")" -ge 1 ]'
        check "AC11 show $n ($name): exactly one \"hive surface: window X Y W H\" line" \
          '[ "$ns" -eq 1 ] && grep -qE "^hive surface: window -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$" <<<"$sl"'
        # No Wayland client knows its own window position: W H must be the
        # hive's size (393 x 362 within 1 px), X Y only have to lie on screen.
        check "AC11 show $n ($name): the window line has the hive's size and lies on the 1280x800 screen" \
          '[ "${kind:-}" = window ] && [ "$sw" -ge 392 ] && [ "$sw" -le 394 ] && [ "$sh" -ge 361 ] && [ "$sh" -le 363 ] && [ "$sx" -ge 0 ] && [ "$sy" -ge 0 ] && [ $((sx + sw)) -le 1280 ] && [ $((sy + sh)) -le 800 ]'
      fi
    done

    if [ "$MODE" = layer ]; then
      check "AC4/AC5: no get_xdg_surface in the whole trace (no hive toplevel, not even a hidden one)" '[ "$xdg_all" -eq 0 ]'
      # AC16: match each pointer enter on this show's catcher or hive
      # subsurface to a later set_shape(<its serial>, 1) before the next
      # pointer enter. Serials come from the compositor, so they are unique
      # across the four connections of the trace.
      local ac16
      ac16=$(python3 -I - "$DLOG" "${marks[@]}" <<'PY'
import re, sys
lines = open(sys.argv[1], errors="replace").read().split("\n")
marks = [int(m) for m in sys.argv[2:]]
enter = re.compile(r"^\[[0-9:.]+\] wl_pointer[#@][0-9]+\.enter\(([0-9]+), (wl_surface[#@][0-9]+),")
total = {"catcher": 0, "hive": 0}
bad = []
for i in range(len(marks) - 1):
    seg = lines[marks[i]:marks[i + 1]]
    cat = hive = None
    for l in seg:
        m = re.search(r"-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, (wl_surface[#@][0-9]+)", l)
        if m and cat is None:
            cat = m.group(1)
        m = re.search(r"-> wl_subcompositor[#@][0-9]+\.get_subsurface\(new id wl_subsurface[#@][0-9]+, (wl_surface[#@][0-9]+)", l)
        if m and hive is None:
            hive = m.group(1)
    got = {"catcher": 0, "hive": 0}
    shaped = 0
    for j, l in enumerate(seg):
        m = enter.match(l)
        if not m or m.group(2) not in (cat, hive):
            continue
        serial, kind = m.group(1), ("catcher" if m.group(2) == cat else "hive")
        got[kind] += 1
        ok = False
        for l2 in seg[j + 1:]:
            if enter.match(l2):
                break
            if re.search(r"-> wp_cursor_shape_device_v1[#@][0-9]+\.set_shape\(" + serial + r", 1\)", l2):
                ok = True
                break
        if ok:
            shaped += 1
        else:
            bad.append(f"show {i + 1} {kind} enter serial {serial}")
    total["catcher"] += got["catcher"]
    total["hive"] += got["hive"]
    print(f"INFO AC16 show {i + 1}: catcher {cat}, hive {hive}, pointer enters on the catcher {got['catcher']} on the hive {got['hive']}, followed by set_shape(serial, 1) {shaped}")
print(f"INFO AC16 enters without set_shape(serial, 1): {', '.join(bad) or 'none'}")
print(f"SUMMARY {total['catcher']} {total['hive']} {len(bad)}")
PY
)
      grep '^INFO' <<<"$ac16"
      local c16 h16 b16; read -r _ c16 h16 b16 <<<"$(grep '^SUMMARY' <<<"$ac16")"
      check "AC16: the pointer entered the catcher and the hive (the test reached both)" '[ "${c16:-0}" -ge 1 ] && [ "${h16:-0}" -ge 1 ]'
      check "AC16: every wl_pointer.enter on the catcher or the hive is followed by wp_cursor_shape_device_v1.set_shape(<serial>, 1)" '[ -n "${b16:-}" ] && [ "$b16" -eq 0 ]'

      # AC18 (S-018f): each hide frees that show's buffers. buffers.py follows
      # every wl_buffer and wl_shm_pool of the show line by line: a buffer still
      # alive at the show's zwlr_layer_surface_v1.destroy must be destroyed by
      # the client within 250 ms and before the compositor's release of it, and
      # no buffer or pool the show made is alive at its end.
      local ranges=() j
      for ((j = 0; j < nshows; j++)); do ranges+=("${marks[j]}:${marks[j + 1]}"); done
      local ac18; ac18=$(python3 -I "$HERE/buffers.py" "$DLOG" "${ranges[@]}" 2>&1)
      grep '^INFO' <<<"$ac18"
      for ((j = 0; j < nshows; j++)); do
        local n=$((j + 1)) name=${names[j]} shm px ld ah lshm lpx lb lp
        read -r shm px _ ld ah lshm lpx lb lp <<<"$(grep "^SHOW $n " <<<"$ac18" | cut -d' ' -f3- | sed -E 's/[a-z_]+=//g')"
        echo "INFO AC18 show $n ($name): shm buffers ${shm:-?}, single pixel ${px:-?}, layer destroys ${ld:-?}, alive at the hide ${ah:-?}, not freed at the hide shm ${lshm:-?} px ${lpx:-?}, left buffers ${lb:-?} pools ${lp:-?}"
        check "AC18 show $n ($name): the show drew into wl_shm buffers (the check is not empty)" '[ "${shm:-0}" -ge 1 ]'
        check "AC18 show $n ($name): the hide sends zwlr_layer_surface_v1.destroy (exactly one)" '[ "${ld:-0}" -eq 1 ]'
        check "AC18 show $n ($name): every wl_shm buffer alive at the hide gets wl_buffer.destroy at the hide, not after its release" '[ -n "${lshm:-}" ] && [ "$lshm" -eq 0 ]'
        check "AC18 show $n ($name): the catcher's single pixel buffer gets wl_buffer.destroy at the hide, not after its release" '[ -n "${lpx:-}" ] && [ "$lpx" -eq 0 ]'
        check "AC18 show $n ($name): no wl_buffer or wl_shm_pool of the show is left alive" '[ -n "${lp:-}" ] && [ "$lb" -eq 0 ] && [ "$lp" -eq 0 ]'
      done
    else
      check "AC6: no get_layer_surface in the whole trace" '[ "$layer_all" -eq 0 ]'
      check "AC6: get_xdg_surface in the trace" '[ "$xdg_all" -ge 1 ]'
      local fl; fl=$(seg_of "$nshows")
      local fl_clicks; fl_clicks=$(clicks_of "$nshows")
      echo "INFO focusloss: GTK clicks $fl_clicks, xdg_toplevel.destroy $(count 'xdg_toplevel[#@][0-9]+\.destroy\(' "$fl"), focus lines $(grep '^hive focus' <<<"$fl" | tr '\n' ' ')"
      check "AC9 window path (guard): the click reached the GTK window" '[ "$fl_clicks" -eq 1 ]'
      check "AC9 window path (guard): focus loss closes the hive (xdg_toplevel destroyed)" \
        'grep -q "^hive focus: false$" <<<"$fl" && [ "$(count "xdg_toplevel[#@][0-9]+\.destroy\(" "$fl")" -ge 1 ]'
    fi
    check "AC$([ "$MODE" = layer ] && echo 4 || echo 6): no protocol error in the daemon log" 'no_protocol_error'

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
      # AC11: the first show's line, against the screenshot.
      local l1; l1=$(surface_lines "$(seg_of 0)" | head -1)
      local _a _b _k lx ly lw lh; read -r _a _b _k lx ly lw lh <<<"$l1"
      check "AC11: the screenshot box lies inside the first show's \"hive surface\" rectangle" \
        '[ -n "${bh:-}" ] && [ -n "${lh:-}" ] && [ "$bx" -ge "$lx" ] && [ "$by" -ge "$ly" ] && [ $((bx + bw)) -le $((lx + lw)) ] && [ $((by + bh)) -le $((ly + lh)) ]'
    fi
    ;;

  input)
    local r1; r1=$("$BIN/sbarhive" actions "$W/photo.png" 2>/dev/null | awk -F'\t' '$2 == 1 { print $1 }')
    local first_ext second_ext
    first_ext=$(sed -n 1p <<<"$r1"); first_ext=${first_ext##*.to-}
    second_ext=$(sed -n 2p <<<"$r1"); second_ext=${second_ext##*.to-}
    echo "INFO ring 1 for photo.png: $(tr '\n' ' ' <<<"$r1")(slot 0 $first_ext, slot 1 right of the centre $second_ext); right cell at ($((640 + RIGHT_DX)), 400)"
    local outside_before roam_before

    # 1. AC8: a click on the ring-1 cell right of the centre chooses it.
    begin cell
    "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1; sleep 1.5
    click $((640 + RIGHT_DX)) 400
    wait_file "$W/photo.$second_ext" 10; local cell_ok=$?
    sleep 0.5
    local cell_other=no; [ -e "$W/photo.$first_ext" ] && cell_other=yes

    # 2. AC8: a click at (100, 700) lands on the catcher, not the GTK window.
    begin outside
    "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1; sleep 1.5
    outside_before=$(nfiles)
    click 100 700; sleep 1.5
    local outside_files; outside_files=$(nfiles)

    # 3. AC9: pointer over the GTK window, Right then Enter.
    begin keys
    "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1; sleep 1.5
    echo "move 100 700" >&3; sleep 0.5
    key 106; sleep 0.3; key 28
    wait_file "$W/photo.$first_ext" 10; local keys_ok=$?
    sleep 0.5

    # 4. AC9: the pointer roams over 5 points (no click), then Esc.
    begin roam
    "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1; sleep 1.5
    roam_before=$(nfiles)
    local p
    for p in "100 700" "1200 80" "640 400" "$((640 + RIGHT_DX)) 400" "60 60"; do
      echo "move $p" >&3; sleep 0.4
    done
    esc; sleep 1
    local roam_files; roam_files=$(nfiles)

    # 5. AC9: opening the settings while the hive is shown does not close it.
    begin settings
    "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1; sleep 1.5
    local s1; s1=$(lines "$DLOG")
    "$BIN/sbarhive" settings >>"$log-cli.log" 2>&1; sleep 3
    local s2; s2=$(lines "$DLOG")
    esc; sleep 1
    click 100 700; sleep 1
    finish

    local seg

    seg=$(seg_of 0)
    echo "INFO cell: photo.$second_ext $( [ -s "$W/photo.$second_ext" ] && mime "$W/photo.$second_ext" || echo missing), GTK clicks $(clicks_of 0), layer destroys $(layer_destroys "$seg")"
    check "AC8 click on the ring-1 cell right of the centre: photo.$second_ext written (on_choose with its id)" '[ "$cell_ok" -eq 0 ]'
    check "AC8 click on the right cell: output is $(mime_of_ext "$second_ext")" '[ "$(mime "$W/photo.$second_ext")" = "$(mime_of_ext "$second_ext")" ]'
    check "AC8 click on the right cell: no other action ran (no photo.$first_ext)" '[ "$cell_other" = no ]'
    check "AC8 click on the right cell: the GTK window got 0 clicks" '[ "$(clicks_of 0)" -eq 0 ]'
    check "AC8 click on the right cell: the hive is a layer and closed" \
      '[ "$(count "zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(" "$seg")" -eq 1 ] && [ "$(layer_destroys "$seg")" -eq 1 ]'

    seg=$(seg_of 1)
    local press destroy dt=none
    press=$(grep -nE -- '^\[[0-9:.]+\] wl_pointer[#@][0-9]+\.button\([0-9]+, [0-9]+, 272, 1\)' <<<"$seg" | head -1)
    if [ -n "$press" ]; then
      destroy=$(tail -n +"${press%%:*}" <<<"$seg" | grep -E -- "${R}zwlr_layer_surface_v1[#@][0-9]+\.destroy\(" | head -1)
      [ -n "$destroy" ] && dt=$(( $(ts_ms "$destroy") - $(ts_ms "${press#*:}") ))
    fi
    echo "INFO outside: press on the catcher ${press:+seen}${press:-none}, press to zwlr_layer_surface_v1.destroy ${dt} ms, GTK clicks $(clicks_of 1), files [$outside_before] -> [$outside_files]"
    check "AC8 click at (100, 700): the press reaches the daemon's catcher" \
      '[ -n "$press" ] && grep -qE "wl_pointer[#@][0-9]+\.enter\([0-9]+, $(catcher_of "$seg")," <<<"$seg"'
    check "AC8 click at (100, 700): the hive closes within 500 ms of the press" '[ "$dt" != none ] && [ "$dt" -le 500 ]'
    check "AC8 click at (100, 700): exactly one layer surface destroyed (one dismiss)" '[ "$(layer_destroys "$seg")" -eq 1 ]'
    check "AC8 click at (100, 700): no action ran (no new file)" '[ -z "$(new_files "$outside_before" "$outside_files")" ]'
    check "AC8 click at (100, 700): the GTK window below got 0 clicks" '[ "$(clicks_of 1)" -eq 0 ]'

    seg=$(seg_of 2)
    echo "INFO keys: photo.$first_ext $( [ -s "$W/photo.$first_ext" ] && mime "$W/photo.$first_ext" || echo missing), keyboard leave $(evcount 'wl_keyboard[#@][0-9]+\.leave\(' "$seg")"
    check "AC9 Right then Enter, pointer over the GTK window: photo.$first_ext written (the id ui_tests chooses, slot 0)" '[ "$keys_ok" -eq 0 ]'
    check "AC9 Right then Enter: output is $(mime_of_ext "$first_ext")" '[ "$(mime "$W/photo.$first_ext")" = "$(mime_of_ext "$first_ext")" ]'
    check "AC9 Right then Enter: the hive had the keyboard (enter on the catcher, hive focus: true)" \
      'grep -qE "wl_keyboard[#@][0-9]+\.enter\([0-9]+, $(catcher_of "$seg")," <<<"$seg" && grep -q "^hive focus: true$" <<<"$seg"'
    check "AC9 Right then Enter: the hive is closed after choosing" '[ "$(layer_destroys "$seg")" -eq 1 ]'

    seg=$(seg_of 3)
    local esc_line esc_destroy="" leave_before=0
    esc_line=$(grep -nE -- '^\[[0-9:.]+\] wl_keyboard[#@][0-9]+\.key\([0-9]+, [0-9]+, 1, 1\)' <<<"$seg" | head -1)
    if [ -n "$esc_line" ]; then
      esc_destroy=$(tail -n +"${esc_line%%:*}" <<<"$seg" | grep -cE -- "${R}zwlr_layer_surface_v1[#@][0-9]+\.destroy\(")
      leave_before=$(head -n "${esc_line%%:*}" <<<"$seg" | grep -cE -- '^\[[0-9:.]+\] wl_keyboard[#@][0-9]+\.leave\(')
    fi
    echo "INFO roam: Esc key event ${esc_line:+seen}${esc_line:-none}, destroys after it ${esc_destroy:-0}, keyboard leaves before it $leave_before, layer destroys before Esc $(head -n "${esc_line%%:*}" <<<"$seg" | grep -cE -- "${R}zwlr_layer_surface_v1[#@][0-9]+\.destroy\(")"
    check "AC9 roam over 5 points (focus follows mouse): the hive stays shown until Esc" \
      '[ -n "$esc_line" ] && [ "$(head -n "${esc_line%%:*}" <<<"$seg" | grep -cE -- "${R}zwlr_layer_surface_v1[#@][0-9]+\.destroy\(")" -eq 0 ]'
    check "AC9 roam: the keyboard never left the hive before Esc" '[ -n "$esc_line" ] && [ "$leave_before" -eq 0 ]'
    check "AC9 roam: Esc reaches the hive and closes it" '[ -n "$esc_line" ] && [ "${esc_destroy:-0}" -eq 1 ]'
    check "AC9 roam: no action ran (no new file)" '[ -z "$(new_files "$roam_before" "$roam_files")" ]'

    seg=$(seg_of 4)
    local during; during=$(between "$s1" "$s2")
    echo "INFO settings: get_xdg_surface after the command $(count 'xdg_wm_base[#@][0-9]+\.get_xdg_surface\(' "$during"), layer destroys in the 3 s after it $(layer_destroys "$during")"
    check "AC9 settings: the hive is a layer surface" '[ "$(count "zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(" "$seg")" -eq 1 ]'
    check "AC9 settings: \`sbarhive settings\` while shown does not close the hive (no destroy in 3 s)" \
      '[ "$(count "xdg_wm_base[#@][0-9]+\.get_xdg_surface\(" "$during")" -ge 1 ] && [ "$(layer_destroys "$during")" -eq 0 ]'
    check "AC8/AC9: no protocol error in the daemon log" 'no_protocol_error'
    ;;

  token)
    local n
    for n in 1 2 3; do
      begin "token$n"
      "$BIN/sbarhive" open --token "sbh-token-$n" "$W/photo.png" >>"$log-cli.log" 2>&1
      sleep 1.5; esc; sleep 1
    done
    begin plain
    "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1
    sleep 1.5; esc; sleep 1
    finish
    local i
    for ((i = 0; i < 4; i++)); do
      local seg cat acts mine el ed=0 al gl
      seg=$(seg_of "$i"); cat=$(catcher_of "$seg")
      acts=$(grep -E -- "${R}xdg_activation_v1[#@][0-9]+\.activate\(" <<<"$seg")
      gl=$(grep -nE -- "${R}zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(" <<<"$seg" | head -1 | cut -d: -f1)
      al=$(grep -nE -- "${R}xdg_activation_v1[#@][0-9]+\.activate\(" <<<"$seg" | head -1 | cut -d: -f1)
      el=$(grep -nE -- '^\[[0-9:.]+\] wl_keyboard[#@][0-9]+\.key\([0-9]+, [0-9]+, 1, 1\)' <<<"$seg" | head -1 | cut -d: -f1)
      [ -n "$el" ] && ed=$(tail -n +"$el" <<<"$seg" | grep -cE -- "${R}zwlr_layer_surface_v1[#@][0-9]+\.destroy\(")
      echo "INFO show $((i + 1)) (${names[i]}): catcher ${cat:-none}, activate: $(sed -E 's/^\[[^]]*\] +//' <<<"$acts" | tr '\n' ' ')"
      if [ "$i" -lt 3 ]; then
        local t="sbh-token-$((i + 1))"
        mine=$(grep -cE "activate\(\"$t\", ${cat:-none}\)" <<<"$acts")
        check "AC10 show $((i + 1)): activate(\"$t\", <catcher>) sent once on this show's catcher" \
          '[ -n "$cat" ] && [ "$mine" -eq 1 ] && [ "$(grep -c . <<<"$acts")" -eq 1 ]'
        check "AC10 show $((i + 1)): activate after get_layer_surface" '[ -n "$gl" ] && [ -n "$al" ] && [ "$al" -gt "$gl" ]'
        check "AC10 show $((i + 1)): Esc after the token closes the hive" '[ -n "$el" ] && [ "$ed" -eq 1 ]'
      else
        check "AC10 show 4 (no token): no activate, the last token is not left pending" '[ -z "$acts" ]'
        check "AC10 show 4: a layer surface, closed by Esc" '[ -n "$cat" ] && [ -n "$el" ] && [ "$ed" -eq 1 ]'
      fi
    done
    check "AC10: no protocol error in the daemon log" 'no_protocol_error'
    ;;

  a11y)
    begin reader-on
    "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1
    sleep 1.5; esc; sleep 1.5
    echo off >&4; sleep 1.5
    begin reader-off
    "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1
    sleep 1.5; esc; sleep 1
    finish
    local on off upto
    on=$(seg_of 0); off=$(seg_of 1); upto=$(head -n "${marks[1]}" "$DLOG")
    # Who asked is not checked: KWin itself reads the same properties.
    echo "INFO a11y: $(tr '\n' ' ' <"$log-a11y.log")"
    echo "INFO reader on: get_layer_surface $(count 'zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$on"), get_xdg_surface up to its end $(count 'xdg_wm_base[#@][0-9]+\.get_xdg_surface\(' "$upto")"
    echo "INFO reader off: get_layer_surface $(count 'zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$off"), get_xdg_surface $(count 'xdg_wm_base[#@][0-9]+\.get_xdg_surface\(' "$off")"
    check "AC12 ScreenReaderEnabled=true: the next show is an xdg_surface (window)" \
      '[ "$(count "xdg_wm_base[#@][0-9]+\.get_xdg_surface\(" "$upto")" -ge 1 ]'
    check "AC12 ScreenReaderEnabled=true: no get_layer_surface" '[ "$(count "zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(" "$on")" -eq 0 ]'
    check "AC12 PropertiesChanged to false: the next show is a layer again" \
      '[ "$(count "zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(" "$off")" -eq 1 ]'
    check "AC12 PropertiesChanged to false: no get_xdg_surface for that show" '[ "$(count "xdg_wm_base[#@][0-9]+\.get_xdg_surface\(" "$off")" -eq 0 ]'
    check "AC12: no protocol error in the daemon log" 'no_protocol_error'
    ;;

  drop)
    # shellcheck source=../drop/layer.sh
    source "$ROOT/tests/drop/layer.sh"
    local n
    for n in 1 2 3; do
      begin "drop$n"
      "$BIN/sbarhive" drop >>"$log-cli.log" 2>&1
      sleep 1.5
      # AC13: in drop mode a click outside the hive passes to the window below.
      [ "$n" = 1 ] && click 100 700
      # No drag enters it, so it closes 10 s after showing (S-014a AC3).
      sleep 11
    done
    finish
    local i
    for ((i = 0; i < 3; i++)); do
      layer_drop_show $((i + 1)) "${marks[i]}" "${marks[i + 1]}" 0
    done
    local seg cat gls dls dt=none pe pb ke
    seg=$(seg_of 0); cat=$(catcher_of "$seg")
    gls=$(grep -E -- "${R}zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(" <<<"$seg" | head -1)
    dls=$(grep -E -- "${R}zwlr_layer_surface_v1[#@][0-9]+\.destroy\(" <<<"$seg" | head -1)
    [ -n "$gls" ] && [ -n "$dls" ] && dt=$(( $(ts_ms "$dls") - $(ts_ms "$gls") ))
    pe=$(evcount "wl_pointer[#@][0-9]+\.enter\([0-9]+, ${cat:-none}," "$seg")
    pb=$(evcount 'wl_pointer[#@][0-9]+\.button\(' "$seg")
    ke=$(evcount 'wl_keyboard[#@][0-9]+\.enter\(' "$seg")
    echo "INFO drop1: GTK clicks $(clicks_of 0), layer destroys $(layer_destroys "$seg"), shown for ${dt} ms, pointer enter on the catcher $pe, pointer buttons $pb, keyboard enter $ke"
    check "AC13 drop mode: the click at (100, 700) reaches the GTK window (one click)" '[ "$(clicks_of 0)" -eq 1 ]'
    check "AC13 drop mode: the catcher gets no pointer enter and the daemon no button" '[ -n "$cat" ] && [ "$pe" -eq 0 ] && [ "$pb" -eq 0 ]'
    check "AC13 drop mode: the catcher gets no keyboard (no wl_keyboard.enter)" '[ "$ke" -eq 0 ]'
    check "AC13 drop mode: the hive stays after the click (closed once, by the 10 s timeout, not by the click)" \
      '[ "$(layer_destroys "$seg")" -eq 1 ] && [ "$dt" != none ] && [ "$dt" -ge 9000 ]'
    check "AC13: no protocol error in the daemon log" 'no_protocol_error'
    ;;

  scheme)
    # AC15. The sample is a 3 x 3 patch in the centre cell above the square a
    # preview will take (S-018 AC24): 0.72 of the cell radius above the
    # centre, inside the hexagon. Both hive-en-* references show surface-3
    # there and at the centre; the exact centre (640, 400) is printed too.
    local sy
    sy=$(python3 -I - "$ROOT/design/tokens.json" <<'PY'
import json, math, sys
r = json.load(open(sys.argv[1]))["hex"]["size"]["md"] / math.sqrt(3)
print(400 - round(0.72 * r))
PY
)
    restart_daemon() { # restart_daemon [VAR=VALUE...]: a new daemon, same log
      "$BIN/sbarhive" quit >/dev/null 2>&1
      timeout 5 tail --pid="$d" -f /dev/null || kill "$d" 2>/dev/null
      env "$@" SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >>"$DLOG" 2>&1 &
      d=$!
      for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
    }
    local want=()
    sshow() { # sshow NAME EXPECTED: one show, a screenshot, closed with Esc
      begin "$1"; want+=("$2")
      "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1
      sleep 1.5
      shot "$log-$1.png"
      esc; sleep 1
    }
    sshow portal-1 dark
    echo 2 >&5; sleep 1.5
    sshow changed-to-2 light
    echo 1 >&5; sleep 1.5
    sshow changed-to-1 dark
    printf 'language = "en"\ntheme = "light"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"; sleep 3
    sshow config-light-portal-1 light
    echo 2 >&5
    printf 'language = "en"\ntheme = "dark"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"; sleep 3
    sshow config-dark-portal-2 dark
    printf 'language = "en"\ntheme = "auto"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"; sleep 3
    restart_daemon SBARHIVE_COLOR_SCHEME=dark
    sshow env-dark-portal-2 dark
    echo 1 >&5; sleep 0.5
    restart_daemon SBARHIVE_COLOR_SCHEME=light
    sshow env-light-portal-1 light
    finish
    echo "INFO portal: $(grep -cE '^call ' "$log-portal.log") call(s), $(grep -E '^SettingChanged' "$log-portal.log" | tr '\n' ' ')"
    local i
    for ((i = 0; i < ${#names[@]}; i++)); do
      local seg name=${names[i]} exp=${want[i]} got centre gl xs
      seg=$(seg_of "$i")
      gl=$(count 'zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$seg")
      xs=$(count 'xdg_wm_base[#@][0-9]+\.get_xdg_surface\(' "$seg")
      got="none"; centre="none"
      if [ -s "$log-$name.png" ]; then
        got=$(python3 -I "$HERE/pixel.py" "$log-$name.png" 640 "$sy" "$ROOT/design/tokens.json" 2>&1)
        centre=$(python3 -I "$HERE/pixel.py" "$log-$name.png" 640 400 "$ROOT/design/tokens.json" 2>&1)
      fi
      echo "INFO show $((i + 1)) ($name): get_layer_surface $gl, get_xdg_surface $xs, patch at (640, $sy) $got, centre (640, 400) $centre"
      check "AC15 show $((i + 1)) ($name): a layer surface (not the window path)" '[ "$gl" -eq 1 ] && [ "$xs" -eq 0 ]'
      check "AC15 show $((i + 1)) ($name): the centre cell is color.$exp.surface-3" '[ "${got%% *}" = "$exp" ]'
    done
    check "AC15: no protocol error in the daemon log" 'no_protocol_error'
    ;;

  1.5)
    shot "$log-before.png"
    begin "scale-${PHASE}"
    "$BIN/sbarhive" open "$W/photo.png" >>"$log-cli.log" 2>&1
    sleep 1.5
    shot "$log-shown.png"
    esc; sleep 1
    finish
    local box="none"
    if [ -s "$log-before.png" ] && [ -s "$log-shown.png" ]; then
      box=$(python3 -I "$HERE/boxdiff.py" "$log-before.png" "$log-shown.png" 8 2>&1)
    fi
    if [ "$PHASE" = ref ]; then
      # Scale 1: the box this hive draws, to compare the 1.5 box against.
      echo "INFO scale 1 reference box: $box"
      echo "$box" >"$OUT/kwin-1.5-ref.box"
    else
      local seg; seg=$(seg_of 0)
      local gl xs pl cl
      gl=$(count 'zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$seg")
      xs=$(count 'xdg_wm_base[#@][0-9]+\.get_xdg_surface\(' "$seg")
      pl=$(grep -nE '^\[[0-9:.]+\] wp_fractional_scale_v1[#@][0-9]+\.preferred_scale\(180\)' <<<"$seg" | head -1 | cut -d: -f1)
      cl=$(grep -nE '^\[[0-9:.]+\] zwlr_layer_surface_v1[#@][0-9]+\.configure\(' <<<"$seg" | head -1 | cut -d: -f1)
      local hive; hive=$(grep -oE -- "${R}wl_subcompositor[#@][0-9]+\.get_subsurface\(new id wl_subsurface[#@][0-9]+, wl_surface[#@][0-9]+" <<<"$seg" | grep -oE 'wl_surface[#@][0-9]+$' | head -1)
      local bufs nbuf nbad
      bufs=$(grep -oE -- "${R}wl_shm_pool[#@][0-9]+\.create_buffer\(new id wl_buffer[#@][0-9]+, -?[0-9]+, [0-9]+, [0-9]+" <<<"$seg" | sed -E 's/.*new id (wl_buffer[#@][0-9]+), -?[0-9]+, ([0-9]+), ([0-9]+)$/\1 \2 \3/')
      nbuf=$(grep -c . <<<"$bufs"); nbad=$(awk '$2 != 590 || $3 != 543' <<<"$bufs" | grep -c .)
      local attached=0 b
      while read -r b; do
        [ -n "$b" ] && grep -qE -- "${R}${hive:-none}\.attach\($b, " <<<"$seg" && attached=$((attached + 1))
      done < <(awk '$2 == 590 && $3 == 543 { print $1 }' <<<"$bufs")
      local sl; sl=$(surface_lines "$seg" | head -1)
      echo "INFO scale 1.5: get_layer_surface $gl, get_xdg_surface $xs, first preferred_scale(180) at segment line ${pl:-none}, first layer configure at ${cl:-none}"
      echo "INFO scale 1.5: hive ${hive:-none}, shm buffers $(awk '{ printf "%s %sx%s; ", $1, $2, $3 }' <<<"$bufs")attached to the hive at 590x543: $attached; ${sl:-no surface line}"
      check "AC17: kscreen-doctor reports scale 1.5 and a 1280x800 logical output" \
        'grep -q "Geometry: 0,0 1280x800" <<<"$outputs" && grep -q "Scale: 1.5" <<<"$outputs"'
      check "AC17: the hive is a layer surface at scale 1.5" '[ "$gl" -eq 1 ] && [ "$xs" -eq 0 ]'
      check "AC17: preferred_scale(180) arrives before the first zwlr_layer_surface_v1.configure" \
        '[ -n "$pl" ] && [ -n "$cl" ] && [ "$pl" -lt "$cl" ]'
      check "AC17: every wl_shm buffer of the show is 590 x 543 (AC2 at 180), and the hive gets one" \
        '[ "$nbuf" -ge 1 ] && [ "$nbad" -eq 0 ] && [ "$attached" -ge 1 ]'
      check "AC17: the hive is placed at (444, 219) with set_destination(393, 362) (AC2, logical)" \
        'grep -qE -- "${R}wl_subsurface[#@][0-9]+\.set_position\(444, 219\)" <<<"$seg" && grep -qE -- "${R}wp_viewport[#@][0-9]+\.set_destination\(393, 362\)" <<<"$seg"'
      local ref; ref=$(cat "$OUT/kwin-1.5-ref.box" 2>/dev/null || echo none)
      echo "INFO scale 1.5 box (physical): $box, scale 1 reference box: $ref"
      local bx by bw bh rx ry rw rh
      read -r bx by bw bh <<<"$box"; read -r rx ry rw rh <<<"$ref"
      # The opaque hive pixels are narrower than the surface (384 against
      # 393 at scale 1), so the box is held to 1.5 times the scale 1 box,
      # centred on the 1920x1200 output, inside the 1.5 times subsurface.
      check "AC17: the screenshot box width is 1.5 times the scale 1 box within 1 px" \
        '[ -n "${bh:-}" ] && [ -n "${rh:-}" ] && python3 -I -c "import sys; w,r=map(int,sys.argv[1:]); sys.exit(not abs(w-1.5*r)<=1)" "$bw" "$rw"'
      check "AC17: the screenshot box is centred on (960, 600) within 1 px and inside (666, 328.5, 589.5, 543)" \
        '[ -n "${bh:-}" ] && python3 -I -c "import sys; x,y,w,h=map(int,sys.argv[1:]); sys.exit(not (abs(x+w/2-960)<=1 and abs(y+h/2-600)<=1 and x>=665 and y>=328 and x+w<=1256 and y+h<=872))" "$bx" "$by" "$bw" "$bh"'
      check "AC17: no protocol error in the daemon log" 'no_protocol_error'
    fi
    ;;
  esac

  # The 1.5 reference phase only measures; the main phase gives the result.
  if [ "${PHASE:-}" != ref ]; then
    if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
  fi

  exec 3>&-
  [ -n "$a11y" ] && { echo quit >&4; exec 4>&-; }
  [ -n "$portal" ] && { echo quit >&5; exec 5>&-; }
  "$BIN/sbarhive" quit >/dev/null 2>&1
  sleep 0.5
  kill $other $d $a11y $portal 2>/dev/null
  kill $kwin 2>/dev/null
  timeout 5 tail --pid=$kwin -f /dev/null || kill -9 $kwin
}
export -f inner

echo "== kwin $MODE"
# One private session per phase; only the 1.5 mode has two (scale 1, then 1.5).
phases=("")
[ "$MODE" = 1.5 ] && { phases=(ref main); rm -f "$OUT/kwin-1.5-ref.box"; }
: >"$OUT/kwin-$MODE.txt"
for PHASE in "${phases[@]}"; do
  export PHASE
  SANDBOX="$(mktemp -d)"
  RUNTIME="$(mktemp -d)"
  chmod 700 "$RUNTIME"
  # Environment set BEFORE dbus-run-session, so nothing the private bus
  # activates can reach the live desktop or the real profile.
  env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
    HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
    XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
    XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY=wayland-sbh-overlay-kwin \
    dbus-run-session -- bash -c inner 2>"$OUT/kwin-$MODE${PHASE:+-$PHASE}-bus.log" | tee -a "$OUT/kwin-$MODE.txt"
  rm -rf "$SANDBOX" "$RUNTIME"
done
grep -q '^RESULT PASS' "$OUT/kwin-$MODE.txt"
