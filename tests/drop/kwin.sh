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
# S-019d (backlog/S-019-hive-at-pointer.md, AC8 to AC10 and the manager's
# decision 1, and decision 12 of 2026-10-21): in the five shows above the
# drag stands still for at least 400 ms after the hive is mapped, then moves
# to the hive's centre (ANCHOR_SETTLE, tests/drop/lib.sh), so they take the
# decision 12 path (layer.sh mode "still"). Three kinds of shows follow, with
# the drag held at P:
#   jitter  a 1 px drag motion around P every 10 ms for 600 ms, started
#           before `sbarhive drop`: P = (400, 500) three times, each dropped
#           on the WEBP cell (the layout there is plan_rings, so the cell is
#           where it is today relative to the hive), and P = (100, 700),
#           released over the centre cell;
#   still   the drag stands still for 1 s after `sbarhive drop`, then the
#           jitter at (400, 500), outside the centred hive (decision 12):
#           one anchor line, "hive anchor: none", one
#           wl_subsurface.set_position, in the middle (444, 219), the
#           catcher region the hive rectangle within 400 ms of the first
#           frame (layer.sh), and the drop on that centred hive's WEBP cell
#           writes the file; positioning a still hand is --at only;
#   at      the drag stands still at (400, 500) and the command is
#           `sbarhive drop --at 400,500`: anchored at once, one
#           set_position at (204, 319), no "hive anchor: none", and the drop
#           on the WEBP cell writes the file.
#   late    (coverage of decision 12, accepted by the manager) the drag
#           stands still, `sbarhive drop`, and the drag moves to P =
#           (400, 500) about 100 ms after the first frame, inside the
#           300 ms window: "hive anchor: none", then one move to the drag
#           position, "hive anchor" within 1 px of P, exactly two
#           wl_subsurface.set_position, (444, 219) then (204, 319), the
#           catcher region the hive rectangle after that first position
#           (layer.sh mode 0), and the drop on the WEBP cell writes the
#           file. The first enter on the catcher must fall after the first
#           frame and within 300 ms of it, or the show did not test the
#           window (a setup FAIL).
# S-019f (AC14, AC15): show 13 drags thumbed.png, whose thumbnail (the
# "photo" picture of tests/overlay/thumbs.py, sky 70 130 200) is seeded in
# the sandbox's XDG_CACHE_HOME, and holds it over the centre cell: a shot
# then has 200 or more sky pixels in the 48 x 48 box around the hive's
# centre (no format code or count is drawn over a thumbnail of one file).
# Every show's "hive preview" lines are checked by preview_checks (lib.sh):
# "icon image" for the PNGs, "icon pdf" for show 5, "thumb W H" for 13.
# S-019e (AC11 to AC13, AC17 to AC19, amended 2026-10-21), after those
# shows and their checks, with `sbarhive drop --hold` (the press) and
# `sbarhive drop --release` (the release); each command must exit 0 within
# 200 ms, or the cycle's other checks cannot pass (they would pass for a
# rejected option). Times are from a UTC stamp taken just before the press
# to the daemon trace's stamps:
#   hold-over    a drag, the press, the hive shows (default threshold
#                400 ms), the drag enters it and goes over the WEBP cell,
#                leaves the hive (decision 3: it stays while the key is
#                held), comes back over WEBP, then the release: the layer
#                surface is destroyed within 500 ms of it, and the drop
#                after it writes no file;
#   tap          the press, the release 200 ms after it: no
#                get_layer_surface and no get_xdg_surface within 1000 ms;
#   held         the press with no release: no surface by 350 ms, exactly
#                one get_layer_surface, after 350 ms and by 550 ms; then the
#                release closes the waiting hive within 500 ms;
#   lone release `sbarhive drop --release` with no press (AC19): exit 0, no
#                surface within 1000 ms;
#   hold_ms      `hold_ms = 1000` written to the sandbox's config.toml with
#                the daemon running (AC18): the next press shows after
#                950 ms and by 1150 ms, then the file is removed.
# Every show is also checked by tests/drop/layer.sh (S-019 AC8, AC9).
# Daemon lines (SBARHIVE_DEBUG_FOCUS): "hive anchor: X Y" or
# "hive anchor: none" before each placement's "hive surface: layer X Y W H";
# the drop-mode move to the first position prints both again.
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
  # trace_ms and the other trace helpers, used during the shows (S-019).
  # shellcheck source=layer.sh
  source "$HERE/layer.sh"
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
  if [ "$MODE" = shortcut ]; then
    make_shortcut_wrap
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
  # rectangle comes from the daemon's last "hive surface: layer X Y W H"
  # line after the last get_layer_surface (S-018 AC11; a drop-mode move to
  # the first position prints another, S-019 AC8), and the hive is shown
  # while there are more get_layer_surface requests than layer surface
  # destroys in the trace.
  hive_geo() {
    local shown gone gl
    shown=$(grep -cE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$DLOG")
    gone=$(grep -cE -- '-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\(' "$DLOG")
    [ "$shown" -gt "$gone" ] || return 0
    gl=$(grep -nE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$DLOG" | tail -1 | cut -d: -f1)
    tail -n +"$gl" "$DLOG" | grep -E '^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$' | tail -1 | cut -d' ' -f4-
  }

  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  "$INPUT" <"$f" >"$log-input.log" 2>&1 &
  exec 3>"$f"
  ptr_move() { echo "move $1 $2" >&3; }
  ptr_down() { echo "down" >&3; }
  ptr_up() { echo "up" >&3; }
  shift_down() { echo "keydown 42" >&3; }
  shift_up() { echo "keyup 42" >&3; }
  # S-019d: the first drag position moves a centred drop hive once.
  ANCHOR_SETTLE=1
  # jitter X Y MS: 1 px drag motion around (X, Y) every 10 ms for MS ms,
  # queued on the input client, so it runs while the caller goes on.
  jitter() {
    local i dx=1 dy=1
    [ "$1" -ge 1279 ] && dx=-1
    [ "$2" -ge 799 ] && dy=-1
    for ((i = 0; i < $3 / 10; i++)); do
      if ((i % 2)); then echo "move $(($1 + dx)) $(($2 + dy))"; else echo "move $1 $2"; fi
      echo "sleep 10"
    done >&3
  }

  W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  local i
  for i in one two three four; do cp "$ROOT/tests/fixtures/images/alpha.png" "$W/photo $i.png"; done
  cp "$ROOT/tests/fixtures/docs/one-page.pdf" "$W/report five.pdf"
  # S-019 AC15: a PNG with a thumbnail in the sandbox cache (no spaces in
  # its path: thumbs.py writes the plain file:// URI).
  THUMBED="$W/thumbed.png"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$THUMBED"
  python3 -I "$ROOT/tests/overlay/thumbs.py" "$THUMBED" "$XDG_CACHE_HOME" large 128 128 photo >"$log-thumb.log" 2>&1 \
    || echo "INFO S-019 AC15: thumbs.py failed: $(cat "$log-thumb.log")"
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
  local geo13=""
  on_centre() {
    [ "$1" = 13 ] || return 0
    geo13="$2 $3 $4 $5"
    shoot thumb "$2" "$3" "$4" "$5"
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

  # S-019 AC15: show 13's shot over the centre cell has the seeded
  # thumbnail's sky (70 130 200) in the 48 x 48 box around the hive centre.
  thumb_checks() {
    local gx gy gw gh sky=""
    read -r gx gy gw gh <<<"${geo13:-0 0 0 0}"
    local t="$OUT_LOG-shot-thumb.png"
    if [ -s "$t" ] && [ "$gw" -gt 0 ]; then
      sky=$(python3 -I "$px" rgbcount "$t" $((gx + gw / 2 - 24)) $((gy + gh / 2 - 24)) 48 48 4682c8 8)
      python3 -I "$px" crop "$t" "$gx" "$gy" "$gw" "$gh" "$OUT/drop-en-light-thumb.png"
    fi
    echo "INFO S-019 AC15 show 13: hive ${geo13:-not over the centre}, sky pixels in the centre box ${sky:-none} (crop: $OUT/drop-en-light-thumb.png)"
    check "S-019 AC15 show 13: over the centre cell after enter, 200 or more pixels of the seeded thumbnail's colour in the centre" \
      '[ "${sky:-0}" -ge 200 ]'
  }

  # anchor_cycle N KIND X Y: one S-019 AC10 show with the drag held at P.
  near() { awk -v a="$1" -v b="$2" -v t="$3" 'BEGIN { d = a - b; if (d < 0) d = -d; exit !(a != "" && b != "" && d <= t) }'; }
  AC10_WROTE=0
  anchor_cycle() {
    local n=$1 kind=$2 x=$3 y=$4
    local file="$W/held $n.png" base="$W/held $n"
    cp "$ROOT/tests/fixtures/images/alpha.png" "$file"
    echo "== show $n: $(basename "$file"), drag held at ($x, $y), $kind"
    printf '%s\n' "$file" >"$NEXT"
    MARKS+=("$(wc -l <"$DLOG")")
    EXPECT+=("$(action_ids "$file")")
    PREVIEW_WANT+=("$(preview_want "$file")")
    local m0=${MARKS[-1]} smark; smark=$(wc -l <"$SLOG")
    drag_past_threshold
    case "$kind" in
      jitter)
        # The motion starts before the command and lasts 600 ms (AC10).
        jitter "$x" "$y" 600
        "$BIN/sbarhive" drop >>"$OUT_LOG-cli.log" 2>&1
        ;;
      still)
        "$BIN/sbarhive" drop >>"$OUT_LOG-cli.log" 2>&1
        sleep 1
        jitter "$x" "$y" 600
        ;;
      at)
        ptr_move "$x" "$y"; sleep 0.5
        "$BIN/sbarhive" drop --at "$x,$y" >>"$OUT_LOG-cli.log" 2>&1
        ;;
      late)
        # Queued on the input client as soon as the command returns, so no
        # polling delay: the first frame comes some 60 to 80 ms after it
        # (50 ms anchor wait, KWin), the move LATE_MS after it.
        "$BIN/sbarhive" drop >>"$OUT_LOG-cli.log" 2>&1
        echo "sleep ${LATE_MS:-170}" >&3
        jitter "$x" "$y" 600
        ;;
    esac
    sleep 0.8
    local geo=""
    geo=$(wait_geo 8) || true
    local seg al sets nset
    seg=$(sed -n "$((m0 + 1)),\$p" "$DLOG")
    al=$(grep -E '^hive anchor: ' <<<"$seg")
    sets=$(grep -E -- '-> wl_subsurface[#@][0-9]+\.set_position\(' <<<"$seg")
    nset=$(grep -c . <<<"$sets")
    local ax ay lx ly
    read -r _ _ ax ay <<<"$(grep -vE '^hive anchor: none$' <<<"$al" | head -1)"
    read -r lx ly < <(tail -1 <<<"$sets" | sed -nE 's/.*set_position\((-?[0-9]+), (-?[0-9]+)\).*/\1 \2/p') || true
    echo "INFO S-019 show $n ($kind at $x $y): hive ${geo:-not shown}; anchor lines: $(tr '\n' '|' <<<"$al"); set_position x$nset: $(sed -E 's/.*set_position//' <<<"$sets" | tr '\n' ' ')"
    case "$kind" in
      jitter | at)
        check "S-019 AC10 show $n ($kind): one \"hive anchor\" line, within 1 px of ($x, $y), and no \"hive anchor: none\"" \
          '[ "$(grep -c . <<<"$al")" -eq 1 ] && near "$ax" "$x" 1 && near "$ay" "$y" 1'
        check "S-019 AC5/AC10 show $n ($kind): exactly one wl_subsurface.set_position (no move after the first frame)" '[ "$nset" -eq 1 ]'
        ;;
      late)
        # Decision 12 coverage: the first position inside the 300 ms window
        # moves the centred hive once, to the drag (AC8).
        local cat child fr en dte=none
        cat=$(grep -oE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
        child=$(grep -oE -- '-> wl_subcompositor[#@][0-9]+\.get_subsurface\(new id wl_subsurface[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
        if [ -n "$cat" ] && [ -n "$child" ]; then
          fr=$(grep -E -- "-> ${child}\.attach\(wl_buffer" <<<"$seg" | head -1)
          en=$(grep -E -- "^\[[0-9:.]+\] (wl_pointer|wl_data_device)[#@][0-9]+\.enter\([0-9]+, ${cat}, " <<<"$seg" | head -1)
          [ -n "$fr" ] && [ -n "$en" ] && dte=$(awk -v a="$(trace_ms "$fr")" -v b="$(trace_ms "$en")" 'BEGIN { printf "%.1f\n", b - a }')
        fi
        echo "INFO S-019 show $n (late): the first enter on the catcher ${dte} ms after the first frame (LATE_MS ${LATE_MS:-170})"
        check "S-019 decision 12 show $n (late) setup: the drag's first position on the catcher comes after the first frame and within its 300 ms window" \
          'awk -v d="$dte" "BEGIN { exit !(d != \"none\" && d + 0 > 0 && d + 0 < 300) }"'
        check "S-019 AC8 show $n (late): \"hive anchor: none\" first, then one \"hive anchor\" line within 1 px of ($x, $y)" \
          '[ "$(grep -c . <<<"$al")" -eq 2 ] && [ "$(head -1 <<<"$al")" = "hive anchor: none" ] && near "$ax" "$x" 1 && near "$ay" "$y" 1'
        check "S-019 AC8 show $n (late): exactly two wl_subsurface.set_position, the middle (444, 219) then (204, 319) within 1 px (one move, then none)" \
          '[ "$nset" -eq 2 ] && grep -qE "set_position\(444, 219\)" <<<"$(head -1 <<<"$sets")" && near "$lx" 204 1 && near "$ly" 319 1'
        ;;
      still)
        # Manager decision 12: no position within 300 ms, so the hive stays
        # in the middle; the later drag motion at P does not move it.
        check "S-019 AC10 show $n (still, decision 12): exactly one anchor line, \"hive anchor: none\" (no move to ($x, $y))" \
          '[ "$al" = "hive anchor: none" ]'
        check "S-019 AC10 show $n (still, decision 12): exactly one wl_subsurface.set_position, in the middle (444, 219)" \
          '[ "$nset" -eq 1 ] && grep -qE "set_position\(444, 219\)" <<<"$sets"'
        ;;
    esac
    if [ "$x $y" = "400 500" ] && [ "$kind" != still ] && [ "$kind" != late ]; then
      check "S-019 AC10 show $n ($kind): the hive ends at (204, 319) within 1 px" 'near "$lx" 204 1 && near "$ly" 319 1'
    fi
    if [ -n "$geo" ] && [ "$x $y" = "400 500" ]; then
      # shellcheck disable=SC2086
      move_to_webp $geo
      ptr_up; sleep 0.3
      if wait_file "$base.webp" 5 && [ "$(file --mime-type -b "$base.webp" 2>/dev/null)" = image/webp ]; then
        # Counted only for a hive anchored at P: (204, 319).
        [ "$kind" = jitter ] && near "$lx" 204 1 && near "$ly" 319 1 && AC10_WROTE=$((AC10_WROTE + 1))
        echo "INFO S-019 show $n: $(basename "$base").webp written"
      fi
      [ "$kind" = at ] && check "S-019 decision 1 show $n (--at): dropped on the WEBP cell, $(basename "$base").webp is written within 5 s" \
        '[ "$(file --mime-type -b "$base.webp" 2>/dev/null)" = image/webp ]'
      [ "$kind" = still ] && check "S-019 decision 12 show $n (still): the drag moved into the centred hive and dropped on its WEBP cell writes $(basename "$base").webp within 5 s" \
        '[ "$(file --mime-type -b "$base.webp" 2>/dev/null)" = image/webp ]'
      [ "$kind" = late ] && check "S-019 decision 12 show $n (late): dropped on the WEBP cell of the moved hive, $(basename "$base").webp is written within 5 s" \
        '[ "$(file --mime-type -b "$base.webp" 2>/dev/null)" = image/webp ]'
    else
      # Released over the centre cell: refused, no file.
      ptr_up; sleep 0.3
    fi
    wait_gone 5 || { ptr_move 1000 700; sleep 0.5; wait_gone 5; } || true
    sleep 1
  }

  # S-019e: the long press (AC11 to AC13, AC17 to AC19). Not in MARKS: two
  # of these cycles show no hive at all.
  # cli_ms CMD...: runs `sbarhive CMD...`, sets CLI_RC and CLI_MS.
  cli_ms() {
    local t0; t0=$(now_ms)
    "$BIN/sbarhive" "$@" >>"$OUT_LOG-cli.log" 2>&1
    CLI_RC=$?
    CLI_MS=$(( $(now_ms) - t0 ))
  }
  # ms_after STAMP LINE: ms from a "HH:MM:SS.ffffff" UTC stamp to a trace line.
  ms_after() { awk -v a="$(tod "$1")" -v b="$(tod "$(grep -oE '^\[[0-9:.]+\]' <<<"$2")")" 'BEGIN { if (b == "") { print "none"; exit } d = b - a; if (d < -43200) d += 86400; printf "%.1f\n", d * 1000 }'; }
  GLS_RE='-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\('
  XDG_RE='-> xdg_wm_base[#@][0-9]+\.get_xdg_surface\('
  DESTROY_RE='-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\('
  # press: the press, with its stamp in T_PRESS and the trace mark in M_PRESS.
  press() {
    M_PRESS=$(wc -l <"$DLOG"); T_PRESS=$(date -u +%T.%6N)
    cli_ms drop --hold
    P_RC=$CLI_RC; P_MS=$CLI_MS
  }
  release() {
    M_REL=$(wc -l <"$DLOG"); T_REL=$(date -u +%T.%6N)
    cli_ms drop --release
    R_RC=$CLI_RC; R_MS=$CLI_MS
  }
  # surfaces_by MARK STAMP MS: get_layer_surface and get_xdg_surface after
  # MARK whose trace stamp is at most MS after STAMP: "LAYERS XDGS FIRST",
  # FIRST the ms of the first get_layer_surface (or none).
  surfaces_by() {
    local seg; seg=$(tail -n +$(($1 + 1)) "$DLOG")
    local l x first="none" line d
    l=0; x=0
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      d=$(ms_after "$2" "$line")
      awk -v d="$d" -v m="$3" 'BEGIN { exit !(d != "none" && d + 0 <= m) }' || continue
      if grep -qE -- "$GLS_RE" <<<"$line"; then l=$((l + 1)); [ "$first" = none ] && first=$d; else x=$((x + 1)); fi
    done < <(grep -E -- "$GLS_RE|$XDG_RE" <<<"$seg")
    echo "$l $x $first"
  }
  # destroyed_after MARK STAMP: ms from STAMP to the first layer destroy after MARK.
  destroyed_after() {
    local line; line=$(tail -n +$(($1 + 1)) "$DLOG" | grep -m1 -E -- "$DESTROY_RE")
    [ -n "$line" ] && ms_after "$2" "$line" || echo none
  }
  in_ms() { awk -v d="$1" -v lo="$2" -v hi="$3" 'BEGIN { exit !(d != "none" && d + 0 > lo && d + 0 <= hi) }'; }
  cli_ok() { [ "$1" = 0 ] && [ "$2" -le 200 ]; }

  hold_cycles() {
    local l x first d before
    # held: the press with no release.
    echo "== S-019e held: \`sbarhive drop --hold\`, no release"
    press; sleep 1.2
    read -r l x first <<<"$(surfaces_by "$M_PRESS" "$T_PRESS" 350)"
    local l350=$l x350=$x
    read -r l x first <<<"$(surfaces_by "$M_PRESS" "$T_PRESS" 1150)"
    echo "INFO S-019e held: press exit $P_RC in $P_MS ms; surfaces by 350 ms: layer $l350 xdg $x350; by 1150 ms: layer $l xdg $x, the first ${first} ms after the press"
    check "S-019 AC11 held: \`sbarhive drop --hold\` exits 0 within 200 ms (the threshold is the daemon's)" 'cli_ok "$P_RC" "$P_MS"'
    check "S-019 AC13 held: no surface at 350 ms from the press, then exactly one get_layer_surface, after 350 ms and by 550 ms" \
      'cli_ok "$P_RC" "$P_MS" && [ "$l350" -eq 0 ] && [ "$x350" -eq 0 ] && [ "$l" -eq 1 ] && [ "$x" -eq 0 ] && in_ms "$first" 350 550'
    release; sleep 0.8
    d=$(destroyed_after "$M_REL" "$T_REL")
    echo "INFO S-019e held: release exit $R_RC in $R_MS ms, layer destroyed ${d} ms after it"
    check "S-019 AC12 held: the release closes the waiting hold show within 500 ms (zwlr_layer_surface_v1.destroy)" \
      'cli_ok "$R_RC" "$R_MS" && [ "$l" -eq 1 ] && in_ms "$d" -1 500'
    wait_gone 12 || true; sleep 0.5

    # tap: the release 200 ms after the press.
    echo "== S-019e tap: press, release at 200 ms"
    press
    sleep "$(awk -v m="$P_MS" 'BEGIN { s = (200 - m) / 1000; if (s < 0) s = 0; printf "%.3f\n", s }')"
    release
    sleep 1
    read -r l x first <<<"$(surfaces_by "$M_PRESS" "$T_PRESS" 1000)"
    echo "INFO S-019e tap: press exit $P_RC in $P_MS ms, release exit $R_RC in $R_MS ms, $(awk -v a="$(tod "$T_PRESS")" -v b="$(tod "$T_REL")" 'BEGIN { printf "%.0f", (b - a) * 1000 }') ms apart; surfaces within 1000 ms: layer $l xdg $x"
    check "S-019 AC13 tap: press and release 200 ms apart, both exit 0 within 200 ms, and no get_layer_surface or get_xdg_surface within 1000 ms of the press" \
      'cli_ok "$P_RC" "$P_MS" && cli_ok "$R_RC" "$R_MS" && [ "$l" -eq 0 ] && [ "$x" -eq 0 ]'
    wait_gone 12 || true

    # hold-over: a drag, the press, the hive, over WEBP, out and back, release.
    echo "== S-019e hold-over: drag, press, over WEBP, leave, back, release, drop"
    local file="$W/held over.png" geo
    cp "$ROOT/tests/fixtures/images/alpha.png" "$file"
    printf '%s\n' "$file" >"$NEXT"
    before=$(find "$W" -type f | sort)
    drag_past_threshold
    press
    geo=$(wait_geo 3) || true
    local stayed=no
    if [ -n "$geo" ]; then
      # shellcheck disable=SC2086
      geo=$(settle_at_centre $geo)
      # shellcheck disable=SC2086
      move_to_webp $geo
      # Decision 3: out of the hive (the catcher's region is the hive
      # rectangle now), the key still held: it stays.
      ptr_move 1200 760; sleep 0.6
      [ -n "$(hive_geo)" ] && stayed=yes
      # shellcheck disable=SC2086
      move_to_webp $geo
    fi
    release; sleep 0.7
    d=$(destroyed_after "$M_REL" "$T_REL")
    ptr_up; sleep 3
    local after; after=$(find "$W" -type f | sort)
    echo "INFO S-019e hold-over: press exit $P_RC in $P_MS ms, hive ${geo:-not shown}, stayed after the leave $stayed, release exit $R_RC in $R_MS ms, layer destroyed ${d} ms after it, new files: $(comm -13 <(echo "$before") <(echo "$after") | tr '\n' ' ')"
    check "S-019 AC13 hold-over: shown after the press (exit 0 within 200 ms), then over the WEBP cell" 'cli_ok "$P_RC" "$P_MS" && [ -n "$geo" ]'
    check "S-019 decision 3 hold-over: the drag leaving the hive does not close it while the key is held" '[ "$stayed" = yes ]'
    check "S-019 AC13 hold-over: the release over the WEBP cell destroys the layer surface within 500 ms" \
      'cli_ok "$R_RC" "$R_MS" && in_ms "$d" -1 500'
    check "S-019 AC13 hold-over: the drop after the release writes no file" '[ -n "$geo" ] && [ "$after" = "$before" ]'
    wait_gone 12 || true; sleep 0.5

    # lone release (AC19): no press before.
    echo "== S-019e lone release"
    M_PRESS=$(wc -l <"$DLOG"); T_PRESS=$(date -u +%T.%6N)
    release; sleep 1
    read -r l x first <<<"$(surfaces_by "$M_PRESS" "$T_PRESS" 1000)"
    echo "INFO S-019e lone release: exit $R_RC in $R_MS ms, surfaces within 1000 ms: layer $l xdg $x"
    check "S-019 AC19 lone release: \`sbarhive drop --release\` with no press exits 0 within 200 ms, no surface" \
      'cli_ok "$R_RC" "$R_MS" && [ "$l" -eq 0 ] && [ "$x" -eq 0 ]'

    # hold_ms changed in config.toml with the daemon running (AC18).
    echo "== S-019e hold_ms = 1000 in config.toml"
    mkdir -p "$XDG_CONFIG_HOME/sbarhive"
    printf 'hold_ms = 1000\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"
    sleep 1
    press; sleep 1.6
    read -r l x first <<<"$(surfaces_by "$M_PRESS" "$T_PRESS" 1500)"
    local l950; read -r l950 _ _ <<<"$(surfaces_by "$M_PRESS" "$T_PRESS" 950)"
    release; sleep 0.8
    rm -f "$XDG_CONFIG_HOME/sbarhive/config.toml"
    echo "INFO S-019e hold_ms 1000: press exit $P_RC in $P_MS ms; surfaces by 950 ms: layer $l950; by 1500 ms: layer $l xdg $x, the first ${first} ms after the press"
    check "S-019 AC18 hold_ms: hold_ms = 1000 written with the daemon running applies at the next press (one get_layer_surface after 950 ms and by 1150 ms)" \
      'cli_ok "$P_RC" "$P_MS" && [ "$l950" -eq 0 ] && [ "$l" -eq 1 ] && [ "$x" -eq 0 ] && in_ms "$first" 950 1150'
    wait_gone 12 || true
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
  else
  cycle 1 "$W/photo one.png" webp 0
  cycle 2 "$W/photo two.png" corner 0
  cycle 3 "$W/photo three.png" webp 1
  cycle 4 "$W/photo four.png" centre 0
  cycle 5 "$W/report five.pdf" corner 0
  # S-019 AC10 and decision 1.
  # layer.sh mode per show (decision 12): 1 to 5 and 10 still, 6 to 9 at
  # the first position (0), 11 --at (1).
  local MODES=(still still still still still 0 0 0 0 still 1 0 still)
  anchor_cycle 6 jitter 400 500
  anchor_cycle 7 jitter 400 500
  anchor_cycle 8 jitter 400 500
  anchor_cycle 9 jitter 100 700
  anchor_cycle 10 still 400 500
  anchor_cycle 11 at 400 500
  check "S-019 AC10: 3 of 3 drags held at (400, 500), the hive at (204, 319), dropped on the WEBP cell wrote the file" '[ "$AC10_WROTE" -eq 3 ]'
  # Decision 12 coverage: a real move inside the 300 ms window.
  anchor_cycle 12 late 400 500
  # S-019 AC15: the seeded thumbnail, held over the centre cell, refused.
  cycle 13 "$THUMBED" centre 0
  protocol_checks
  enter_cells_checks
  preview_checks
  shot_checks
  thumb_checks
  # S-018d (AC13, AC14): the layer path's drop checks, per show.
  # shellcheck source=layer.sh
  source "$HERE/layer.sh"
  for ((i = 0; i + 1 < ${#MARKS[@]}; i++)); do
    layer_drop_show $((i + 1)) "${MARKS[i]}" "${MARKS[i + 1]}" 1 "${MODES[i]:-0}"
  done
  hold_cycles
  fi
  summary

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
