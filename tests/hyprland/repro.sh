#!/usr/bin/env bash
# The hive on Hyprland, in a private nested Hyprland (aquamarine Wayland
# backend) inside a private headless kwin_wayland --virtual. Reproduces the
# owner's report on Hyprland 0.56 with SBARHIVE_SURFACE=layer: the overlay
# shows, but no `hive focus` line ever comes and input feels frozen.
# Input comes from org_kde_kwin_fake_input on the outer KWin
# (tests/drop/kwin-input), so it reaches Hyprland as a real seat keyboard and
# pointer through its Wayland backend. Another client (keys.py, a full screen
# GTK window inside Hyprland) logs the keys and clicks it gets.
# Modes:
#   (default) layer    human PASS/FAIL lines on stdout, plus INFO lines with
#                      the measured numbers (latency, CPU, drop mode).
#   window             the same checks with SBARHIVE_SURFACE=window.
#   --selftest         only start the nested Hyprland, check that it takes a
#                      client (keys.py maps), tear down; exit 0 on success.
#   --json             the checks; human lines go to stderr and stdout gets
#                      exactly one line: {"total": N, "failed": F, "skipped": S}.
#   --json --quasimode adds the S-019 checks (a screen reader flag without a
#                      running reader keeps the layer, key release hides, hive
#                      near the pointer, same output as the pointer, adaptive
#                      shape);
#                      not implemented yet, so each counts as failed.
# Checks (layer or window): surface shown, keyboard focus received, show
# latency under 150 ms, Esc closes, click outside closes, input reaches the
# other client after close, a click outside a drop show passes through;
# then drop mode mid-drag, as the owner's Super+Z
# and Alt-hold bindings run it (a GTK source from tests/drop/source.py drags
# a PNG): the hive gets wl_data_device.motion, a release over the WEBP cell
# writes the file, the drop timeout and Esc mid-drag both end with the hive
# gone and pointer input working. In a drop show with no drag (S-020 AC4) a
# click at (40, 40), outside the hive, reaches keys.py once and the hive
# stays until the drop timeout. Exit 0 only when none failed.
# On Hyprland 0.56.2 the layer path gets no drag motion: Hyprland makes no
# view for a subsurface of a layer surface, so its drag motion listener finds
# none for the hive subsurface and sends nothing (src/protocols/core/
# DataDevice.cpp, mouse.move listener in initiateDrag).
# HYPR_SCALE sets the nested output scale (default 1.333333, the owner's laptop panel).
# HYPR_DROP_VIA=exec runs the mid-drag `sbarhive drop` through Hyprland's
# exec dispatcher, as the owner's Super+Z binding does.
# Never touches the live desktop: everything runs with a throwaway HOME, XDG
# dirs, runtime dir and session bus, with DISPLAY, WAYLAND_DISPLAY,
# HYPRLAND_INSTANCE_SIGNATURE and the desktop names unset before
# dbus-run-session. LIBSEAT_BACKEND=seatd with no seatd socket makes
# aquamarine's DRM backend fail at once, so logind is never asked for the
# live session's devices.
# Usage: tests/hyprland/repro.sh [--selftest|--json [--quasimode]] [layer|window] [BIN_DIR]
# BIN_DIR defaults to target/release; ~/.local/bin tests the installed build.
# shellcheck disable=SC2016,SC2034,SC2329
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
SELFTEST=0 JSON=0 QUASI=0 SURFACE=layer
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in
    --selftest) SELFTEST=1 ;;
    --json) JSON=1 ;;
    --quasimode) QUASI=1 ;;
    layer|window) SURFACE=$a ;;
    *) BIN="$a" ;;
  esac
done
INPUT="$ROOT/tests/drop/kwin-input/target/release/kwin-input"
need() { echo "$1" >&2; [ "$JSON" = 1 ] && echo '{"total": 0, "failed": 1, "skipped": 0}'; exit 2; }
if ! command -v Hyprland >/dev/null || ! command -v kwin_wayland >/dev/null; then need "needs Hyprland and kwin_wayland"; fi
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || need "build first: cargo build --release"
[ -x "$INPUT" ] || need "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"
OUT="${HYPR_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT INPUT SELFTEST JSON QUASI SURFACE
export HYPR_SCALE="${HYPR_SCALE:-1.333333}"

inner() {
  set -u
  local log="$OUT/hypr-$SURFACE"
  [ "$SELFTEST" = 1 ] && log="$OUT/hypr-selftest"
  local TOTAL=0 FAILS=0
  # Shared drop helpers (webp_offset, drag_past_threshold, move_to_webp),
  # which drive the pointer through ptr_move, ptr_down and ptr_up, defined
  # below. lib.sh defines its own check(); the one below replaces it so
  # that TOTAL counts every check.
  # shellcheck source-path=SCRIPTDIR source=../drop/lib.sh
  source "$ROOT/tests/drop/lib.sh"
  # Human lines go to stdout here; the caller sends them to stderr in --json.
  say() { echo "$*"; }
  check() { TOTAL=$((TOTAL + 1)); if eval "$2"; then say "PASS $1"; else say "FAIL $1"; FAILS=$((FAILS + 1)); fi; }
  now_us() { echo "${EPOCHREALTIME/./}"; }
  pids=()
  finish() {
    exec 3>&- 2>/dev/null
    [ -n "${d:-}" ] && { "$BIN/sbarhive" quit >/dev/null 2>&1; sleep 0.3; }
    kill "${pids[@]}" 2>/dev/null
    sleep 0.5
    kill -9 "${pids[@]}" 2>/dev/null
    wait 2>/dev/null
    pids=()
  }
  # Every private process goes, whatever way this function ends.
  trap finish EXIT
  trap '' PIPE

  printf '[Windows]\nFocusStealingPreventionLevel=0\n' >"$XDG_CONFIG_HOME/kwinrc"
  KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 \
    kwin_wayland --virtual --no-lockscreen --socket sbh-kwin --width 1600 --height 1000 >"$log-kwin.log" 2>&1 &
  pids+=("$!")
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/sbh-kwin" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/sbh-kwin" ] || { say "FAIL kwin did not start"; finish; return 1; }
  sleep 1

  mkdir -p "$XDG_CONFIG_HOME/hypr"
  # No stay_focused rule for the hive window: the owner's live config has
  # one, but integrations/linux/hyprland/README.md does not recommend it, and
  # it keeps the window path's hive open on a click outside by construction.
  cat >"$XDG_CONFIG_HOME/hypr/sbh.lua" <<LUA
hl.monitor({ output = "", mode = "1280x720", position = "0x0", scale = "$HYPR_SCALE" })
hl.config({
  misc = { disable_hyprland_logo = true, disable_splash_rendering = true, disable_autoreload = true },
  animations = { enabled = false },
  input = { follow_mouse = 1 },
})
-- The owner's rules for the hive window (window path), so it floats centred.
local sbarhive = { class = [[^(io\\.github\\.m7mmad_omar\\.SbarHive)$]], title = [[^(SbarHive)$]] }
hl.window_rule({ match = sbarhive, float = true })
hl.window_rule({ match = sbarhive, center = true })
hl.window_rule({ match = sbarhive, pin = true })
hl.window_rule({ match = sbarhive, no_anim = true })
hl.window_rule({ match = sbarhive, border_size = 0 })
hl.layer_rule({ match = { namespace = "^sbarhive$" }, no_anim = true })
LUA
  WAYLAND_DISPLAY=sbh-kwin LIBSEAT_BACKEND=seatd HYPRLAND_NO_SD_NOTIFY=1 HYPRLAND_NO_CRASHREPORTER=1 \
    Hyprland --config "$XDG_CONFIG_HOME/hypr/sbh.lua" >"$log-hyprland.log" 2>&1 &
  local hy=$!
  pids+=("$hy")
  local sig="" wl=""
  for _ in $(seq 100); do
    sig=$(find "$XDG_RUNTIME_DIR/hypr" -mindepth 1 -maxdepth 1 -printf '%f\n' 2>/dev/null | head -1)
    if [ -n "$sig" ]; then
      wl=$(HYPRLAND_INSTANCE_SIGNATURE=$sig hyprctl instances -j 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)[0]["wl_socket"])' 2>/dev/null)
      [ -n "$wl" ] && HYPRLAND_INSTANCE_SIGNATURE=$sig hyprctl monitors 2>/dev/null | grep -qE '^\s+[1-9][0-9]*x[0-9]+@' && break
    fi
    sleep 0.2
  done
  [ -n "$wl" ] && [ -S "$XDG_RUNTIME_DIR/$wl" ] || { say "FAIL nested Hyprland did not start"; tail -5 "$log-hyprland.log" >&2; finish; return 1; }
  export HYPRLAND_INSTANCE_SIGNATURE=$sig WAYLAND_DISPLAY=$wl
  # A config error makes Hyprland fall back to its defaults (other scale).
  local cerr; cerr=$(hyprctl configerrors 2>/dev/null | grep -v '^\s*$')
  [ -n "$cerr" ] && { say "FAIL private Hyprland config: $cerr"; finish; return 1; }
  # The owner's session names Hyprland; only private processes see these.
  export XDG_CURRENT_DESKTOP=Hyprland XDG_SESSION_TYPE=wayland
  say "INFO nested $(Hyprland --version | head -1 | cut -d' ' -f1-2) on $wl, scale $HYPR_SCALE, output $(hyprctl monitors | grep -m1 -oE '[0-9]+x[0-9]+@[0-9.]+')"

  local KLOG="$log-keys.log" kp
  # start_keys: a new keys.py (the other client), logging to KLOG; sets kp.
  start_keys() {
    GDK_BACKEND=wayland GTK_A11Y=none python3 -I "$HERE/keys.py" >"$KLOG" 2>&1 &
    kp=$!
    pids+=("$kp")
  }
  start_keys
  local mapped=0
  for _ in $(seq 50); do hyprctl clients 2>/dev/null | grep -q 'class: org.sbarhive.HyprKeys' && { mapped=1; break; }; sleep 0.2; done

  if [ "$SELFTEST" = 1 ]; then
    check "selftest: nested Hyprland takes a client" '[ "$mapped" = 1 ]'
    finish
    [ "$FAILS" -eq 0 ]
    return
  fi

  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  # Fake input goes to the outer KWin, which forwards it to the nested output.
  WAYLAND_DISPLAY=sbh-kwin "$INPUT" <"$f" >"$log-input.log" 2>&1 &
  pids+=("$!")
  exec 3>"$f"
  key() { echo "keydown $1" >&3; echo "sleep 40" >&3; echo "keyup $1" >&3; }

  # The nested output is a window in KWin: learn KWin position = o + s * hypr.
  # Computed once, in millipixels (ox, oy) and millionths (sc), so that
  # move() is integer arithmetic with no process per move.
  local ox oy sc
  cal() { echo "move $1 $2" >&3; sleep 0.4; hyprctl cursorpos | tr -d ' '; }
  local p1 p2; p1=$(cal 500 400); p2=$(cal 900 700)
  read -r ox oy sc < <(awk -v a="$p1" -v b="$p2" 'BEGIN {
    split(a, p, ","); split(b, q, ",")
    s = (q[1] != p[1]) ? 400 / (q[1] - p[1]) : 1
    printf "%d %d %d\n", (500 - s * p[1]) * 1000, (400 - s * p[2]) * 1000, s * 1000000 }')
  : "${ox:=0}" "${oy:=0}" "${sc:=1000000}"
  say "INFO pointer map: kwin = ($ox, $oy) / 1000 + $sc / 1000000 * hypr (from $p1 and $p2)"
  move() { echo "move $(( (ox * 1000 + sc * $1 + 500000) / 1000000 )) $(( (oy * 1000 + sc * $2 + 500000) / 1000000 ))" >&3; }
  # The lib.sh caller contract, in Hyprland logical pixels.
  ptr_move() { move "$1" "$2"; }
  ptr_down() { echo "down" >&3; }
  ptr_up() { echo "up" >&3; }
  click() { move "$1" "$2"; echo "sleep 150" >&3; echo "down" >&3; echo "sleep 40" >&3; echo "up" >&3; }
  # Focus the other client with a click on it, then measure from there.
  # keys.py now and then maps but never takes input (seen 2 runs in ~15);
  # a fresh one always has, so restart it rather than fail the input checks.
  local tries; for tries in 1 2 3 4 5 6; do
    click 100 100; sleep 0.6
    grep -q '^keys: active True' "$KLOG" && break
    if [ "$tries" = 3 ]; then
      say "INFO keys.py took no input after 3 clicks; starting a new one"
      kill "$kp" 2>/dev/null; sleep 0.5
      start_keys
      sleep 2
    fi
  done
  grep -q '^keys: active True' "$KLOG" || say "INFO keys.py never became active; the input checks below may fail for that reason"

  local W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/photo.png"
  mkdir -p "$XDG_CONFIG_HOME/sbarhive"
  printf 'language = "en"\ntheme = "light"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"
  local DLOG="$log-daemon.log"
  : >"$DLOG"
  # start_daemon [WL]: WL=1 (the default) adds WAYLAND_DEBUG=1. Show 1, the
  # latency show, runs on a daemon without it, so the trace's writes are not
  # part of the measured time.
  local DAEMON_WL=1
  start_daemon() {
    DAEMON_WL=${1:-1}
    if [ "$DAEMON_WL" = 1 ]; then
      SBARHIVE_SURFACE=$SURFACE SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >>"$DLOG" 2>&1 &
    else
      SBARHIVE_SURFACE=$SURFACE SBARHIVE_DEBUG_FOCUS=1 "$BIN/sbarhived" >>"$DLOG" 2>&1 &
    fi
    d=$!
    pids+=("$d")
    for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
  }
  stop_daemon() {
    "$BIN/sbarhive" quit >/dev/null 2>&1
    timeout 3 tail --pid="$d" -f /dev/null 2>/dev/null || kill "$d" 2>/dev/null
    sleep 0.3
  }
  start_daemon 0
  # A hive left shown by a failed check would spoil the next ones: restart.
  reset_hive() {
    shown || return 0
    say "INFO hive still shown after $1; restarting the daemon"
    stop_daemon
    start_daemon "$DAEMON_WL"
  }

  # Shown: a layer named sbarhive, or the window titled exactly SbarHive.
  shown() {
    if [ "$SURFACE" = layer ]; then hyprctl layers 2>/dev/null | grep -q 'namespace: sbarhive'
    else hyprctl clients 2>/dev/null | grep -qE '^\s+title: SbarHive$'; fi
  }
  gone_within() { local i; for i in $(seq "$1"); do shown || return 0; sleep 0.1; done; ! shown; }
  after() { sed -n "$(($1 + 1)),\$p" "$DLOG" | grep -v '^\['; }
  cpu() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
  # cpu_pct SECONDS PID...: each PID's share of one core over one shared
  # interval (every stat file read, one sleep, every stat file read again).
  cpu_pct() {
    local t=$1 p a=() out=() i=0 hz; shift; hz=$(getconf CLK_TCK)
    for p in "$@"; do a+=("$(cpu "$p")"); done
    sleep "$t"
    for p in "$@"; do out+=("$(( ($(cpu "$p") - a[i]) * 100 / (hz * t) ))"); i=$((i + 1)); done
    echo "${out[*]}"
  }

  # geo: the shown hive's x y w h in Hyprland logical pixels.
  geo() {
    if [ "$SURFACE" = layer ]; then after "$MARK" | grep -m1 '^hive surface:' | cut -d' ' -f4-
    else
      local i; for i in $(seq 20); do
        hyprctl clients -j 2>/dev/null | python3 -I -c 'import json,sys
for c in json.load(sys.stdin):
    if c["title"] == "SbarHive": print(*c["at"], *c["size"]); break' && return; sleep 0.1
      done
    fi
  }
  # show CMD...: runs it, waits (up to 5 s) for the `hive surface` line by
  # following the log from MARK; LAT is in ms. The window path prints its
  # surface line once per build, so a later window show is seen by its
  # first focus line.
  local MARK LAT
  show() {
    MARK=$(wc -l <"$DLOG")
    local t0 pat="^hive surface:"
    [ "$SURFACE" = window ] && pat="^hive surface:|^hive focus: true$"
    t0=$(now_us)
    "$@" >>"$log-cli.log" 2>&1
    LAT=""
    # grep returns at the first match; the tail behind it ends at its next
    # write or at the timeout.
    if grep -m1 -qE "$pat" < <(timeout 5 tail -n +"$((MARK + 1))" -f "$DLOG"); then
      LAT=$(( ($(now_us) - t0) / 1000 ))
    fi
  }

  # Show 1: open, then Esc.
  show "$BIN/sbarhive" open "$W/photo.png"
  check "surface shown ($SURFACE)" 'after "$MARK" | grep -q "^hive surface: $SURFACE"'
  check "show latency under 150 ms (${LAT:-none} ms)" '[ -n "$LAT" ] && [ "$LAT" -lt 150 ]'
  sleep 0.5
  check "keyboard focus received (hive focus: true)" 'after "$MARK" | grep -q "^hive focus: true"'
  local k0; k0=$(wc -l <"$KLOG")
  local cd ch; read -r cd ch <<<"$(cpu_pct 2 "$d" "$hy")"
  say "INFO shown 2 s: sbarhived ${cd}% CPU, Hyprland ${ch}% CPU (one core = 100)"
  key 30; sleep 0.5
  say "INFO a key while shown reached the other client: $( [ "$(wc -l <"$KLOG")" -gt "$k0" ] && echo yes || echo no )"
  key 1
  check "Esc closes" 'gone_within 15'
  say "INFO hive focus: true lines this show: $(after "$MARK" | grep -c '^hive focus: true$')"
  reset_hive Esc
  # From here on the daemon traces its Wayland requests (WAYLAND_DEBUG=1).
  stop_daemon
  start_daemon 1

  # Show 2: open, then a click far from the centre (on the catcher, inside the smallest logical output).
  show "$BIN/sbarhive" open "$W/photo.png"
  sleep 0.5
  click 40 40
  check "click outside closes" 'gone_within 15'
  reset_hive "the click outside"

  # After the hive is gone the other client gets keys again.
  click 100 100; sleep 0.3
  k0=$(wc -l <"$KLOG")
  key 30; sleep 0.5
  check "input reaches another client after close" '[ "$(sed -n "$((k0 + 1)),\$p" "$KLOG" | grep -c "^keys: key")" -ge 1 ]'

  # Drop mode with no drag (informative numbers), then S-020 AC4: a click
  # at (40, 40), outside the hive rectangle, passes through to keys.py
  # (exactly one click) and the hive stays until the 10 s drop timeout.
  local dclicks=none dstay=no dgone=none
  if "$BIN/sbarhive" ping >/dev/null 2>&1; then
    local dt0; dt0=$(now_us)
    show "$BIN/sbarhive" drop
    sleep 0.5
    k0=$(wc -l <"$KLOG")
    key 30; sleep 0.5
    say "INFO drop: surface $(after "$MARK" | grep -m1 '^hive surface:' | cut -d' ' -f3 || true) in ${LAT:-none} ms, focus lines $(after "$MARK" | grep -c '^hive focus'), key reached other client: $( [ "$(wc -l <"$KLOG")" -gt "$k0" ] && echo yes || echo no ), sbarhived $(cpu_pct 2 "$d")% CPU"
    k0=$(wc -l <"$KLOG")
    local dpre=no; shown && dpre=yes
    click 40 40; sleep 0.6
    dclicks=$(sed -n "$((k0 + 1)),\$p" "$KLOG" | grep -c '^keys: click$')
    [ "$dpre" = yes ] && shown && dstay=yes
    local i; for i in $(seq 150); do shown || { dgone=$(( ($(now_us) - dt0) / 1000 )); break; }; sleep 0.1; done
    reset_hive "the drop show"
  fi
  say "INFO drop click at (40, 40): keys.py clicks ${dclicks}, hive shown after the click ${dstay}, hive gone ${dgone} ms after \`sbarhive drop\`"
  check "S-020 AC4 drop show: a click at (40, 40) outside the hive reaches keys.py once and the hive stays until the timeout" \
    '[ "$dclicks" = 1 ] && [ "$dstay" = yes ] && [ "$dgone" != none ] && [ "$dgone" -ge 9000 ]'

  # Drop mode mid-drag, as the owner's Super+Z and Alt-hold bindings do: a
  # GTK drag source (tests/drop/source.py) drags a PNG and `sbarhive drop`
  # runs while the button is held. Three cases:
  #   cell     the drag enters the hive and is released over the WEBP cell
  #            (ring 1, upper right); the file must be written.
  #   timeout  the drag stays over the hive past the 10 s drop timeout, then
  #            is released on the source window.
  #   esc      Esc over the hive mid-drag.
  # After each, a new drag must begin (pointer input reaches the source).
  gone_within 120 || true
  kill "$kp" 2>/dev/null
  local NEXT="$XDG_RUNTIME_DIR/next" SLOG="$log-source.log"
  GDK_BACKEND=wayland GTK_A11Y=none python3 -I "$ROOT/tests/drop/source.py" "$NEXT" >"$SLOG" 2>&1 &
  pids+=("$!")
  sleep 2
  # begin_drag: press on the source and pass GTK's threshold (lib.sh).
  begin_drag() { drag_past_threshold; }
  # drag_case CASE: sets MOTIONS (wl_data_device.motion events the hive got),
  # WROTE, CLOSED and AFTER (a new drag began afterwards).
  local MOTIONS WROTE CLOSED AFTER
  drag_case() {
    local c=$1 file="$W/drag-$1.png" s0 i
    cp "$ROOT/tests/fixtures/images/alpha.png" "$file"
    printf '%s\n' "$file" >"$NEXT"
    s0=$(wc -l <"$SLOG")
    begin_drag
    local dragging=no; sed -n "$((s0 + 1)),\$p" "$SLOG" | grep -q drag-begin && dragging=yes
    # HYPR_DROP_VIA=exec runs it as the owner's binding does, through
    # Hyprland's exec (which hands the process an activation token).
    if [ "${HYPR_DROP_VIA:-}" = exec ]; then show hyprctl dispatch "hl.dsp.exec_cmd(\"$BIN/sbarhive drop\")"
    else show "$BIN/sbarhive" drop; fi
    local gx gy gw gh cdrag="" cur=""
    read -r gx gy gw gh < <(geo)
    if [ -n "${gw:-}" ]; then
      # Corner, midpoint, then the WEBP cell (lib.sh, through ptr_move).
      move_to_webp "$gx" "$gy" "$gw" "$gh"
      cur="$(hyprctl cursorpos)"
      cdrag=$(cpu_pct 1 "$d")
      case "$c" in
        cell) echo "up" >&3 ;;
        timeout) sleep 10; move 60 60; sleep 0.3; echo "up" >&3 ;;
        esc) key 1; sleep 0.5; echo "up" >&3 ;;
      esac
    else
      echo "up" >&3
    fi
    sleep 0.3
    WROTE=no; for i in $(seq 30); do [ -s "${file%.png}.webp" ] && { WROTE=yes; break; }; sleep 0.1; done
    CLOSED=no; gone_within 130 && CLOSED=yes
    MOTIONS=$(sed -n "$((MARK + 1)),\$p" "$DLOG" | grep -cE '^\[[0-9:.]+\] wl_data_device#[0-9]+\.motion\(')
    local enter; enter=$(sed -n "$((MARK + 1)),\$p" "$DLOG" | grep -m1 -oE 'wl_data_device#[0-9]+\.enter\([0-9]+, wl_surface#[0-9]+, [0-9.]+, [0-9.]+' | sed 's/.*, \([0-9.]*\), \([0-9.]*\)$/\1,\2/')
    local s1; s1=$(wc -l <"$SLOG")
    begin_drag; echo "up" >&3; sleep 0.5
    AFTER=no; sed -n "$((s1 + 1)),\$p" "$SLOG" | grep -q drag-begin && AFTER=yes
    gone_within 130 || true
    reset_hive "drag $c"
    say "INFO drag $c: under way $dragging; drop hive ${gw:+at $gx $gy $gw $gh} in ${LAT:-none} ms; enter at ${enter:-none} (surface-local); cursor over WEBP at ${cur:-?}; motion events $MOTIONS; sbarhived ${cdrag:-?}% CPU; file written $WROTE; hive gone $CLOSED; source: $(sed -n "$((s0 + 1)),$((s1))p" "$SLOG" | grep -oE 'drag-(end|cancel).*' | tr '\n' ' ')"
  }
  drag_case cell
  check "drop mid-drag: the hive gets drag motion events" '[ "$MOTIONS" -gt 0 ]'
  check "drop mid-drag: released over the WEBP cell writes the file" '[ "$WROTE" = yes ]'
  check "drop mid-drag: a new drag begins afterwards (input not stuck)" '[ "$AFTER" = yes ]'
  drag_case timeout
  check "drop timeout mid-drag: the hive closes and a new drag begins" '[ "$CLOSED" = yes ] && [ "$AFTER" = yes ]'
  drag_case esc
  check "Esc mid-drag: the hive closes and a new drag begins" '[ "$CLOSED" = yes ] && [ "$AFTER" = yes ]'

  if [ "$QUASI" = 1 ]; then
    for c in "screen reader flag without a reader keeps the layer" "key release hides" "hive near pointer" "same output as pointer" "adaptive shape"; do
      check "quasimode: $c (S-019, not implemented)" 'false'
    done
  fi

  finish
  say "RESULT $([ "$FAILS" -eq 0 ] && echo PASS || echo "FAIL ($FAILS)")"
  echo "{\"total\": $TOTAL, \"failed\": $FAILS, \"skipped\": 0}" >"$XDG_RUNTIME_DIR/json"
  [ "$FAILS" -eq 0 ]
}
export -f inner

[ "$JSON" = 1 ] || echo "== hyprland $([ "$SELFTEST" = 1 ] && echo selftest || echo "$SURFACE")"
# Human lines: stdout, or stderr with --json (stdout then carries the JSON).
if [ "$JSON" = 1 ]; then exec 4>&2; else exec 4>&1; fi
SANDBOX="$(mktemp -d)"
# Short on purpose: Hyprland's Wayland backend needs socket paths under 108 bytes.
RUNTIME="$(mktemp -d /tmp/sbhh.XXXXXX)"
chmod 700 "$RUNTIME"
mkdir -p "$SANDBOX/.config" "$SANDBOX/.local/share" "$SANDBOX/.cache" "$SANDBOX/.local/state"
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates can reach the live desktop or the real profile.
env -u DISPLAY -u WAYLAND_DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  -u XDG_SESSION_DESKTOP -u DESKTOP_SESSION -u XDG_ACTIVATION_TOKEN -u DESKTOP_STARTUP_ID -u SBARHIVE_SURFACE \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" XDG_RUNTIME_DIR="$RUNTIME" \
  GIO_USE_VFS=local GVFS_DISABLE_FUSE=1 \
  nice -n 19 dbus-run-session -- bash -c inner >&4 2>"$OUT/hypr-bus.log"
rc=$?
if [ "$JSON" = 1 ]; then
  if [ -s "$RUNTIME/json" ]; then cat "$RUNTIME/json"; else echo '{"total": 0, "failed": 1, "skipped": 0}'; rc=1; fi
fi
cp "$RUNTIME"/hypr/*/hyprland.log "$OUT/hypr-hyprland-full.log" 2>/dev/null
rm -rf "$SANDBOX" "$RUNTIME" 2>/dev/null || { sleep 1; rm -rf "$SANDBOX" "$RUNTIME"; }
exit "$rc"
