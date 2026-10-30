#!/usr/bin/env bash
# The hive on Hyprland, in a private nested Hyprland (aquamarine Wayland
# backend) inside a private headless kwin_wayland --virtual. Reproduces the
# report on Hyprland 0.56 with KAWARO_SURFACE=layer: the overlay
# shows, but no `hive focus` line ever comes and input feels frozen.
# Input comes from org_kde_kwin_fake_input on the outer KWin
# (tests/drop/kwin-input), so it reaches Hyprland as a real seat keyboard and
# pointer through its Wayland backend. Another client (keys.py, a full screen
# GTK window inside Hyprland) logs the keys and clicks it gets.
# Modes:
#   (default) layer    human PASS/FAIL lines on stdout, plus INFO lines with
#                      the measured numbers (latency, CPU, drop mode).
#   window             the same checks with KAWARO_SURFACE=window.
#   --selftest         only start the nested Hyprland, check that it takes a
#                      client (keys.py maps), tear down; exit 0 on success.
#   --json             the checks; human lines go to stderr and stdout gets
#                      exactly one line: {"total": N, "failed": F, "skipped": S}.
#   --json --quasimode adds the S-019 checks (a screen reader flag without a
#                      running reader keeps the layer, key release hides, hive
#                      near the pointer, same output as the pointer, adaptive
#                      shape), all real checks:
#                      hive near pointer: `kawaro open` while a 1 px
#                        pointer jitter around P = (300, 200) runs (every
#                        10 ms for 600 ms, from before the command): a
#                        "hive anchor" line within 1 px of P or P + 1
#                        (the two jitter points, see near_jit) and the
#                        "hive surface" line equal to the AC3 placement for
#                        it (tests/overlay/anchor.py, 1 px) on the 960x540
#                        logical output; then a drag held still at
#                        (500, 300) and `kawaro drop --at "$(hyprctl
#                        cursorpos)"`, as the documented binding would run it
#                        (manager decision 1): anchored there at once, one
#                        wl_subsurface.set_position;
#                      key release hides (S-019e, AC11 to AC13 on
#                        Hyprland): `kawaro drop --hold` with the pointer
#                        still exits 0 within 200 ms, no layer is listed at
#                        300 ms, one is by 1000 ms, then `kawaro drop
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
#                      adaptive shape (S-019b, AC3, AC4): `kawaro open`
#                        with the pointer jittering 1 px around (4, 4): the
#                        anchor within 1 px of (4, 4) or (5, 5), the
#                        "hive surface" box
#                        and the "hive cells" count and centre equal to
#                        tests/overlay/anchor.py for the printed anchor
#                        (1 px), the box inside the output;
#                      screen reader flag without a reader (S-019a, AC1,
#                        AC2), last, on a daemon with no KAWARO_SURFACE:
#                        the fake a11y bus (tests/overlay/a11y.py, started
#                        before KWin with the flag off) sets
#                        ScreenReaderEnabled with no owner of
#                        org.gnome.Orca.Service: a layer show and exactly one
#                        "hive surface choice: layer ... screen reader flag
#                        set, no reader running" line; then a11y.py owns the
#                        name: the next show is a window, with one "hive
#                        surface choice: window ... screen reader running".
#   --portal           S-014e3 (backlog/S-014-drop-hive.md, AC20 for
#                      Hyprland), and nothing else: the installed
#                      xdg-desktop-portal with xdg-desktop-portal-hyprland on
#                      the private bus, the daemon's real registration, the ID
#                      read from `hyprctl globalshortcuts` of the nested
#                      instance and its global bind in the private config,
#                      then AC20 (a), (b), (c) and the AC21 runs of
#                      tests/drop/portal_real.sh, a release over the hive,
#                      and the record of what the backend sent. See
#                      tests/hyprland/portal.sh. The only mode that skips
#                      tests/no_portal.sh; the others keep it.
# Checks (layer or window): surface shown, keyboard focus received, show
# latency under 150 ms, Esc closes, click outside closes, input reaches the
# other client after close. Then S-021 (backlog/S-021-drop-only-on-drag.md),
# the drop shows with no drag, keys.py under the hive; each one's probe line
# and hive buffers are checked by probe_checks (tests/drop/layer.sh: exactly
# one "hive reveal: N" or "hive probe: hidden (R)" line, and no attach on the
# hive subsurface before the reveal or in a hidden show):
#   AC5 (b), replacing S-020 AC4: the pointer still at (40, 40), outside the
#     centred hive, and `kawaro drop`: a click there after the switch to
#     the hive rectangle reaches keys.py once and the catcher stays bound;
#     the pointer then moves into the rectangle: "hidden (pointer)". The same
#     show keeps S-019 decision 12 with the S-021 AC4 reference: the
#     catcher's first input region is the hive rectangle (one add, the last
#     "hive surface" line, also its last region) within 400 ms of the
#     catcher's ready commit (its first commit after the first configure),
#     by the daemon's WAYLAND_DEBUG trace timestamps (still_region);
#   AC5 (a): `drop --at` the cursor, the pointer still inside the hive, then
#     a 5 px move: "hidden (pointer)", the layer destroyed within 500 ms of
#     the catcher's wl_pointer.enter, a click after the move reaches keys.py
#     once;
#   AC9 (1): `drop --hold --at` the cursor, the layer bound, a 5 px move:
#     "hidden (pointer)", and `drop --release` after that hide exits 0
#     within 200 ms and binds no surface for 1000 ms;
#   AC5 (c) and AC9 (2): `drop --hold`, the layer bound, the pointer still,
#     `drop --release` (exit 0 within 200 ms): gone within 500 ms,
#     "hidden (release)".
# Then drop mode mid-drag, as the documented Super+Z
# and Alt-hold bindings run it (a GTK source from tests/drop/source.py drags
# a PNG): AC9 (3), on the same daemon, a PNG drag and `drop --hold`, the drag
# moved into the hive: "hive reveal: N", and no protocol error in the daemon
# log; the hive gets wl_data_device.motion, a release over the WEBP cell
# writes the file, the drop timeout and Esc mid-drag both end with the hive
# gone and pointer input working. S-021 AC7 on the "cell" drag: "hive
# reveal: N" with N the lines of `kawaro actions` for the PNG, and the
# hive's first attach after the drag's first wl_data_device.enter on the
# catcher; the timeout and Esc drags each print one "hive reveal" line
# (AC3). S-021 AC6: a drag of notes.unknownext (no action) moved into the
# hive gives an empty "drop cells:" line, then "hidden (no-actions)", no
# wl_data_offer.accept with a type, no new file, and a new drag begins
# afterwards; a drag of a plain string (source.py "text:", no
# text/uri-list) gives "hidden (no-uri-list)" and no wl_data_offer.receive.
# S-022 (backlog/S-022-drop-followups.md), layer mode, on the same daemon:
# AC7, the pointer still at (40, 40), `drop --at 500,300` twice 1 s apart,
# then a move there: "hidden (replaced)" between the two shows' "hive
# surface:" lines, then "hidden (pointer)", nothing drawn; AC1, a PNG drag,
# `drop --hold`, the drag over the revealed hive's WEBP cell and `drop
# --release` there (the layer destroyed within 500 ms, the button let go in
# place writes no WEBP), then a second PNG drag whose enter is read (one
# "drop cells:" line, "hive reveal: 15", an accept naming a type, its WEBP
# written) and no protocol error; AC4, a PNG drag resting at the centre of
# the revealed hive: 4 or more attach on the hive subsurface in the 200 ms
# from its first with motion, 1 or 2 on a daemon with
# KAWARO_REDUCED_MOTION=1, a normal show's count as INFO; AC5, the S-021
# AC8 budget over eight revealed shows (AC9 (3), the two AC1 drags, the two
# AC4 shows, cell, timeout, Esc).
# 55 checks, 62 with --quasimode. The S-021 and S-022 checks run in layer
# mode only (decision 1: the window path keeps its behaviour and prints no
# probe line), so window mode runs 11 checks (18 with --quasimode).
# Exit 0 only when none failed.
# On Hyprland 0.56.2 the layer path gets no drag motion: Hyprland makes no
# view for a subsurface of a layer surface, so its drag motion listener finds
# none for the hive subsurface and sends nothing (src/protocols/core/
# DataDevice.cpp, mouse.move listener in initiateDrag).
# HYPR_SCALE sets the nested output scale (default 1.333333, a common laptop panel scale).
# HYPR_DROP_VIA=exec runs the mid-drag `kawaro drop` through Hyprland's
# exec dispatcher, as the documented Super+Z binding does.
# Never touches the live desktop: everything runs with a throwaway HOME, XDG
# dirs, runtime dir and session bus, with DISPLAY, WAYLAND_DISPLAY,
# HYPRLAND_INSTANCE_SIGNATURE and the desktop names unset before
# dbus-run-session. LIBSEAT_BACKEND=seatd with no seatd socket makes
# aquamarine's DRM backend fail at once, so logind is never asked for the
# live session's devices.
# --altdrag (S-095, ALTN drags per order, default 20; layer only): the
# documented Alt-hold binds (bindni and bindrni on ALT_L, Lua form) in the
# private config, real ALT_L key events and a real drag of a PNG, in both
# orders. Writes $OUT/altdrag-results.tsv, one row per drag, and
# $OUT/altdrag-N.log per drag (daemon log and `hyprctl layers`).
# Usage: tests/hyprland/repro.sh [--selftest|--portal|--altdrag|--json [--quasimode]] [layer|window] [BIN_DIR]
# BIN_DIR defaults to target/release; ~/.local/bin tests the installed build.
# shellcheck disable=SC2016,SC2034,SC2329
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
# shellcheck source=../no_portal.sh
source "$ROOT/tests/no_portal.sh"
SELFTEST=0 JSON=0 QUASI=0 PORTAL=0 ALTDRAG=0 SURFACE=layer
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in
    --selftest) SELFTEST=1 ;;
    --json) JSON=1 ;;
    --quasimode) QUASI=1 ;;
    --portal) PORTAL=1 ;;
    --altdrag) ALTDRAG=1 ;;
    layer|window) SURFACE=$a ;;
    *) BIN="$a" ;;
  esac
done
INPUT="$ROOT/tests/drop/kwin-input/target/release/kwin-input"
need() { echo "$1" >&2; [ "$JSON" = 1 ] && echo '{"total": 0, "failed": 1, "skipped": 0}'; exit 2; }
if ! command -v Hyprland >/dev/null || ! command -v kwin_wayland >/dev/null; then need "needs Hyprland and kwin_wayland"; fi
[ -x "$BIN/kawaro" ] && [ -x "$BIN/kawarod" ] || need "build first: cargo build --release"
[ -x "$INPUT" ] || need "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"
OUT="${HYPR_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT INPUT SELFTEST JSON QUASI PORTAL ALTDRAG SURFACE ALTN
ALTN="${ALTN:-20}"
export HYPR_SCALE="${HYPR_SCALE:-1.333333}"

inner() {
  set -u
  local log="$OUT/hypr-$SURFACE"
  [ "$SELFTEST" = 1 ] && log="$OUT/hypr-selftest"
  [ "$PORTAL" = 1 ] && log="$OUT/hypr-portal"
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
    [ -n "${d:-}" ] && { "$BIN/kawaro" quit >/dev/null 2>&1; sleep 0.3; }
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
  # No stay_focused rule for the hive window: a typical live config has
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
-- Typical rules for the hive window (window path), so it floats centred.
local kawaro = { class = [[^(io\\.github\\.m7mmad_omar\\.Kawaro)$]], title = [[^(Kawaro)$]] }
hl.window_rule({ match = kawaro, float = true })
hl.window_rule({ match = kawaro, center = true })
hl.window_rule({ match = kawaro, pin = true })
hl.window_rule({ match = kawaro, no_anim = true })
hl.window_rule({ match = kawaro, border_size = 0 })
hl.layer_rule({ match = { namespace = "^kawaro$" }, no_anim = true })
LUA
  if [ "$ALTDRAG" = 1 ]; then
    # The documented Alt-hold binds and the S-102 key hook for Alt chords
    # (integrations/linux/hyprland/README.md).
    cat >>"$XDG_CONFIG_HOME/hypr/sbh.lua" <<LUA
hl.bind("ALT_L", hl.dsp.exec_cmd("$BIN/kawaro drop --hold --at \\"\$(hyprctl cursorpos)\\""),
  { non_consuming = true, ignore_mods = true })
hl.bind("ALT_L", hl.dsp.exec_cmd("$BIN/kawaro drop --release"),
  { release = true, non_consuming = true, ignore_mods = true })
local alt_down = false
hl.on("input.keyboard.key", function(code, _, state)
  if code == 64 then
    alt_down = state == 1
  elseif alt_down and state == 1 then
    hl.exec_cmd("sleep 0.05; $BIN/kawaro drop --other-key")
  end
end)
LUA
  fi
  # --portal: no systemd or activation environment updates by Hyprland
  # itself (the portal's environment is set below, on this bus only).
  [ "$PORTAL" = 1 ] && export HYPRLAND_NO_SD_VARS=1
  HYPR_CONF="$XDG_CONFIG_HOME/hypr/sbh.lua"
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
  # A real session names Hyprland; only private processes see these.
  export XDG_CURRENT_DESKTOP=Hyprland XDG_SESSION_TYPE=wayland
  if [ "$PORTAL" = 1 ]; then
    # S-014e3: what the bus hands to the portal it activates, on this private
    # bus only (no --systemd, no --all). Portal processes activated before
    # this (none expected) are ended, so the next activation reads it.
    # shellcheck source=portal.sh
    source "$HERE/portal.sh"
    dbus-update-activation-environment WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
    local early; early=$(bus_procs)
    say "INFO portal: activation environment set on the private bus; portal processes already running: $(tr '\n' ';' <<<"${early:-none}")"
    for p in $(cut -d' ' -f1 <<<"$early"); do kill "$p" 2>/dev/null; done
  fi
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
  for _ in $(seq 50); do hyprctl clients 2>/dev/null | grep -q 'class: org.kawaro.HyprKeys' && { mapped=1; break; }; sleep 0.2; done

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

  if [ "$PORTAL" = 1 ]; then
    # S-014e3 (tests/hyprland/portal.sh): keys.py is not needed; the dialog
    # watch counts every window that is not the daemon's or the source's.
    kill "$kp" 2>/dev/null
    local W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
    mkdir -p "$XDG_CONFIG_HOME/kawaro"
    printf 'language = "en"\ntheme = "light"\nhold_ms = 400\n' >"$XDG_CONFIG_HOME/kawaro/config.toml"
    local NEXT="$XDG_RUNTIME_DIR/next" SLOG="$log-source.log" OUT_LOG="$log" MODE=portal-real
    local DLOG="$log-first-daemon.log" d="" src=""
    portal_hypr_run
    finish
    say "RESULT $([ "$FAILS" -eq 0 ] && echo PASS || echo "FAIL ($FAILS)")"
    echo "{\"total\": $TOTAL, \"failed\": $FAILS, \"skipped\": 0}" >"$XDG_RUNTIME_DIR/json"
    [ "$FAILS" -eq 0 ]
    return
  fi
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
  mkdir -p "$XDG_CONFIG_HOME/kawaro"
  printf 'language = "en"\ntheme = "light"\nportal_shortcut = false\n' >"$XDG_CONFIG_HOME/kawaro/config.toml"
  local DLOG="$log-daemon.log"
  : >"$DLOG"
  # start_daemon [WL] [RM]: WL=1 (the default) adds WAYLAND_DEBUG=1. Show 1,
  # the latency show, runs on a daemon without it, so the trace's writes are
  # not part of the measured time. RM is KAWARO_REDUCED_MOTION, 0 (motion)
  # by default; S-022 AC4 runs one show on a daemon with RM=1.
  local DAEMON_WL=1 DAEMON_RM=0
  start_daemon() {
    DAEMON_WL=${1:-1} DAEMON_RM=${2:-0}
    if [ "$DAEMON_WL" = 1 ]; then
      KAWARO_REDUCED_MOTION=$DAEMON_RM KAWARO_SURFACE=$SURFACE KAWARO_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/kawarod" >>"$DLOG" 2>&1 &
    else
      KAWARO_REDUCED_MOTION=$DAEMON_RM KAWARO_SURFACE=$SURFACE KAWARO_DEBUG_FOCUS=1 "$BIN/kawarod" >>"$DLOG" 2>&1 &
    fi
    d=$!
    pids+=("$d")
    for _ in $(seq 50); do "$BIN/kawaro" ping >/dev/null 2>&1 && break; sleep 0.1; done
  }
  stop_daemon() {
    "$BIN/kawaro" quit >/dev/null 2>&1
    timeout 3 tail --pid="$d" -f /dev/null 2>/dev/null || kill "$d" 2>/dev/null
    sleep 0.3
  }
  start_daemon 0
  # A hive left shown by a failed check would spoil the next ones: restart.
  reset_hive() {
    shown || return 0
    say "INFO hive still shown after $1; restarting the daemon"
    stop_daemon
    start_daemon "$DAEMON_WL" "$DAEMON_RM"
  }

  # Shown: a layer named kawaro, or the window titled exactly Kawaro.
  shown() {
    if [ "$SURFACE" = layer ]; then hyprctl layers 2>/dev/null | grep -q 'namespace: kawaro'
    else hyprctl clients 2>/dev/null | grep -qE '^\s+title: Kawaro$'; fi
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
    if c["title"] == "Kawaro": print(*c["at"], *c["size"]); break' && return; sleep 0.1
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
  show "$BIN/kawaro" open "$W/photo.png"
  check "surface shown ($SURFACE)" 'after "$MARK" | grep -q "^hive surface: $SURFACE"'
  check "show latency under 150 ms (${LAT:-none} ms)" '[ -n "$LAT" ] && [ "$LAT" -lt 150 ]'
  sleep 0.5
  check "keyboard focus received (hive focus: true)" 'after "$MARK" | grep -q "^hive focus: true"'
  local k0; k0=$(wc -l <"$KLOG")
  local cd ch; read -r cd ch <<<"$(cpu_pct 2 "$d" "$hy")"
  say "INFO shown 2 s: kawarod ${cd}% CPU, Hyprland ${ch}% CPU (one core = 100)"
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
  show "$BIN/kawaro" open "$W/photo.png"
  sleep 0.5
  click 40 40
  check "click outside closes" 'gone_within 15'
  reset_hive "the click outside"

  # After the hive is gone the other client gets keys again.
  click 100 100; sleep 0.3
  k0=$(wc -l <"$KLOG")
  key 30; sleep 0.5
  check "input reaches another client after close" '[ "$(sed -n "$((k0 + 1)),\$p" "$KLOG" | grep -c "^keys: key")" -ge 1 ]'

  # S-021 is the layer path's (the window path prints no probe line and
  # keeps its own drop behaviour, S-021 decision 1): layer mode only.
  if [ "$SURFACE" = layer ]; then
    # S-021: the drop shows with no drag (AC5 (a), (b), (c)) and the first
    # half of the AC9 cycle, while keys.py is the client under the hive.
    # seg_from MARK: the daemon log after MARK, the trace included.
    seg_from() { sed -n "$(($1 + 1)),\$p" "$DLOG"; }
    # enter_to_gone SEG: ms from the first wl_pointer.enter on the catcher to
    # the layer surface's destroy, by the trace's stamps ("none" if either is
    # missing).
    enter_to_gone() {
      local cat e g
      cat=$(grep -oE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$1" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
      [ -n "$cat" ] && e=$(grep -m1 -E -- "^\[[0-9:.]+\] wl_pointer[#@][0-9]+\.enter\([0-9]+, ${cat}, " <<<"$1")
      g=$(grep -m1 -E -- '-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\(' <<<"$1")
      [ -n "${e:-}" ] && [ -n "$g" ] || { echo none; return; }
      awk -v a="$(trace_ms "$e")" -v b="$(trace_ms "$g")" 'BEGIN { printf "%.1f\n", b - a }'
    }
    # new_surfaces SEG: get_layer_surface and get_xdg_surface requests in SEG.
    new_surfaces() { grep -cE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(|-> xdg_wm_base[#@][0-9]+\.get_xdg_surface\(' <<<"$1"; }
    # wait_shown MS: until the layer (or window) is listed, at most MS ms.
    wait_shown() { local t0; t0=$(now_us); while [ $(( ($(now_us) - t0) / 1000 )) -lt "$1" ]; do shown && return 0; sleep 0.05; done; shown; }

    # AC5 (b), replacing S-020 AC4: the pointer still at (40, 40), outside
    # the rectangle of the centred hive, and `kawaro drop` (no --at). A
    # click there after the switch to the hive rectangle (S-019 decision 12:
    # 300 ms from the position, so after the last "hive surface" line or
    # 400 ms) reaches keys.py once and leaves the catcher bound; then the
    # pointer moves into the rectangle: "hive probe: hidden (pointer)".
    local bclicks=none bstay=no bseg=""
    if "$BIN/kawaro" ping >/dev/null 2>&1; then
      move 40 40; sleep 0.3
      show "$BIN/kawaro" drop
      sleep 0.9
      k0=$(wc -l <"$KLOG")
      click 40 40; sleep 0.6
      bclicks=$(sed -n "$((k0 + 1)),\$p" "$KLOG" | grep -c '^keys: click$')
      shown && bstay=yes
      local i; for i in 1 2 3 4 5 6 7 8; do move $((40 + i * (hcx - 40) / 8)) $((40 + i * (hcy - 40) / 8)); sleep 0.05; done
      sleep 0.8
      bseg=$(seg_from "$MARK")
      reset_hive "the drop show (b)"
    fi
    say "INFO S-021 AC5 (b): keys.py clicks at (40, 40) ${bclicks}, catcher still bound after the click ${bstay}"
    # S-105: Hyprland 0.57 enters a layer surface on map and the catcher then
    # takes the first click; known limit, the check still runs and asserts
    # exactly as before on older Hyprland (version gate, manager ruling).
    local hv hmaj hmin
    hv=$(hyprctl version 2>/dev/null | grep -oE 'Hyprland [0-9]+\.[0-9]+' | head -1)
    hmaj=$(sed -E 's/Hyprland ([0-9]+)\..*/\1/' <<<"$hv"); hmin=$(sed -E 's/Hyprland [0-9]+\.([0-9]+)/\1/' <<<"$hv")
    if [ -n "$hv" ] && { [ "$hmaj" -gt 0 ] || [ "$hmin" -ge 57 ]; }; then
      say "SKIP S-021 AC5 (b) known limit on Hyprland >= 0.57 (S-105)"
    else
    check "S-021 AC5 (b) drop show, pointer at (40, 40) outside the hive: a click there reaches keys.py once and the catcher stays bound" \
      '[ "$bclicks" = 1 ] && [ "$bstay" = yes ]'
    fi
    probe_checks "AC5 (b) pointer into the rectangle" "$bseg" pointer
    # S-019 decision 12, the same show (the trace lines, not after()), with
    # the S-021 AC4 reference: the catcher's ready commit (its first commit
    # after the first configure), as no hive is drawn in a show with no drag.
    local sdt=none sent sfirst=none slast sreg=none ssw
    [ -n "$bseg" ] && IFS='|' read -r sdt sent sfirst slast sreg ssw <<<"$(still_region "$bseg")"
    say "INFO drop show, still pointer, no --at (decision 12): catcher's first wl_region ${sdt} ms after the catcher's ready commit, that region (adds, last add) ${sfirst}, last catcher region ${slast:-none}, last hive surface ${sreg}, enters on the catcher before the region ${sent:-?}"
    check "S-019 decision 12, S-021 AC4 drop show, still pointer, no --at: the catcher's input region is the hive rectangle (one add, the last \"hive surface\" line, also its last region) within 400 ms of the catcher's ready commit (first commit after the first configure)" \
      '[ "$sreg" != none ] && [ "$sfirst" = "1 $sreg" ] && [ "$slast" = "1 $sreg" ] && within_ms "$sdt" 400'

    # AC5 (a): `drop --at` the cursor, the pointer still inside the hive
    # rectangle; a 5 px move brings Hyprland's wl_pointer.enter (S-021 P1):
    # "hidden (pointer)", the layer destroyed within 500 ms of that enter by
    # the trace's stamps, and a click after the move reaches keys.py once.
    local aclicks=none agone=none aseg=""
    centre_ptr
    show "$BIN/kawaro" drop --at "$(hyprctl cursorpos)"
    sleep 0.5
    move $((hcx + 5)) "$hcy"; sleep 0.6
    k0=$(wc -l <"$KLOG")
    click $((hcx + 5)) "$hcy"; sleep 0.6
    aclicks=$(sed -n "$((k0 + 1)),\$p" "$KLOG" | grep -c '^keys: click$')
    aseg=$(seg_from "$MARK")
    agone=$(enter_to_gone "$aseg")
    reset_hive "the drop --at show (a)"
    say "INFO S-021 AC5 (a): layer destroyed ${agone} ms after the catcher's wl_pointer.enter, keys.py clicks after the move ${aclicks}"
    check "S-021 AC5 (a) \`drop --at\` the cursor, then a 5 px move: the layer is destroyed within 500 ms of the catcher's wl_pointer.enter and a click after the move reaches keys.py once" \
      'within_ms "$agone" 500 && [ "$aclicks" = 1 ]'
    probe_checks "AC5 (a) drop --at, 5 px move" "$aseg" pointer

    # AC9, first half of the cycle, on one daemon: (1) `drop --hold --at` the
    # cursor as the documented binding runs it, the layer bound, a 5 px move:
    # "hidden (pointer)"; then `drop --release` after that hide exits 0
    # within 200 ms and binds nothing for 1000 ms; (2) AC5 (c): `drop --hold`,
    # the layer bound, the pointer still, `drop --release`: "hidden (release)".
    local m91 m92 m93 h1seen=no r1rc r1ms r1new=none t0 seg91="" seg93="" h2seen=no r2rc r2ms h2gone=no
    centre_ptr
    m91=$(wc -l <"$DLOG")
    "$BIN/kawaro" drop --hold --at "$(hyprctl cursorpos)" >>"$log-cli.log" 2>&1
    wait_shown 1500 && h1seen=yes
    sleep 0.3
    move $((hcx + 5)) "$hcy"; sleep 0.6
    seg91=$(seg_from "$m91")
    m92=$(wc -l <"$DLOG"); t0=$(now_us)
    "$BIN/kawaro" drop --release >>"$log-cli.log" 2>&1; r1rc=$?; r1ms=$(( ($(now_us) - t0) / 1000 ))
    sleep 1
    r1new=$(new_surfaces "$(seg_from "$m92")")
    reset_hive "the AC9 hold (1)"
    centre_ptr
    m93=$(wc -l <"$DLOG")
    "$BIN/kawaro" drop --hold >>"$log-cli.log" 2>&1
    wait_shown 1500 && h2seen=yes
    sleep 0.3
    t0=$(now_us)
    "$BIN/kawaro" drop --release >>"$log-cli.log" 2>&1; r2rc=$?; r2ms=$(( ($(now_us) - t0) / 1000 ))
    gone_within 5 && h2gone=yes
    seg93=$(seg_from "$m93")
    reset_hive "the AC9 hold (2)"
    say "INFO S-021 AC9 (1): layer bound $h1seen, probe line $(grep -E '^hive (reveal|probe):' <<<"$seg91" | tr '\n' ' '); release after the hide: exit $r1rc in $r1ms ms, new surfaces in 1000 ms $r1new; (2) AC5 (c): layer bound $h2seen, release exit $r2rc in $r2ms ms, gone within 500 ms $h2gone, probe line $(grep -E '^hive (reveal|probe):' <<<"$seg93" | tr '\n' ' ')"
    # S-095 (manager ruling 2026-10-30): the hive may be shown once more after
    # this hide while the key is held, so a drag that starts later finds it.
    # What the check protects is no flicker loop: one probe line and at most
    # one re-show (an anchor and a surface line) with the pointer still.
    PROBE=$(probe_show "$seg91")
    local pall prev phid pn pr pcells ppre patt pcatt pe1 pa1 pafter pchild
    IFS='|' read -r pall prev phid pn pr pcells ppre patt pcatt pe1 pa1 pafter pchild <<<"$PROBE"
    say "INFO S-021 AC9 (1) drop --hold --at, 5 px move: probe lines $pall, hide reason $pr, anchor or surface lines after the hide $pafter (at most 2: one re-show), hive attach $patt, catcher attach $pcatt"
    check "S-021 AC3 AC9 (1) drop --hold --at, 5 px move: exactly one probe line, \"hive probe: hidden (pointer)\", no \"hive reveal:\" line, and at most one re-show (2 anchor or surface lines) after it (S-095)" \
      '[ "$pall" -eq 1 ] && [ "$phid" -eq 1 ] && [ "$pr" = pointer ] && [ "$pafter" -le 2 ]'
    check "S-021 AC4 AC9 (1) drop --hold --at, 5 px move: no attach on the hive subsurface in the whole show, and the catcher took its pixel" \
      '[ "$patt" -eq 0 ] && [ "$pcatt" -ge 1 ]'
    check "S-021 AC9 (1): \`drop --release\` after that hide exits 0 within 200 ms and binds no new surface for 1000 ms" \
      '[ "$h1seen" = yes ] && [ "$r1rc" = 0 ] && [ "$r1ms" -le 200 ] && [ "$r1new" = 0 ]'
    check "S-021 AC5 (c): \`drop --hold\` with the pointer still, the layer bound, then \`drop --release\` (exit 0 within 200 ms) removes it within 500 ms" \
      '[ "$h2seen" = yes ] && [ "$r2rc" = 0 ] && [ "$r2ms" -le 200 ] && [ "$h2gone" = yes ]'
    probe_checks "AC5 (c) and AC9 (2) drop --hold, release" "$seg93" release

    # S-022 AC7 (backlog/S-022-drop-followups.md): a new drop show while the
    # last one still probes. The pointer still at (40, 40), `drop --at
    # 500,300` twice 1 s apart (the catcher's region is the hive rectangle
    # from its first commit, so the still pointer is outside it), then the
    # pointer moves to (500, 300): exactly two probe lines, "hidden
    # (replaced)" for the first show, printed between the two shows' "hive
    # surface:" lines, then "hidden (pointer)", and nothing drawn
    # (layer.sh replace_checks, over one segment for both shows).
    local m7 seg7=""
    move 40 40; sleep 0.3
    m7=$(wc -l <"$DLOG")
    "$BIN/kawaro" drop --at 500,300 >>"$log-cli.log" 2>&1
    sleep 1
    "$BIN/kawaro" drop --at 500,300 >>"$log-cli.log" 2>&1
    sleep 1
    for i in 1 2 3 4 5 6 7 8; do move $((40 + i * 460 / 8)) $((40 + i * 260 / 8)); sleep 0.05; done
    sleep 0.8
    seg7=$(seg_from "$m7")
    reset_hive "the S-022 AC7 pair"
    replace_checks "pointer at (40, 40), drop --at 500,300 twice, then a move there" "$seg7"
  fi

  # Drop mode mid-drag, as the documented Super+Z and Alt-hold bindings do: a
  # GTK drag source (tests/drop/source.py) drags a PNG and `kawaro drop`
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
  SRCPID=$!
  pids+=("$!")
  sleep 2
  # begin_drag: press on the source and pass GTK's threshold (lib.sh).
  begin_drag() { drag_past_threshold; }
  if [ "$ALTDRAG" = 1 ]; then
    # shellcheck source=altdrag.sh
    source "$HERE/altdrag.sh"
    altdrag_run
    finish
    say "RESULT $([ "$FAILS" -eq 0 ] && echo PASS || echo "FAIL ($FAILS)")"
    echo "{\"total\": $TOTAL, \"failed\": $FAILS, \"skipped\": 0}" >"$XDG_RUNTIME_DIR/json"
    [ "$FAILS" -eq 0 ]
    return
  fi
  # Layer mode only (S-021 decision 1).
  if [ "$SURFACE" = layer ]; then
    # S-021 AC9 (3), the end of the cycle on the same daemon: a PNG drag, then
    # `drop --hold` with the hand still; the drag moves into the centred hive
    # (Hyprland sends wl_data_device.enter with the first motion, S-021 P3):
    # "hive reveal: N". Then the drag leaves the hive (it stays while the key
    # is held, S-019 decision 3), the button is let go over the source, and
    # `drop --release` closes it. Releasing the key with the drag still over
    # the hive is S-022 AC1, below.
    local m94 seg94="" h3seen=no
    cp "$ROOT/tests/fixtures/images/alpha.png" "$W/cycle.png"
    printf '%s\n' "$W/cycle.png" >"$NEXT"
    begin_drag
    m94=$(wc -l <"$DLOG")
    "$BIN/kawaro" drop --hold >>"$log-cli.log" 2>&1
    wait_shown 1500 && h3seen=yes
    sleep 0.5
    for i in 1 2 3 4 5 6; do move $((210 + i * (hcx - 210) / 6)) $((180 + i * (hcy - 180) / 6)); sleep 0.05; done
    sleep 0.8
    move 60 60; sleep 0.3; echo "up" >&3; sleep 0.5
    "$BIN/kawaro" drop --release >>"$log-cli.log" 2>&1
    sleep 0.5
    seg94=$(seg_from "$m94")
    gone_within 30 || true
    reset_hive "the AC9 hold (3)"
    say "INFO S-021 AC9 (3): layer bound $h3seen, probe line $(grep -E '^hive (reveal|probe):' <<<"$seg94" | tr '\n' ' ')"
    probe_checks "AC9 (3) PNG drag, drop --hold" "$seg94" reveal
    check "S-021 AC9: no protocol error in the daemon log after the cycle (hidden (pointer), hidden (release), then a reveal)" \
      '! grep -qE "wl_display[#@]1\.error\(|[Pp]rotocol error" "$DLOG"'

    # S-022 AC1: `drop --release` with the drag over a revealed hive, then a
    # new drag. A PNG drag, `drop --hold`, the drag into the centred hive
    # ("hive reveal: N") and over its WEBP cell; `drop --release` there: the
    # layer is destroyed within 500 ms of the release (a UTC stamp before
    # the command against the trace's stamps), and the button let go in
    # place writes no WEBP. Then a second PNG drag, `drop --hold` and the
    # same moves: its enter is read (one "drop cells:" line, "hive reveal:
    # N", N the 15 actions of a PNG, an accept naming a type), the drop on
    # WEBP writes the second file, and the daemon log has no protocol error.
    # On 2026-10-22 (S-021b) the second show's enter went unread.
    local nact1 m1a m1b mrel t1 g1 gone1=none web1=no seg1a="" seg1b=""
    nact1=$("$BIN/kawaro" actions "$W/photo.png" 2>/dev/null | grep -c .)
    # held_over_webp FILE: a drag of FILE, `drop --hold`, into the hive and
    # over its WEBP cell (MARK is the show's start).
    held_over_webp() {
      local gx gy gw gh
      cp "$ROOT/tests/fixtures/images/alpha.png" "$1"
      printf '%s\n' "$1" >"$NEXT"
      begin_drag
      show "$BIN/kawaro" drop --hold
      read -r gx gy gw gh < <(geo)
      if [ -n "${gw:-}" ]; then
        sleep 0.5
        move $((gx + gw / 2)) $((gy + gh / 2)); sleep 0.5
        read -r gx gy gw gh < <(geo)
        move_to_webp "$gx" "$gy" "$gw" "$gh"
      fi
    }
    held_over_webp "$W/first.png"
    m1a=$MARK
    sleep 0.3
    mrel=$(wc -l <"$DLOG"); t1=$(date -u +%T.%6N)
    "$BIN/kawaro" drop --release >>"$log-cli.log" 2>&1
    sleep 0.7
    echo "up" >&3
    sleep 1.5
    [ -e "$W/first.webp" ] && web1=yes
    g1=$(seg_from "$mrel" | grep -m1 -E -- '-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\(' | grep -oE '^\[[0-9:.]+\]')
    [ -n "$g1" ] && gone1=$(awk -v a="$(tod "$t1")" -v b="$(tod "$g1")" 'BEGIN { d = b - a; if (d < -43200) d += 86400; printf "%.1f\n", d * 1000 }')
    gone_within 30 || true
    reset_hive "the S-022 AC1 release over the hive"
    held_over_webp "$W/second.png"
    m1b=$MARK
    echo "up" >&3
    for i in $(seq 30); do [ -s "$W/second.webp" ] && break; sleep 0.1; done
    sleep 0.3
    "$BIN/kawaro" drop --release >>"$log-cli.log" 2>&1
    sleep 0.5
    seg1a=$(sed -n "$((m1a + 1)),${m1b}p" "$DLOG")
    seg1b=$(seg_from "$m1b")
    gone_within 30 || true
    reset_hive "the S-022 AC1 drag after it"
    say "INFO S-022 AC1: first drag, layer destroyed ${gone1} ms after \`drop --release\`, first.webp written $web1; second drag: $(grep -E '^(drop cells:|hive (reveal|probe):)' <<<"$seg1b" | tr '\n' '|')"
    probe_checks "S-022 AC1 first drag, drop --hold, over WEBP" "$seg1a" reveal
    local r1n; IFS='|' read -r _ _ _ r1n _ <<<"$PROBE"
    check "S-022 AC1 first drag: \"hive reveal: N\" with N the $nact1 lines of \`kawaro actions\`, then \`drop --release\` over the WEBP cell destroys the layer within 500 ms and the button let go there writes no WEBP" \
      '[ "$r1n" != none ] && [ "$r1n" -eq "$nact1" ] && within_ms "$gone1" 500 && [ "$web1" = no ]'
    again_checks "AC1 second drag after the release" "$seg1b" "$nact1" "$W/second.webp"
    check "S-022 AC1: no protocol error in the daemon log after the two drags" \
      '! grep -qE "wl_display[#@]1\.error\(|[Pp]rotocol error" "$DLOG"'

    # S-022 AC4: the opening animation starts at the reveal. A PNG drag,
    # `kawaro drop` with the drag still, then one move to the centred
    # hive's centre (no cell lit), where it rests 400 ms. The attach count
    # on the hive subsurface in the 200 ms from its first one (layer.sh
    # attach_burst): 4 or more on this daemon (motion), 2 or fewer on one
    # with KAWARO_REDUCED_MOTION=1. A normal show with motion gives the
    # calibration count (INFO). The daemon is then started again with
    # motion, as before.
    local PVSEG seg4m seg4r seg4n b4m b4r b4n
    centre_rest() {
      local gx gy gw gh
      cp "$ROOT/tests/fixtures/images/alpha.png" "$1"
      printf '%s\n' "$1" >"$NEXT"
      begin_drag
      show "$BIN/kawaro" drop
      read -r gx gy gw gh < <(geo)
      sleep 0.6
      [ -n "${gw:-}" ] && move $((gx + gw / 2)) $((gy + gh / 2))
      sleep 0.4
      PVSEG=$(seg_from "$MARK")
      echo "up" >&3; sleep 0.5
      gone_within 30 || true
      reset_hive "the S-022 AC4 show"
    }
    centre_rest "$W/motion.png"; seg4m=$PVSEG
    centre_ptr
    show "$BIN/kawaro" open "$W/photo.png"
    sleep 0.6
    seg4n=$(seg_from "$MARK")
    key 1; gone_within 15 || true
    reset_hive "the S-022 AC4 normal show"
    stop_daemon
    start_daemon 1 1
    centre_rest "$W/calm.png"; seg4r=$PVSEG
    stop_daemon
    start_daemon 1 0
    b4m=$(attach_burst "$seg4m" 200); b4r=$(attach_burst "$seg4r" 200); b4n=$(attach_burst "$seg4n" 200)
    say "INFO S-022 AC4: hive subsurface attach in the 200 ms from the first: drop reveal with motion $b4m, with KAWARO_REDUCED_MOTION=1 $b4r; calibration, a normal show with motion $b4n"
    check "S-022 AC4: a revealed drop show with motion draws its opening animation: 4 or more attach on the hive subsurface in the 200 ms from its first" \
      '[ "$b4m" != none ] && [ "$b4m" -ge 4 ]'
    check "S-022 AC4: with KAWARO_REDUCED_MOTION=1 the reveal draws 1 or 2 attach in those 200 ms (no animation)" \
      '[ "$b4r" != none ] && [ "$b4r" -ge 1 ] && [ "$b4r" -le 2 ]'
  fi

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
    # HYPR_DROP_VIA=exec runs it as the documented binding does, through
    # Hyprland's exec (which hands the process an activation token).
    if [ "${HYPR_DROP_VIA:-}" = exec ]; then show hyprctl dispatch "hl.dsp.exec_cmd(\"$BIN/kawaro drop\")"
    else show "$BIN/kawaro" drop; fi
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
    say "INFO drag $c: under way $dragging; drop hive ${gw:+at $gx $gy $gw $gh} in ${LAT:-none} ms; enter at ${enter:-none} (surface-local); cursor over WEBP at ${cur:-?}; motion events $MOTIONS; kawarod ${cdrag:-?}% CPU; file written $WROTE; hive gone $CLOSED; source: $(sed -n "$((s0 + 1)),$((s1))p" "$SLOG" | grep -oE 'drag-(end|cancel).*' | tr '\n' ' ')"
  }
  drag_case cell
  # S-019f: this show's trace, for the quasimode preview check.
  local CELL_SEG; CELL_SEG=$(sed -n "$((MARK + 1)),\$p" "$DLOG")
  check "drop mid-drag: the hive gets drag motion events" '[ "$MOTIONS" -gt 0 ]'
  check "drop mid-drag: released over the WEBP cell writes the file" '[ "$WROTE" = yes ]'
  check "drop mid-drag: a new drag begins afterwards (input not stuck)" '[ "$AFTER" = yes ]'
  drag_case timeout
  local TIMEOUT_SEG; TIMEOUT_SEG=$(sed -n "$((MARK + 1)),\$p" "$DLOG")
  check "drop timeout mid-drag: the hive closes and a new drag begins" '[ "$CLOSED" = yes ] && [ "$AFTER" = yes ]'
  drag_case esc
  local ESC_SEG; ESC_SEG=$(sed -n "$((MARK + 1)),\$p" "$DLOG")
  check "Esc mid-drag: the hive closes and a new drag begins" '[ "$CLOSED" = yes ] && [ "$AFTER" = yes ]'

  # Layer mode only (S-021 decision 1).
  if [ "$SURFACE" = layer ]; then
    # S-021 AC7, the "cell" drag: one "hive reveal: N" line with N the lines
    # of `kawaro actions` for the PNG, the hive drawn only after the drag's
    # first enter on the catcher; the WEBP write is the check above.
    local nact; nact=$("$BIN/kawaro" actions "$W/photo.png" 2>/dev/null | grep -c .)
    probe_checks "AC7 cell drag" "$CELL_SEG" reveal
    local cn; IFS='|' read -r _ _ _ cn _ <<<"$PROBE"
    say "INFO S-021 AC7 cell: hive reveal N $cn, kawaro actions lines $nact"
    check "S-021 AC7 cell drag: \"hive reveal: N\" with N the number of \`kawaro actions\` lines for the PNG ($nact)" \
      '[ "$cn" != none ] && [ "$nact" -ge 1 ] && [ "$cn" -eq "$nact" ]'
    # AC3 for the timeout and Esc drags: each revealed, one probe line.
    local tall trev eall erev
    IFS='|' read -r tall trev _ <<<"$(probe_show "$TIMEOUT_SEG")"
    IFS='|' read -r eall erev _ <<<"$(probe_show "$ESC_SEG")"
    say "INFO S-021 AC3 timeout drag: probe lines $tall (reveal $trev); Esc drag: probe lines $eall (reveal $erev)"
    check "S-021 AC3 timeout and Esc drags: each prints exactly one probe line, \"hive reveal: N\"" \
      '[ "$tall" -eq 1 ] && [ "$trev" -eq 1 ] && [ "$eall" -eq 1 ] && [ "$erev" -eq 1 ]'
    # S-021 AC8 (the maintainer's budget, question 1) and S-022 AC5: the eight
    # revealed shows (AC9 (3), the two S-022 AC1 drags, the two S-022 AC4
    # shows, cell, timeout, Esc) are visible, the hive's first attach, at
    # most 150 ms after the drag's first enter on the catcher, by the
    # daemon's trace timestamps.
    reveal_budget "AC9 (3), S-022 AC1 (two) and AC4 (two), cell, timeout and Esc drags" 8 \
      "$(reveal_ms "${seg94:-}")" "$(reveal_ms "${seg1a:-}")" "$(reveal_ms "${seg1b:-}")" "$(reveal_ms "${seg4m:-}")" "$(reveal_ms "${seg4r:-}")" \
      "$(reveal_ms "$CELL_SEG")" "$(reveal_ms "$TIMEOUT_SEG")" "$(reveal_ms "$ESC_SEG")"

    # S-021 AC6: a drag the hive has nothing for. no_drop KIND sets NSEG (the
    # show's trace), NNEW (files new in the work folder) and NAFTER (a new drag
    # began afterwards). KIND file drags notes.unknownext (no action), text a
    # plain string (source.py "text:", no text/uri-list). The drag stands
    # still at `kawaro drop`, then moves into the centred hive's rectangle.
    local NSEG NNEW NAFTER
    no_drop() {
      local before s1 i
      if [ "$1" = file ]; then
        printf 'x\n' >"$W/notes.unknownext"
        printf '%s\n' "$W/notes.unknownext" >"$NEXT"
      else
        printf 'text:%s\n' "kawaro drop text" >"$NEXT"
      fi
      before=$(find "$W" -type f | sort)
      begin_drag
      show "$BIN/kawaro" drop
      sleep 0.5
      for i in 1 2 3 4 5 6; do move $((210 + i * (hcx - 210) / 6)) $((180 + i * (hcy - 180) / 6)); sleep 0.05; done
      sleep 0.8
      echo "up" >&3; sleep 1.5
      NSEG=$(seg_from "$MARK")
      NNEW=$(comm -13 <(echo "$before") <(find "$W" -type f | sort) | grep -c .)
      gone_within 130 || true
      s1=$(wc -l <"$SLOG")
      begin_drag; echo "up" >&3; sleep 0.5
      NAFTER=no; sed -n "$((s1 + 1)),\$p" "$SLOG" | grep -q drag-begin && NAFTER=yes
      gone_within 130 || true
      reset_hive "the $1 drag"
    }
    no_drop file
    local fcl facc
    fcl=$(grep -E '^drop cells:' <<<"$NSEG" | tr '\n' '|')
    facc=$(grep -cE -- '-> wl_data_offer[#@][0-9]+\.accept\([0-9]+, "' <<<"$NSEG")
    say "INFO S-021 AC6 notes.unknownext: drop cells lines ${fcl:-none}, accept with a type $facc, new files $NNEW, a new drag afterwards $NAFTER"
    check "S-021 AC6 notes.unknownext: an empty \"drop cells:\" line before the hide, no wl_data_offer.accept with a type, no new file, and a new drag begins afterwards" \
      '[ "$fcl" = "drop cells:|" ] && [ "$(grep -nE "^drop cells: ?$" <<<"$NSEG" | head -1 | cut -d: -f1)" -lt "$(grep -nE "^hive probe: " <<<"$NSEG" | head -1 | cut -d: -f1)" ] && [ "$facc" -eq 0 ] && [ "$NNEW" -eq 0 ] && [ "$NAFTER" = yes ]'
    probe_checks "AC6 notes.unknownext drag" "$NSEG" no-actions
    no_drop text
    local trcv; trcv=$(grep -cE -- '-> wl_data_offer[#@][0-9]+\.receive\(' <<<"$NSEG")
    say "INFO S-021 AC6 text drag: offers $(grep -oE 'wl_data_offer[#@][0-9]+\.offer\("[^"]*"' <<<"$NSEG" | sed 's/.*offer(//' | sort -u | tr '\n' ' '), receive $trcv, new files $NNEW, a new drag afterwards $NAFTER"
    check "S-021 AC6 text drag (no text/uri-list): no wl_data_offer.receive, and the drag entered the catcher" \
      '[ "$trcv" -eq 0 ] && grep -qE "^\[[0-9:.]+\] wl_data_device[#@][0-9]+\.enter\(" <<<"$NSEG"'
    probe_checks "AC6 text drag" "$NSEG" no-uri-list
  fi

  if [ "$QUASI" = 1 ]; then
    # Key release hides (S-019e): the press, the threshold, the release.
    gone_within 120 || true
    reset_hive "the drag cases"
    centre_ptr
    local hm hrc hms rrc rms pre300=no post=no hgone=no t0 t1
    hm=$(wc -l <"$DLOG"); t0=$(now_us)
    "$BIN/kawaro" drop --hold >>"$log-cli.log" 2>&1; hrc=$?; hms=$(( ($(now_us) - t0) / 1000 ))
    while [ $(( ($(now_us) - t0) / 1000 )) -lt 300 ]; do sleep 0.02; done
    shown && pre300=yes
    while [ $(( ($(now_us) - t0) / 1000 )) -lt 1000 ]; do shown && { post=yes; break; }; sleep 0.05; done
    sleep 0.3
    t1=$(now_us)
    "$BIN/kawaro" drop --release >>"$log-cli.log" 2>&1; rrc=$?; rms=$(( ($(now_us) - t1) / 1000 ))
    gone_within 5 && hgone=yes
    say "INFO quasimode key release: press exit $hrc in $hms ms, layer at 300 ms $pre300, layer by 1000 ms $post, release exit $rrc in $rms ms, gone within 500 ms $hgone, get_layer_surface $(sed -n "$((hm + 1)),\$p" "$DLOG" | grep -cE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(')"
    reset_hive "the hold show"
    check "quasimode: key release hides (S-019 AC11 to AC13): \`drop --hold\` exits 0 within 200 ms, no layer at 300 ms, one by 1000 ms, and \`drop --release\` (exit 0 within 200 ms) removes it within 500 ms" \
      '[ "$hrc" = 0 ] && [ "$hms" -le 200 ] && [ "$pre300" = no ] && [ "$post" = yes ] && [ "$rrc" = 0 ] && [ "$rms" -le 200 ] && [ "$hgone" = yes ]'
    # A tap: the release 150 ms after the press.
    local tseen=no tgls
    hm=$(wc -l <"$DLOG"); t0=$(now_us)
    "$BIN/kawaro" drop --hold >>"$log-cli.log" 2>&1; hrc=$?; hms=$(( ($(now_us) - t0) / 1000 ))
    while [ $(( ($(now_us) - t0) / 1000 )) -lt 150 ]; do sleep 0.01; done
    t1=$(now_us)
    "$BIN/kawaro" drop --release >>"$log-cli.log" 2>&1; rrc=$?; rms=$(( ($(now_us) - t1) / 1000 ))
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
    local nact; nact=$("$BIN/kawaro" actions "$W/photo.png" 2>/dev/null | grep -c .)
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

    # Hive near pointer, 1: `kawaro open` while the pointer moves 1 px
    # around P (Hyprland logical pixels).
    gone_within 120 || true
    local i px=300 py=200
    move "$px" "$py"; sleep 0.5
    MARK=$(wc -l <"$DLOG")
    for ((i = 0; i < 60; i++)); do
      if ((i % 2)); then move $((px + 1)) $((py + 1)); else move "$px" "$py"; fi
      echo "sleep 10" >&3
    done
    show "$BIN/kawaro" open "$W/photo.png"
    sleep 1
    local nseg; nseg=$(after "$MARK")
    local nax nay; read -r nax nay <<<"$(anchor_of "$nseg")"
    say "INFO quasimode near pointer (open): P ($px, $py), output ${lw}x${lh}, $(grep -m1 -E '^hive anchor: ' <<<"$nseg" || echo 'no anchor line'), $(grep -m1 -E '^hive surface: ' <<<"$nseg" || echo 'no surface line')"
    key 1; gone_within 30 || true
    reset_hive "the near pointer show"
    check "quasimode: hive near pointer (S-019 AC5, AC6): \`kawaro open\` with the pointer moving at ($px, $py) is anchored within 1 px of it (or of it + 1, the jitter's other point) and placed by plan_at for that anchor" \
      'near_jit "$nax" "$px" && near_jit "$nay" "$py" && placed "$nseg" "$lw" "$lh"'

    # Hive near pointer, 2: decision 1, a drag held still and
    # `kawaro drop --at "$(hyprctl cursorpos)"` (no wait, no move).
    local s0; s0=$(wc -l <"$SLOG")
    cp "$ROOT/tests/fixtures/images/alpha.png" "$W/held.png"
    printf '%s\n' "$W/held.png" >"$NEXT"
    begin_drag
    move 500 300; sleep 0.5
    local cur; cur=$(hyprctl cursorpos | tr -d ' ')
    show "$BIN/kawaro" drop --at "$(hyprctl cursorpos)"
    sleep 1
    local dseg; dseg=$(after "$MARK")
    local dax day; read -r dax day <<<"$(anchor_of "$dseg")"
    local dsets; dsets=$(sed -n "$((MARK + 1)),\$p" "$DLOG" | grep -cE -- '-> wl_subsurface[#@][0-9]+\.set_position\(')
    say "INFO quasimode near pointer (drop --at): cursorpos $cur, $(grep -E '^hive anchor: ' <<<"$dseg" | tr '\n' ' ' || true)$(grep -m1 -E '^hive surface: ' <<<"$dseg" || echo 'no surface line'), set_position x$dsets"
    echo "up" >&3
    gone_within 130 || true
    reset_hive "the drop --at show"
    check "quasimode: hive near pointer (S-019 decision 1): a still drag and \`kawaro drop --at \"\$(hyprctl cursorpos)\"\` anchor at the cursor (1 px) at once, with one set_position" \
      '[ "$(grep -cE "^hive anchor: " <<<"$dseg")" -eq 1 ] && near "$dax" "${cur%,*}" 1 && near "$day" "${cur#*,}" 1 && [ "$dsets" -eq 1 ] && placed "$dseg" "$lw" "$lh"'

    # Adaptive shape (S-019b, AC3, AC4): `kawaro open` with the pointer
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
    show "$BIN/kawaro" open "$W/photo.png"
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
    check "quasimode: adaptive shape (S-019b AC3, AC4): \`kawaro open\` with the pointer at the top left corner is anchored there (1 px from B or B + 1, the two jitter points), its \"hive surface\" box and \"hive cells\" count and centre equal plan_at by tests/overlay/anchor.py (1 px), and the box lies inside the ${lw}x${lh} output" \
      'near_jit "$bax" "$bx" && near_jit "$bay" "$by" && placed "$bseg" "$lw" "$lh" && [ "$bcells" = yes ] && [ "$bsx" -ge 0 ] && [ "$bsy" -ge 0 ] && [ $((bsx + bsw)) -le "$lw" ] && [ $((bsy + bsh)) -le "$lh" ]'

    # Screen reader rule (S-019a, AC1, AC2), last because it replaces the
    # daemon: one daemon with no KAWARO_SURFACE (config hive_surface absent,
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
    KAWARO_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/kawarod" >>"$DLOG" 2>&1 &
    d=$!
    pids+=("$d")
    for _ in $(seq 50); do "$BIN/kawaro" ping >/dev/null 2>&1 && break; sleep 0.1; done
    centre_ptr
    show "$BIN/kawaro" open "$W/photo.png"
    sleep 0.5; key 1; sleep 1
    am2=$(wc -l <"$DLOG")
    echo orca >&6
    timeout 10 gdbus wait --session org.gnome.Orca.Service || say "INFO fake org.gnome.Orca.Service did not appear"
    sleep 1
    centre_ptr
    show "$BIN/kawaro" open "$W/photo.png"
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
    say "INFO quasimode: same output as pointer (S-019 AC5): \`kawaro open\` on the second output is checked by tests/overlay/kwin.sh outputs"
    say "INFO quasimode: same output as pointer (S-019 decision 1): \`kawaro drop --at\` with global coordinates on the second output is checked by tests/overlay/kwin.sh outputs"
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
if [ "$PORTAL" = 1 ]; then
  # S-014e3: the real portal on the private bus; the app's desktop entry
  # where an install puts it, in the throwaway data directory.
  mkdir -p "$SANDBOX/.local/share/applications"
  cp "$ROOT/packaging/io.github.m7mmad_omar.Kawaro.desktop" "$SANDBOX/.local/share/applications/"
else
  no_real_portal "$SANDBOX"
fi
# Short on purpose: Hyprland's Wayland backend needs socket paths under 108 bytes.
RUNTIME="$(mktemp -d /tmp/sbhh.XXXXXX)"
chmod 700 "$RUNTIME"
mkdir -p "$SANDBOX/.config" "$SANDBOX/.local/share" "$SANDBOX/.cache" "$SANDBOX/.local/state"
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates can reach the live desktop or the real profile.
ENVS=(-u DISPLAY -u WAYLAND_DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP
  -u XDG_SESSION_DESKTOP -u DESKTOP_SESSION -u XDG_ACTIVATION_TOKEN -u DESKTOP_STARTUP_ID -u KAWARO_SURFACE
  -u KAWARO_REDUCED_MOTION
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share"
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" XDG_RUNTIME_DIR="$RUNTIME"
  GIO_USE_VFS=local GVFS_DISABLE_FUSE=1)
if [ "$PORTAL" = 1 ]; then
  # --portal: the services the bus activates (the portal and its backend)
  # inherit its stdout, so theirs goes to a log; inner's lines go to fd 4
  # (through fd 5), as in the other modes.
  env "${ENVS[@]}" nice -n 19 dbus-run-session -- bash -c 'inner >&5' \
    5>&4 >"$OUT/hypr-portal-bus-stdout.log" 2>"$OUT/hypr-bus.log"
else
  env "${ENVS[@]}" nice -n 19 dbus-run-session -- bash -c inner >&4 2>"$OUT/hypr-bus.log"
fi
rc=$?
if [ "$JSON" = 1 ]; then
  if [ -s "$RUNTIME/json" ]; then cat "$RUNTIME/json"; else echo '{"total": 0, "failed": 1, "skipped": 0}'; rc=1; fi
fi
cp "$RUNTIME"/hypr/*/hyprland.log "$OUT/hypr-hyprland-full.log" 2>/dev/null
# --portal: the document portal's FUSE mount, if the private bus started it.
fusermount3 -u "$RUNTIME/doc" 2>/dev/null
rm -rf "$SANDBOX" "$RUNTIME" 2>/dev/null || { sleep 1; rm -rf "$SANDBOX" "$RUNTIME"; }
exit "$rc"
