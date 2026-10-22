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
#                      shape), all real checks:
#                      hive near pointer: `sbarhive open` while a 1 px
#                        pointer jitter around P = (300, 200) runs (every
#                        10 ms for 600 ms, from before the command): a
#                        "hive anchor" line within 1 px of P or P + 1
#                        (the two jitter points, see near_jit) and the
#                        "hive surface" line equal to the AC3 placement for
#                        it (tests/overlay/anchor.py, 1 px) on the 960x540
#                        logical output; then a drag held still at
#                        (500, 300) and `sbarhive drop --at "$(hyprctl
#                        cursorpos)"`, as the owner's binding would run it
#                        (manager decision 1): anchored there at once, one
#                        wl_subsurface.set_position;
#                      key release hides (S-019e, AC11 to AC13 on
#                        Hyprland): `sbarhive drop --hold` with the pointer
#                        still exits 0 within 200 ms, no layer is listed at
#                        300 ms, one is by 1000 ms, then `sbarhive drop
#                        --release` exits 0 within 200 ms and the layer is
#                        gone within 500 ms; and a tap (the release 150 ms
#                        after the press) shows nothing for 1000 ms (no
#                        layer listed, no get_layer_surface in the trace),
#                        both commands exiting 0 within 200 ms;
#                      drop preview after enter (S-019f, AC14, AC15): in
#                        the "cell" drag above, the daemon prints
#                        "hive preview: icon image" after the drag's first
#                        wl_data_device.enter and no preview line before it.
#                      Same output as the pointer is INFO here: nested
#                        Hyprland gave no reliable second output (`hyprctl
#                        output create` listed none within 10 s), so those
#                        two checks run in tests/overlay/kwin.sh outputs
#                        (a private KWin with two virtual outputs).
#                      adaptive shape (S-019b, AC3, AC4): `sbarhive open`
#                        with the pointer jittering 1 px around (4, 4): the
#                        anchor within 1 px of (4, 4) or (5, 5), the
#                        "hive surface" box
#                        and the "hive cells" count and centre equal to
#                        tests/overlay/anchor.py for the printed anchor
#                        (1 px), the box inside the output;
#                      screen reader flag without a reader (S-019a, AC1,
#                        AC2), last, on a daemon with no SBARHIVE_SURFACE:
#                        the fake a11y bus (tests/overlay/a11y.py, started
#                        before KWin with the flag off) sets
#                        ScreenReaderEnabled with no owner of
#                        org.gnome.Orca.Service: a layer show and exactly one
#                        "hive surface choice: layer ... screen reader flag
#                        set, no reader running" line; then a11y.py owns the
#                        name: the next show is a window, with one "hive
#                        surface choice: window ... screen reader running".
# Checks (layer or window): surface shown, keyboard focus received, show
# latency under 150 ms, Esc closes, click outside closes, input reaches the
# other client after close, a click outside a drop show passes through;
# then drop mode mid-drag, as the owner's Super+Z
# and Alt-hold bindings run it (a GTK source from tests/drop/source.py drags
# a PNG): the hive gets wl_data_device.motion, a release over the WEBP cell
# writes the file, the drop timeout and Esc mid-drag both end with the hive
# gone and pointer input working. In a drop show with no drag (S-020 AC4) a
# click at (40, 40), outside the hive, reaches keys.py once and the hive
# stays until the drop timeout; the click comes after the last "hive
# surface" line or 400 ms, whichever is later. In that show the pointer
# stands still and there is no --at, and Hyprland sends no enter to a new
# layer under a still pointer, so (S-019 manager decision 12) the catcher's
# first input region is the hive rectangle (one add, the last "hive
# surface" line) within 400 ms of the first frame (the hive subsurface's
# first attach), by the daemon's WAYLAND_DEBUG trace timestamps
# (still_region, tests/drop/layer.sh), and the hive never moves (one
# wl_subsurface.set_position). 13 checks, 20 with --quasimode.
# Exit 0 only when none failed.
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
  # still_region and within_ms (S-019 decision 12).
  # shellcheck source-path=SCRIPTDIR source=../drop/layer.sh
  source "$ROOT/tests/drop/layer.sh"
  # Human lines go to stdout here; the caller sends them to stderr in --json.
  say() { echo "$*"; }
  check() { TOTAL=$((TOTAL + 1)); if eval "$2"; then say "PASS $1"; else say "FAIL $1"; FAILS=$((FAILS + 1)); fi; }
  now_us() { echo "${EPOCHREALTIME/./}"; }
  pids=()
  finish() {
    exec 3>&- 2>/dev/null
    exec 6>&- 2>/dev/null
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

  # S-019a (--quasimode): the fake accessibility bus (tests/overlay/a11y.py)
  # owns org.a11y.Bus before KWin, Hyprland or the daemon can activate the
  # real at-spi-bus-launcher on this private bus. It starts with the flag off
  # and no reader, so the checks before S-019a see no screen reader.
  local A11Y="" A11YF="$XDG_RUNTIME_DIR/a11y.fifo"
  if [ "$QUASI" = 1 ]; then
    mkfifo "$A11YF"
    python3 -I -W ignore::DeprecationWarning "$ROOT/tests/overlay/a11y.py" "$A11YF" false >"$log-a11y.log" 2>&1 &
    A11Y=$!
    pids+=("$A11Y")
    exec 6>"$A11YF"
    timeout 10 gdbus wait --session org.a11y.Bus || say "INFO fake org.a11y.Bus did not appear"
  fi

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
-- S-019d: a rule for a second output SBH-2, no longer created (the same
-- output checks moved to tests/overlay/kwin.sh outputs); kept so that the
-- first output's rule, which names WAYLAND-1, stays as measured on
-- Hyprland 0.56.2.
hl.monitor({ output = "SBH-2", mode = "1280x720", position = "960x0", scale = "1" })
hl.monitor({ output = "WAYLAND-1", mode = "1280x720", position = "0x0", scale = "$HYPR_SCALE" })
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
  # S-019c and S-019d setup: the hive is anchored at the pointer, so the
  # shows below start with the pointer in the middle of the output, where
  # the hive is where these checks expect it (centred).
  local hcx hcy
  read -r hcx hcy < <(hyprctl monitors -j | python3 -I -c 'import json,sys
m = json.load(sys.stdin)[0]
print(round(m["width"] / m["scale"] / 2), round(m["height"] / m["scale"] / 2))')
  : "${hcx:=480}" "${hcy:=270}"
  centre_ptr() { move "$hcx" "$hcy"; sleep 0.3; }
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

  # geo: the shown hive's x y w h in Hyprland logical pixels (the last
  # surface line: a drop-mode move to the first position prints another,
  # S-019 AC8).
  geo() {
    if [ "$SURFACE" = layer ]; then after "$MARK" | grep '^hive surface:' | tail -1 | cut -d' ' -f4-
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
  centre_ptr
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
  centre_ptr
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
  local dclicks=none dstay=no dgone=none dseg=""
  if "$BIN/sbarhive" ping >/dev/null 2>&1; then
    centre_ptr
    local dt0; dt0=$(now_us)
    show "$BIN/sbarhive" drop
    # S-019 decision 12: the full region phase lasts at most 300 ms from
    # the first frame, so the click at (40, 40) waits for the last "hive
    # surface" line or 400 ms: show returns at the first one, and none may
    # follow after 300 ms. No wait for a "hive anchor" line.
    sleep 0.4
    say "INFO drop: last hive surface line 400 ms after the first: $(after "$MARK" | grep '^hive surface:' | tail -1)"
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
    dseg=$(sed -n "$((MARK + 1)),\$p" "$DLOG")
    reset_hive "the drop show"
  fi
  say "INFO drop click at (40, 40): keys.py clicks ${dclicks}, hive shown after the click ${dstay}, hive gone ${dgone} ms after \`sbarhive drop\`"
  check "S-020 AC4 drop show: a click at (40, 40) outside the hive reaches keys.py once and the hive stays until the timeout" \
    '[ "$dclicks" = 1 ] && [ "$dstay" = yes ] && [ "$dgone" != none ] && [ "$dgone" -ge 9000 ]'
  # S-019 decision 12, the same show (the trace lines, not after()).
  local sdt=none sent sfirst=none slast sreg=none ssw dsets=0
  if [ -n "$dseg" ]; then
    IFS='|' read -r sdt sent sfirst slast sreg ssw <<<"$(still_region "$dseg")"
    dsets=$(grep -cE -- '-> wl_subsurface[#@][0-9]+\.set_position\(' <<<"$dseg")
  fi
  say "INFO drop show, still pointer, no --at (decision 12): catcher's first wl_region ${sdt} ms after the first frame, that region (adds, last add) ${sfirst}, last catcher region ${slast:-none}, last hive surface ${sreg}, enters on the catcher before the region ${sent:-?}, set_position x$dsets"
  check "S-019 decision 12 drop show, still pointer, no --at: the catcher's input region is the hive rectangle (one add, the last \"hive surface\" line) within 400 ms of the first frame, and the hive never moves (one set_position)" \
    '[ "$sreg" != none ] && [ "$sfirst" = "1 $sreg" ] && [ "$slast" = "1 $sreg" ] && within_ms "$sdt" 400 && [ "$dsets" -eq 1 ]'

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
      # S-019d setup: the first drag position moves a centred drop hive
      # once; at its centre that move changes nothing. Then read it again.
      move $((gx + gw / 2)) $((gy + gh / 2)); sleep 0.5
      read -r gx gy gw gh < <(geo)
    fi
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
  # S-019f: this show's trace, for the quasimode preview check.
  local CELL_SEG; CELL_SEG=$(sed -n "$((MARK + 1)),\$p" "$DLOG")
  check "drop mid-drag: the hive gets drag motion events" '[ "$MOTIONS" -gt 0 ]'
  check "drop mid-drag: released over the WEBP cell writes the file" '[ "$WROTE" = yes ]'
  check "drop mid-drag: a new drag begins afterwards (input not stuck)" '[ "$AFTER" = yes ]'
  drag_case timeout
  check "drop timeout mid-drag: the hive closes and a new drag begins" '[ "$CLOSED" = yes ] && [ "$AFTER" = yes ]'
  drag_case esc
  check "Esc mid-drag: the hive closes and a new drag begins" '[ "$CLOSED" = yes ] && [ "$AFTER" = yes ]'

  if [ "$QUASI" = 1 ]; then
    # Key release hides (S-019e): the press, the threshold, the release.
    gone_within 120 || true
    reset_hive "the drag cases"
    centre_ptr
    local hm hrc hms rrc rms pre300=no post=no hgone=no t0 t1
    hm=$(wc -l <"$DLOG"); t0=$(now_us)
    "$BIN/sbarhive" drop --hold >>"$log-cli.log" 2>&1; hrc=$?; hms=$(( ($(now_us) - t0) / 1000 ))
    while [ $(( ($(now_us) - t0) / 1000 )) -lt 300 ]; do sleep 0.02; done
    shown && pre300=yes
    while [ $(( ($(now_us) - t0) / 1000 )) -lt 1000 ]; do shown && { post=yes; break; }; sleep 0.05; done
    sleep 0.3
    t1=$(now_us)
    "$BIN/sbarhive" drop --release >>"$log-cli.log" 2>&1; rrc=$?; rms=$(( ($(now_us) - t1) / 1000 ))
    gone_within 5 && hgone=yes
    say "INFO quasimode key release: press exit $hrc in $hms ms, layer at 300 ms $pre300, layer by 1000 ms $post, release exit $rrc in $rms ms, gone within 500 ms $hgone, get_layer_surface $(sed -n "$((hm + 1)),\$p" "$DLOG" | grep -cE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(')"
    reset_hive "the hold show"
    check "quasimode: key release hides (S-019 AC11 to AC13): \`drop --hold\` exits 0 within 200 ms, no layer at 300 ms, one by 1000 ms, and \`drop --release\` (exit 0 within 200 ms) removes it within 500 ms" \
      '[ "$hrc" = 0 ] && [ "$hms" -le 200 ] && [ "$pre300" = no ] && [ "$post" = yes ] && [ "$rrc" = 0 ] && [ "$rms" -le 200 ] && [ "$hgone" = yes ]'
    # A tap: the release 150 ms after the press.
    local tseen=no tgls
    hm=$(wc -l <"$DLOG"); t0=$(now_us)
    "$BIN/sbarhive" drop --hold >>"$log-cli.log" 2>&1; hrc=$?; hms=$(( ($(now_us) - t0) / 1000 ))
    while [ $(( ($(now_us) - t0) / 1000 )) -lt 150 ]; do sleep 0.01; done
    t1=$(now_us)
    "$BIN/sbarhive" drop --release >>"$log-cli.log" 2>&1; rrc=$?; rms=$(( ($(now_us) - t1) / 1000 ))
    while [ $(( ($(now_us) - t0) / 1000 )) -lt 1000 ]; do shown && tseen=yes; sleep 0.05; done
    tgls=$(sed -n "$((hm + 1)),\$p" "$DLOG" | grep -cE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(|-> xdg_wm_base[#@][0-9]+\.get_xdg_surface\(')
    say "INFO quasimode tap: press exit $hrc in $hms ms, release exit $rrc in $rms ms, a layer listed within 1000 ms $tseen, new surfaces in the trace $tgls"
    reset_hive "the tap"
    check "quasimode: a tap shows nothing (S-019 AC13): press and release 150 ms apart (both exit 0 within 200 ms), no layer and no new surface for 1000 ms" \
      '[ "$hrc" = 0 ] && [ "$hms" -le 200 ] && [ "$rrc" = 0 ] && [ "$rms" -le 200 ] && [ "$tseen" = no ] && [ "$tgls" -eq 0 ]'

    # Drop preview after enter (S-019f), in the "cell" drag's trace.
    local pe1 ppre=0 ppost=""
    pe1=$(grep -nE '^\[[0-9:.]+\] wl_data_device[#@][0-9]+\.enter\(' <<<"$CELL_SEG" | head -1 | cut -d: -f1)
    if [ -n "$pe1" ]; then
      ppre=$(head -n "$pe1" <<<"$CELL_SEG" | grep -cE '^hive preview: ')
      ppost=$(tail -n +"$pe1" <<<"$CELL_SEG" | grep -m1 -E '^hive preview: ')
    fi
    say "INFO quasimode drop preview: first enter at segment line ${pe1:-none}, preview lines before it $ppre, first after it: ${ppost:-none}"
    check "quasimode: drop preview after enter (S-019 AC14, AC15): \"hive preview: icon image\" after the drag's first enter, none before it" \
      '[ -n "$pe1" ] && [ "$ppre" -eq 0 ] && [ "$ppost" = "hive preview: icon image" ]'
    local nact; nact=$("$BIN/sbarhive" actions "$W/photo.png" 2>/dev/null | grep -c .)
    # near A B TOL: |A - B| <= TOL, for decimals.
    near() { awk -v a="$1" -v b="$2" -v t="$3" 'BEGIN { d = a - b; if (d < 0) d = -d; exit !(a != "" && b != "" && d <= t) }'; }
    # near_jit A P: A within 1 px of P or of P + 1, the two points of a
    # 1 px jitter. move() rounds to whole KWin pixels, so at scale 1.333333
    # the corner jitter (4, 5) lands at 3.75 and 5.25: an anchor read at the
    # second point is 1.25 px from P (seen once in 5 runs).
    near_jit() { near "$1" "$2" 1 || near "$1" "$(($2 + 1))" 1; }
    # The output size in logical pixels (960x540 at 1.333333).
    local lw lh
    read -r lw lh < <(hyprctl monitors -j | python3 -I -c 'import json,sys
m = json.load(sys.stdin)[0]
print(round(m["width"] / m["scale"]), round(m["height"] / m["scale"]))')
    # placed SEG W H: "ok" when the segment's anchor line is a position and
    # its surface line equals the AC3 placement for it within 1 px.
    placed() {
      local ax ay sx sy sw sh mr ox oy ow oh
      read -r _ _ ax ay <<<"$(grep -m1 -E '^hive anchor: ' <<<"$1")"
      read -r _ _ _ sx sy sw sh <<<"$(grep -m1 -E '^hive surface: layer ' <<<"$1")"
      [ -n "${ay:-}" ] && [ -n "${sh:-}" ] || return 1
      while read -r mr _ _ _ ox oy ow oh; do
        near "$sx" "$ox" 1 && near "$sy" "$oy" 1 && near "$sw" "$ow" 1 && near "$sh" "$oh" 1 && return 0
      done < <(python3 -I "$ROOT/tests/overlay/anchor.py" "$ROOT/design/tokens.json" "$2" "$3" "$ax" "$ay" "$nact")
      return 1
    }
    anchor_of() { grep -m1 -E '^hive anchor: ' <<<"$1" | cut -d' ' -f3-; }

    # Hive near pointer, 1: `sbarhive open` while the pointer moves 1 px
    # around P (Hyprland logical pixels).
    gone_within 120 || true
    local i px=300 py=200
    move "$px" "$py"; sleep 0.5
    MARK=$(wc -l <"$DLOG")
    for ((i = 0; i < 60; i++)); do
      if ((i % 2)); then move $((px + 1)) $((py + 1)); else move "$px" "$py"; fi
      echo "sleep 10" >&3
    done
    show "$BIN/sbarhive" open "$W/photo.png"
    sleep 1
    local nseg; nseg=$(after "$MARK")
    local nax nay; read -r nax nay <<<"$(anchor_of "$nseg")"
    say "INFO quasimode near pointer (open): P ($px, $py), output ${lw}x${lh}, $(grep -m1 -E '^hive anchor: ' <<<"$nseg" || echo 'no anchor line'), $(grep -m1 -E '^hive surface: ' <<<"$nseg" || echo 'no surface line')"
    key 1; gone_within 30 || true
    reset_hive "the near pointer show"
    check "quasimode: hive near pointer (S-019 AC5, AC6): \`sbarhive open\` with the pointer moving at ($px, $py) is anchored within 1 px of it (or of it + 1, the jitter's other point) and placed by plan_at for that anchor" \
      'near_jit "$nax" "$px" && near_jit "$nay" "$py" && placed "$nseg" "$lw" "$lh"'

    # Hive near pointer, 2: decision 1, a drag held still and
    # `sbarhive drop --at "$(hyprctl cursorpos)"` (no wait, no move).
    local s0; s0=$(wc -l <"$SLOG")
    cp "$ROOT/tests/fixtures/images/alpha.png" "$W/held.png"
    printf '%s\n' "$W/held.png" >"$NEXT"
    begin_drag
    move 500 300; sleep 0.5
    local cur; cur=$(hyprctl cursorpos | tr -d ' ')
    show "$BIN/sbarhive" drop --at "$(hyprctl cursorpos)"
    sleep 1
    local dseg; dseg=$(after "$MARK")
    local dax day; read -r dax day <<<"$(anchor_of "$dseg")"
    local dsets; dsets=$(sed -n "$((MARK + 1)),\$p" "$DLOG" | grep -cE -- '-> wl_subsurface[#@][0-9]+\.set_position\(')
    say "INFO quasimode near pointer (drop --at): cursorpos $cur, $(grep -E '^hive anchor: ' <<<"$dseg" | tr '\n' ' ' || true)$(grep -m1 -E '^hive surface: ' <<<"$dseg" || echo 'no surface line'), set_position x$dsets"
    echo "up" >&3
    gone_within 130 || true
    reset_hive "the drop --at show"
    check "quasimode: hive near pointer (S-019 decision 1): a still drag and \`sbarhive drop --at \"\$(hyprctl cursorpos)\"\` anchor at the cursor (1 px) at once, with one set_position" \
      '[ "$(grep -cE "^hive anchor: " <<<"$dseg")" -eq 1 ] && near "$dax" "${cur%,*}" 1 && near "$day" "${cur#*,}" 1 && [ "$dsets" -eq 1 ] && placed "$dseg" "$lw" "$lh"'

    # Adaptive shape (S-019b, AC3, AC4): `sbarhive open` with the pointer
    # moving 1 px around B, a few pixels inside the top left corner (Hyprland
    # 0 is the nested window's edge in KWin). The anchor is clamped, so the
    # hive keeps only the slots that fit: its box, its cell count and its
    # centre must be what tests/overlay/anchor.py predicts for the printed
    # anchor, and the box must lie inside the output.
    gone_within 120 || true
    local bx=4 by=4
    move "$bx" "$by"; sleep 0.5
    MARK=$(wc -l <"$DLOG")
    for ((i = 0; i < 60; i++)); do
      if ((i % 2)); then move $((bx + 1)) $((by + 1)); else move "$bx" "$by"; fi
      echo "sleep 10" >&3
    done
    local bcur; bcur=$(hyprctl cursorpos | tr -d ' ')
    show "$BIN/sbarhive" open "$W/photo.png"
    sleep 1
    local bseg; bseg=$(after "$MARK")
    key 1; gone_within 30 || true
    reset_hive "the corner show"
    local bax bay bsx="" bsy="" bsw="" bsh="" bcn="" bccx="" bccy="" borc="" bcells=no
    read -r bax bay <<<"$(anchor_of "$bseg")"
    read -r _ _ _ bsx bsy bsw bsh <<<"$(grep -m1 -E '^hive surface: layer ' <<<"$bseg")"
    read -r _ _ bcn bccx bccy <<<"$(grep -m1 -E '^hive cells: ' <<<"$bseg")"
    [ -n "${bay:-}" ] && [ "$bax" != none ] &&
      borc=$(python3 -I "$ROOT/tests/overlay/anchor.py" "$ROOT/design/tokens.json" "$lw" "$lh" "$bax" "$bay" "$nact")
    local ocx ocy ocn
    while read -r _ ocx ocy ocn _; do
      [ -n "${ocn:-}" ] && [ "$bcn" = "$ocn" ] && [ -n "$bccy" ] &&
        near "$(awk -v a="$bsx" -v b="$bccx" 'BEGIN { print a + b }')" "$ocx" 1 &&
        near "$(awk -v a="$bsy" -v b="$bccy" 'BEGIN { print a + b }')" "$ocy" 1 && bcells=yes
    done <<<"$borc"
    say "INFO quasimode adaptive shape: B ($bx, $by), cursorpos $bcur, actions $nact, $(grep -m1 -E '^hive anchor: ' <<<"$bseg" || echo 'no anchor line'), $(grep -m1 -E '^hive surface: ' <<<"$bseg" || echo 'no surface line'), $(grep -m1 -E '^hive cells: ' <<<"$bseg" || echo 'no cells line'), anchor.py (min_rings cx cy count x y w h): $(tr '\n' '|' <<<"$borc")"
    check "quasimode: adaptive shape (S-019b AC3, AC4): \`sbarhive open\` with the pointer at the top left corner is anchored there (1 px from B or B + 1, the two jitter points), its \"hive surface\" box and \"hive cells\" count and centre equal plan_at by tests/overlay/anchor.py (1 px), and the box lies inside the ${lw}x${lh} output" \
      'near_jit "$bax" "$bx" && near_jit "$bay" "$by" && placed "$bseg" "$lw" "$lh" && [ "$bcells" = yes ] && [ "$bsx" -ge 0 ] && [ "$bsy" -ge 0 ] && [ $((bsx + bsw)) -le "$lw" ] && [ $((bsy + bsh)) -le "$lh" ]'

    # Screen reader rule (S-019a, AC1, AC2), last because it replaces the
    # daemon: one daemon with no SBARHIVE_SURFACE (config hive_surface absent,
    # so auto). With ScreenReaderEnabled set on the fake bus and no owner of
    # org.gnome.Orca.Service, its show is a layer surface and exactly one
    # "hive surface choice:" line names the reason; once a11y.py owns the
    # name (as Orca does when it starts), the same daemon's next show is a
    # window and its choice line says a reader is running. The second
    # segment starts before the name appears, because the line comes when
    # NameOwnerChanged arrives, not at the next show.
    gone_within 120 || true
    stop_daemon
    echo on >&6; sleep 1
    local am1 am2 aseg1 aseg2
    am1=$(wc -l <"$DLOG")
    SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >>"$DLOG" 2>&1 &
    d=$!
    pids+=("$d")
    for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
    centre_ptr
    show "$BIN/sbarhive" open "$W/photo.png"
    sleep 0.5; key 1; sleep 1
    am2=$(wc -l <"$DLOG")
    echo orca >&6
    timeout 10 gdbus wait --session org.gnome.Orca.Service || say "INFO fake org.gnome.Orca.Service did not appear"
    sleep 1
    centre_ptr
    show "$BIN/sbarhive" open "$W/photo.png"
    sleep 0.5; key 1; sleep 1
    stop_daemon
    aseg1=$(sed -n "$((am1 + 1)),${am2}p" "$DLOG")
    aseg2=$(sed -n "$((am2 + 1)),\$p" "$DLOG")
    # rq SEG KIND: the daemon's get_layer_surface (layer) or get_xdg_surface
    # (xdg) requests in SEG; nch SEG: its choice lines.
    rq() {
      local p='-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\('
      [ "$2" = xdg ] && p='-> xdg_wm_base[#@][0-9]+\.get_xdg_surface\('
      grep -cE -- "$p" <<<"$1"
    }
    nch() { grep -cE '^hive surface choice: ' <<<"$1"; }
    say "INFO quasimode screen reader, flag set and no reader: get_layer_surface $(rq "$aseg1" layer), get_xdg_surface $(rq "$aseg1" xdg), choice lines: $(grep -E '^hive surface choice: ' <<<"$aseg1" | tr '\n' '|')"
    say "INFO quasimode screen reader, fake Orca on the bus: get_layer_surface $(rq "$aseg2" layer), get_xdg_surface $(rq "$aseg2" xdg), choice lines: $(grep -E '^hive surface choice: ' <<<"$aseg2" | tr '\n' '|')"
    say "INFO a11y.py: $(tr '\n' ' ' <"$log-a11y.log" | cut -c1-600)"
    check "quasimode: screen reader flag without a reader keeps the layer (S-019a AC1, AC2): with ScreenReaderEnabled set and no org.gnome.Orca.Service owner the show is a layer (no get_xdg_surface) with exactly one \"hive surface choice: layer ... screen reader flag set, no reader running\" line; with a fake Orca owner the next show is a window (no get_layer_surface) with one \"hive surface choice: window ... screen reader running\" line" \
      '[ "$(rq "$aseg1" layer)" -ge 1 ] && [ "$(rq "$aseg1" xdg)" -eq 0 ] && [ "$(nch "$aseg1")" -eq 1 ] && grep -qE "^hive surface choice: layer\b.*screen reader flag set, no reader running" <<<"$aseg1" && [ "$(rq "$aseg2" layer)" -eq 0 ] && [ "$(rq "$aseg2" xdg)" -ge 1 ] && [ "$(nch "$aseg2")" -eq 1 ] && grep -qE "^hive surface choice: window\b.*screen reader running" <<<"$aseg2"'

    # Same output as the pointer (S-019 AC5, decision 1): nested Hyprland
    # made no reliable second output (`hyprctl output create wayland SBH-2`
    # answered ok, but SBH-2 was not listed with a mode within 10 s), so the
    # two checks moved to a private KWin with two virtual outputs.
    say "INFO quasimode: same output as pointer (S-019 AC5): \`sbarhive open\` on the second output is checked by tests/overlay/kwin.sh outputs"
    say "INFO quasimode: same output as pointer (S-019 decision 1): \`sbarhive drop --at\` with global coordinates on the second output is checked by tests/overlay/kwin.sh outputs"
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
