#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-114: the hive in a private X11 session. Starts a virtual X server and the
# xfwm4 window manager (no compositor) on a session bus of its own, with a
# temporary HOME and runtime directory, then measures the hive for real:
# placement at the pointer, the choice line, focus, dismissal, the plate pixel,
# the 150 ms budget and the 40 MB budget. It never touches the live desktop.
#
# A manual release step (run by hand before the 0.5.0 tag, output kept in the
# release notes); cargo test and CI never call it. See tests/x11/README.md.
#
# Usage: tests/x11/run.sh
#   KAWARO_BIN_DIR  where kawarod and kawaro are (default target/release).
# Run it inside the scope from CLAUDE.md ("Running heavy work").
set -u

if ! command -v Xvfb >/dev/null 2>&1; then
  echo "SKIP: Xvfb not found"
  exit 0
fi
if ! command -v xfwm4 >/dev/null 2>&1; then
  echo "SKIP: xfwm4 not found"
  exit 0
fi

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
# shellcheck source=tests/x11/lib.sh
source "$HERE/lib.sh"

now_ms() { local t="${EPOCHREALTIME/./}"; echo $((t / 1000)); }

# ------------------------------------------------------------------ outer ---
if [ "${1:-}" != "--inner" ]; then
  REAL_HOME="$HOME"
  BIN="${KAWARO_BIN_DIR:-$ROOT/target/release}"
  for b in kawarod kawaro; do
    [ -x "$BIN/$b" ] || { echo "ERROR: $BIN/$b missing; run cargo build --release first" >&2; exit 2; }
  done
  mkdir -p "$REAL_HOME/.cache"
  TMP="$(mktemp -d "$REAL_HOME/.cache/kawaro-x11.XXXXXX")" || exit 2
  cleanup_outer() { rm -rf "$TMP"; }
  trap cleanup_outer EXIT

  TOOL_TARGET="$REAL_HOME/.cache/kawaro-x11-tool-target"
  CARGO_TARGET_DIR="$REAL_HOME/.cache/kawaro-x11-tool-target" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}" \
    cargo build --quiet --release --manifest-path "$HERE/x11-tool/Cargo.toml" || { echo "ERROR: x11-tool did not build" >&2; exit 2; }

  mkdir -p "$TMP/home" "$TMP/run" "$TMP/tmp"
  chmod 700 "$TMP/run"
  export CARGO_HOME="${CARGO_HOME:-$REAL_HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$REAL_HOME/.rustup}"
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  export KAWARO_REAL_HOME="$REAL_HOME" KAWARO_X11_TMP="$TMP" KAWARO_X11_BIN="$BIN"
  export KAWARO_X11_TOOL="$TOOL_TARGET/release/x11-tool"
  export HOME="$TMP/home" TMPDIR="$TMP/tmp" XDG_RUNTIME_DIR="$TMP/run"
  export XDG_CONFIG_HOME="$TMP/home/.config" XDG_DATA_HOME="$TMP/home/.local/share"
  export XDG_CACHE_HOME="$TMP/home/.cache" XDG_STATE_HOME="$TMP/home/.local/state"
  export GDK_BACKEND=x11 XDG_SESSION_TYPE=x11
  unset WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE DISPLAY DBUS_SESSION_BUS_ADDRESS \
    XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID KAWARO_SURFACE

  dbus-run-session -- "$BASH" "$HERE/run.sh" --inner
  rc=$?

  alive=0
  if [ -f "$TMP/pids" ]; then
    while read -r p; do
      kill -0 "$p" 2>/dev/null && alive=$((alive + 1))
    done <"$TMP/pids"
  fi
  started="$(wc -l <"$TMP/pids" 2>/dev/null || echo 0)"
  if [ "$alive" -eq 0 ]; then
    echo "PASS AC2 isolation: $started started processes, 0 alive after exit"
  else
    echo "FAIL AC2 isolation: $alive of $started started processes still alive"
    rc=1
  fi
  if [ -s "$TMP/wayland-seen" ]; then
    echo "FAIL AC2 isolation: a wayland-* file appeared in the runtime dir"
    rc=1
  else
    echo "PASS AC2 isolation: no wayland-* file in the runtime dir at any 1 s check"
  fi
  rm -rf "$TMP"
  [ -d "$TMP" ] && { echo "FAIL AC2 isolation: temporary directory remains"; rc=1; } || echo "PASS AC2 isolation: temporary directory removed"
  exit "$rc"
fi

# ------------------------------------------------------------------ inner ---
TMP="$KAWARO_X11_TMP"
BIN="$KAWARO_X11_BIN"
TOOL="$KAWARO_X11_TOOL"
PIDS=()
FAILS=0
RAW="$TMP/daemon.log"
TSLOG="$TMP/daemon.ts.log"
: >"$RAW"
: >"$TSLOG"

pass() { echo "PASS $*"; }
fail() { FAILS=$((FAILS + 1)); echo "FAIL $*"; }
info() { echo "INFO $*"; }
track() { PIDS+=("$1"); echo "$1" >>"$TMP/pids"; }

stop_all() {
  local i p
  for ((i = ${#PIDS[@]} - 1; i >= 0; i--)); do
    p="${PIDS[$i]}"
    kill "$p" 2>/dev/null
  done
  for p in "${PIDS[@]}"; do
    for _ in $(seq 50); do
      kill -0 "$p" 2>/dev/null || break
      sleep 0.1
    done
    kill -0 "$p" 2>/dev/null && kill -9 "$p" 2>/dev/null
    wait "$p" 2>/dev/null
  done
}
trap stop_all EXIT

# Stamps every line of stderr with the time in ms: raw copy and stamped copy.
stamp() {
  local l t
  while IFS= read -r l; do
    t="${EPOCHREALTIME/./}"
    printf '%s\n' "$l" >>"$RAW"
    printf '%s %s\n' "${t:0:${#t}-3}" "$l" >>"$TSLOG"
  done
}

wait_until() { # wait_until TIMEOUT_MS cmd...
  local end=$(($(now_ms) + $1))
  shift
  while ! "$@"; do
    [ "$(now_ms)" -ge "$end" ] && return 1
    sleep 0.02
  done
}

MARK=0
mark() { MARK="$(wc -l <"$TSLOG")"; }
new_lines() { tail -n +$((MARK + 1)) "$TSLOG"; }
have_line() { new_lines | grep -qF "$1"; }
line_ts() { new_lines | grep -F -m1 "$1" | cut -d' ' -f1; }
line_text() { new_lines | grep -F -m1 "$1" | cut -d' ' -f2-; }

hive_gone() { ! "$TOOL" geometry Kawaro >/dev/null 2>&1; }
hive_shown() { "$TOOL" geometry Kawaro >/dev/null 2>&1; }
close_hive() {
  if hive_shown; then
    "$TOOL" key Escape >/dev/null 2>&1
    wait_until 2000 hive_gone
  fi
  return 0
}

# ---- AC2: the private X server and window manager ----
(while :; do
  if compgen -G "$XDG_RUNTIME_DIR/wayland-*" >/dev/null; then echo seen >>"$TMP/wayland-seen"; fi
  sleep 1
done) &
track $!

Xvfb -displayfd 3 -screen 0 1280x800x24 -nolisten tcp 3>"$TMP/display" 2>"$TMP/xvfb.log" &
track $!
wait_until 10000 test -s "$TMP/display" || { echo "ERROR: Xvfb did not start"; cat "$TMP/xvfb.log"; exit 2; }
dnum="$(head -1 "$TMP/display")"
export DISPLAY=":$dnum"
info "private X server on $DISPLAY (1280x800x24)"

xfwm4 --compositor=off >"$TMP/xfwm4.log" 2>&1 &
track $!
wait_until 10000 bash -c 'xprop -root _NET_SUPPORTING_WM_CHECK 2>/dev/null | grep -q "window id"' \
  || { echo "ERROR: xfwm4 did not become the window manager"; cat "$TMP/xfwm4.log"; exit 2; }
info "xfwm4 --compositor=off is the window manager"

# ---- AC3: the tool ----
if out="$("$TOOL" move 400 500)" && [ "$out" = "400 500" ]; then
  pass "AC3 tool: move 400 500 read back as '$out'"
else
  fail "AC3 tool: move 400 500 read back as '$out'"
fi

WORK="$HOME/work"
mkdir -p "$WORK"
cp "$ROOT/tests/fixtures/images/kawaro-sample.png" "$WORK/sample.png"
FILE="$WORK/sample.png"
files_now() { find "$WORK" -type f | wc -l; }
FILES0="$(files_now)"

# ---- the service, with its stderr read (KAWARO_DEBUG_FOCUS) ----
KAWARO_DEBUG_FOCUS=1 "$BIN/kawarod" 2> >(stamp) &
DPID=$!
track "$DPID"
wait_until 10000 compgen -G "$XDG_RUNTIME_DIR/*kawaro*" >/dev/null || sleep 2
sleep 1

open_hive() { env "$@" "$BIN/kawaro" open "$FILE" >/dev/null 2>&1; }

# ---- AC4: placement ----
for p in "400 500" "0 0" "1279 799"; do
  # shellcheck disable=SC2086
  "$TOOL" move $p >/dev/null
  mark
  open_hive
  if ! wait_until 3000 have_line "hive surface: window"; then
    fail "AC4 pointer ($p): no 'hive surface: window' line within 3 s"
    close_hive
    continue
  fi
  logged="$(x11_parse_surface "$(line_text 'hive surface: window')")"
  sleep 0.3
  read1="$("$TOOL" geometry Kawaro 2>&1)"
  sleep 0.5
  read2="$("$TOOL" geometry Kawaro 2>&1)"
  info "AC4 pointer ($p): logged '$logged', server '$read1', 500 ms later '$read2'"
  x11_geometry_match "$logged" "$read1" && pass "AC4 pointer ($p): log equals server geometry within 1 px" \
    || fail "AC4 pointer ($p): log '$logged' differs from server '$read1'"
  x11_box_inside "$read1" 1280 800 && pass "AC4 pointer ($p): box inside 1280x800" \
    || fail "AC4 pointer ($p): box '$read1' not inside 1280x800"
  [ "$read1" = "$read2" ] && pass "AC4 pointer ($p): window did not move after the first frame" \
    || fail "AC4 pointer ($p): window moved from '$read1' to '$read2'"
  close_hive
done

# ---- AC6: focus, 3 runs without a token and 3 with DESKTOP_STARTUP_ID ----
for variant in none startup-id; do
  for run in 1 2 3; do
    "$TOOL" move 640 400 >/dev/null
    mark
    if [ "$variant" = none ]; then open_hive; else open_hive DESKTOP_STARTUP_ID=kawaro-s114-test; fi
    wait_until 3000 have_line "hive focus: true"
    s="$(line_ts 'hive surface: window')"
    f="$(line_ts 'hive focus: true')"
    if [ -n "$s" ] && [ -n "$f" ] && [ $((f - s)) -le 300 ]; then
      pass "AC6 focus ($variant, run $run): hive focus: true $((f - s)) ms after the first frame"
    else
      fail "AC6 focus ($variant, run $run): surface ts '${s:-none}', focus ts '${f:-none}'"
    fi
    close_hive
  done
done

# ---- AC5: 20 shows, the choice line once; the service RSS ----
for _ in $(seq 20); do
  open_hive
  wait_until 3000 hive_shown
  close_hive
done
choices="$(x11_count_choice "$RAW")"
CHOICE='hive surface choice: window, not a Wayland session'
[ "$choices" = 1 ] && pass "AC5 '$CHOICE' appears $choices time over the whole run (20 shows)" \
  || fail "AC5 '$CHOICE' appears $choices times, want 1"
rss="$(awk '/VmRSS/{print $2}' "/proc/$DPID/status")"
if x11_rss_ok "${rss:-999999}"; then
  pass "AC9 service RSS after the shows is ${rss} KB (limit 40960)"
else
  fail "AC9 service RSS after the shows is ${rss:-unknown} KB (limit 40960)"
fi

# ---- AC7: dismissal ----
for run in 1 2 3; do
  "$TOOL" move 640 400 >/dev/null
  mark
  open_hive
  wait_until 3000 have_line "hive focus: true"
  t0="$(now_ms)"
  "$TOOL" key Escape >/dev/null
  if wait_until 500 hive_gone; then
    pass "AC7 Esc (run $run): hive closed $(($(now_ms) - t0)) ms after the key"
  else
    fail "AC7 Esc (run $run): hive still shown 500 ms after the key"
    close_hive
  fi
done
if command -v xmessage >/dev/null 2>&1; then
  for run in 1 2 3; do
    "$TOOL" move 640 400 >/dev/null
    mark
    open_hive
    wait_until 3000 have_line "hive focus: true"
    t0="$(now_ms)"
    xmessage -name s114-test -geometry +20+20 "focus target" >/dev/null 2>&1 &
    xp=$!
    track "$xp"
    if wait_until 500 hive_gone; then
      pass "AC7 focus move (run $run): hive closed $(($(now_ms) - t0)) ms after the test window started"
    else
      fail "AC7 focus move (run $run): hive still shown 500 ms after the test window started"
      close_hive
    fi
    kill "$xp" 2>/dev/null
    wait "$xp" 2>/dev/null
  done
else
  info "AC7 focus move: xmessage not found, this half is not measured"
fi
[ "$(files_now)" = "$FILES0" ] && pass "AC7 dismissal wrote 0 new files" || fail "AC7 dismissal wrote new files"

# ---- AC8: the plate ----
xsetroot -solid '#ff00ff'
"$TOOL" move 640 400 >/dev/null
mark
open_hive
if wait_until 3000 have_line "hive surface: window"; then
  box="$(x11_parse_surface "$(line_text 'hive surface: window')")"
  read -r bx by _ _ <<<"$box"
  sleep 0.3
  px="$("$TOOL" pixel "$bx" "$by")"
  want="$(grep -o '"surface-0": "#[0-9A-Fa-f]*"' "$ROOT/design/tokens.json" | grep -o '#[0-9A-Fa-f]*' | sort -u)"
  matched=""
  for w in $want; do x11_pixel_eq "$px" "$w" && matched="$w"; done
  if [ -n "$matched" ]; then
    pass "AC8 plate: corner pixel ($bx,$by) is $px, equal to surface-0 $matched, not the root colour FF00FF"
  else
    fail "AC8 plate: corner pixel ($bx,$by) is $px, surface-0 is one of: $(echo "$want" | tr '\n' ' ')"
  fi
  "$TOOL" move $((bx + 2)) $((by + 2)) >/dev/null
  "$TOOL" key button1 >/dev/null
  sleep 0.5
  [ "$(files_now)" = "$FILES0" ] && pass "AC8 plate: a click at the corner wrote 0 files" \
    || fail "AC8 plate: a click at the corner wrote files"
else
  fail "AC8 plate: no 'hive surface: window' line"
fi
close_hive
xsetroot -solid '#808080'

# ---- stop the service, count on_dismiss with the bench service ----
kill "$DPID" 2>/dev/null
wait "$DPID" 2>/dev/null
BENCH="$BIN/examples/hive_bench"
if [ -x "$BENCH" ]; then
  KAWARO_DEBUG_FOCUS=1 "$BENCH" daemon 2>"$TMP/bench.log" &
  BPID=$!
  track "$BPID"
  wait_until 10000 grep -q listening "$TMP/bench.log"
  "$BENCH" show >/dev/null 2>&1
  wait_until 3000 hive_shown
  sleep 0.5
  "$TOOL" key Escape >/dev/null
  wait_until 2000 hive_gone
  sleep 0.3
  d="$(grep -c '^dismissed' "$TMP/bench.log")"
  [ "$d" = 1 ] && pass "AC7 on_dismiss ran $d time for one Esc dismissal (bench service)" \
    || fail "AC7 on_dismiss ran $d times for one Esc dismissal (bench service)"
  "$BENCH" ctl quit >/dev/null 2>&1
else
  info "AC7 on_dismiss count: $BENCH not built, not measured"
fi

# ---- AC9: the 150 ms budget, through crates/ui/tests/measure.sh ----
for mode in open drop; do
  extra=()
  [ "$mode" = drop ] && extra=(DROP=1)
  out="$(env -u CARGO_TARGET_DIR X11=1 "${extra[@]}" "$ROOT/crates/ui/tests/measure.sh" 20 2 2>&1)"
  echo "$out" | sed 's/^/    measure /' | tail -12
  med="$(grep 'warm first_frame_ms median' <<<"$out" | awk '{print $4}')"
  if [ -z "$med" ]; then
    med="$(grep 'warm catcher_ms median' <<<"$out" | awk '{print $4}')"
  fi
  med_int="${med%.*}"
  if [[ "$med_int" =~ ^[0-9]+$ ]] && [ "$med_int" -lt 150 ]; then
    pass "AC9 $mode: median over 20 shows is $med ms (limit 150)"
  else
    fail "AC9 $mode: median over 20 shows is '${med:-none}' ms (limit 150)"
  fi
done

echo "x11 session checks done, $FAILS failed"
[ "$FAILS" -eq 0 ]
