#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-115: the drop cell on X11, checked live in a private session. Starts a
# virtual X server and xfwm4 (no compositor) on a session bus of its own, runs
# `kawaro drop`, then drags a PNG from a GTK 4 source (tests/drop/source.py,
# GDK_BACKEND=x11, offers copy and move) onto a hive cell with Shift held, the
# key a file manager reads as "move". The original must survive: the source
# reports deleted=False, the original's sha256 is unchanged and <name>.webp
# appears within 5 s. 3 runs, all must pass (AC5). It also checks that the
# hive stays above the raised source window (AC2) and that the hovered cell
# turns accent within 100 ms (AC4). Never touches the live desktop.
#
# A manual release step, like tests/x11/run.sh; cargo test and CI never call it.
# Usage: tests/x11/drop.sh
#   KAWARO_BIN_DIR  where kawarod and kawaro are (default target/release).
# Run it inside the scope from CLAUDE.md ("Running heavy work").
set -u

for tool in Xvfb xfwm4 sha256sum file; do
  command -v "$tool" >/dev/null 2>&1 || { echo "SKIP: $tool not found"; exit 0; }
done
PY=/usr/bin/python3
"$PY" -c 'import gi; gi.require_version("Gtk", "4.0")' >/dev/null 2>&1 || { echo "SKIP: GTK 4 for $PY not found"; exit 0; }

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
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
  trap 'rm -rf "$TMP" 2>/dev/null || { sleep 2; rm -rf "$TMP"; }' EXIT
  TOOL_TARGET="$REAL_HOME/.cache/kawaro-x11-tool-target"
  CARGO_TARGET_DIR="$TOOL_TARGET" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}" \
    cargo build --quiet --release --manifest-path "$HERE/x11-tool/Cargo.toml" || { echo "ERROR: x11-tool did not build" >&2; exit 2; }
  mkdir -p "$TMP/home" "$TMP/run" "$TMP/tmp"
  chmod 700 "$TMP/run"
  export CARGO_HOME="${CARGO_HOME:-$REAL_HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$REAL_HOME/.rustup}"
  export KAWARO_X11_TMP="$TMP" KAWARO_X11_BIN="$BIN" KAWARO_X11_TOOL="$TOOL_TARGET/release/x11-tool"
  export HOME="$TMP/home" TMPDIR="$TMP/tmp" XDG_RUNTIME_DIR="$TMP/run"
  export XDG_CONFIG_HOME="$TMP/home/.config" XDG_DATA_HOME="$TMP/home/.local/share"
  export XDG_CACHE_HOME="$TMP/home/.cache" XDG_STATE_HOME="$TMP/home/.local/state"
  export GDK_BACKEND=x11 XDG_SESSION_TYPE=x11
  unset WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE DISPLAY DBUS_SESSION_BUS_ADDRESS \
    XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID KAWARO_SURFACE
  dbus-run-session -- "$BASH" "$HERE/drop.sh" --inner
  rc=$?
  alive=0
  if [ -f "$TMP/pids" ]; then
    while read -r p; do kill -0 "$p" 2>/dev/null && alive=$((alive + 1)); done <"$TMP/pids"
  fi
  [ "$alive" -eq 0 ] && echo "PASS isolation: 0 started processes alive after exit" \
    || { echo "FAIL isolation: $alive started processes still alive"; rc=1; }
  exit "$rc"
fi

# ------------------------------------------------------------------ inner ---
TMP="$KAWARO_X11_TMP"
BIN="$KAWARO_X11_BIN"
TOOL="$KAWARO_X11_TOOL"
PIDS=()
FAILS=0
DLOG="$TMP/daemon.log"
SLOG="$TMP/source.log"
: >"$DLOG"
: >"$SLOG"
pass() { echo "PASS $*"; }
fail() { FAILS=$((FAILS + 1)); echo "FAIL $*"; }
info() { echo "INFO $*"; }
track() { PIDS+=("$1"); echo "$1" >>"$TMP/pids"; }
stop_all() {
  local i p
  "$TOOL" up button1 >/dev/null 2>&1
  for ((i = ${#PIDS[@]} - 1; i >= 0; i--)); do kill "${PIDS[$i]}" 2>/dev/null; done
  for p in "${PIDS[@]}"; do
    for _ in $(seq 50); do kill -0 "$p" 2>/dev/null || break; sleep 0.1; done
    kill -0 "$p" 2>/dev/null && kill -9 "$p" 2>/dev/null
    wait "$p" 2>/dev/null
  done
}
trap stop_all EXIT
wait_until() { # wait_until TIMEOUT_MS cmd...
  local end=$(($(now_ms) + $1))
  shift
  while ! "$@"; do
    [ "$(now_ms)" -ge "$end" ] && return 1
    sleep 0.02
  done
}
hive_shown() { "$TOOL" geometry Kawaro >/dev/null 2>&1; }
hive_gone() { ! hive_shown; }

Xvfb -displayfd 3 -screen 0 1280x800x24 -nolisten tcp 3>"$TMP/display" 2>"$TMP/xvfb.log" &
track $!
wait_until 10000 test -s "$TMP/display" || { echo "ERROR: Xvfb did not start"; exit 2; }
export DISPLAY=":$(head -1 "$TMP/display")"
xfwm4 --compositor=off >"$TMP/xfwm4.log" 2>&1 &
track $!
wait_until 10000 bash -c 'xprop -root _NET_SUPPORTING_WM_CHECK 2>/dev/null | grep -q "window id"' \
  || { echo "ERROR: xfwm4 did not become the window manager"; exit 2; }
info "private X server on $DISPLAY, xfwm4 --compositor=off"

WORK="$HOME/work"
mkdir -p "$WORK"
NEXT="$TMP/next"
: >"$NEXT"
"$PY" "$ROOT/tests/drop/source.py" "$NEXT" >>"$SLOG" 2>&1 &
track $!
wait_until 15000 grep -q "source: shown" "$SLOG" || { echo "ERROR: the GTK 4 source did not start"; cat "$SLOG"; exit 2; }
sleep 1

KAWARO_DEBUG_FOCUS=1 "$BIN/kawarod" >>"$DLOG" 2>&1 &
DPID=$!
track "$DPID"
wait_until 10000 compgen -G "$XDG_RUNTIME_DIR/*kawaro*" >/dev/null || sleep 2
sleep 1

# Ring 1 cell i, clockwise from the top right, is at 78 px (hex.size.md 72 plus
# hex.gap 6) from the hive centre, at i * 60 + 30 degrees from straight up.
cell_xy() { # cell_xy BOX_X BOX_Y BOX_W BOX_H INDEX
  awk -v x="$1" -v y="$2" -v w="$3" -v h="$4" -v i="$5" 'BEGIN {
    pi = atan2(0, -1); a = (i * 60 + 30) * pi / 180
    printf "%d %d\n", x + w / 2 + 78 * sin(a), y + h / 2 - 78 * cos(a) }'
}
accent="$(grep -o '"accent": "#[0-9A-Fa-f]*"' "$ROOT/design/tokens.json" | grep -o '#[0-9A-Fa-f]*' | sort -u)"

for run in $(seq 1 3); do
  D="$WORK/run$run"
  mkdir -p "$D"
  cp "$ROOT/tests/fixtures/images/kawaro-sample.png" "$D/sample.png"
  before="$(sha256sum "$D/sample.png" | cut -d' ' -f1)"
  mime="$(file --mime-type -b "$D/sample.png")"
  echo "$D/sample.png" >"$NEXT"
  DMARK="$(wc -l <"$DLOG")"
  SMARK="$(wc -l <"$SLOG")"
  "$TOOL" move 100 100 >/dev/null
  "$BIN/kawaro" drop >/dev/null 2>&1
  if ! wait_until 3000 hive_shown; then
    fail "run $run: the hive did not show for kawaro drop"
    continue
  fi
  sleep 0.5
  read -r bx by bw bh <<<"$("$TOOL" geometry Kawaro)"
  [ -n "${KAWARO_X11_DEBUG:-}" ] && { xprop -root _NET_CLIENT_LIST_STACKING; for w in $(xprop -root _NET_CLIENT_LIST | cut -d'#' -f2 | tr -d ','); do echo "== $w"; xprop -id "$w" _NET_WM_NAME _NET_WM_STATE _NET_WM_WINDOW_TYPE; done; }
  # AC2: a click on the source raises it; the hive must stay above it.
  "$TOOL" down button1 >/dev/null
  sleep 0.3
  order="$("$TOOL" stacking | grep -nE '^(Kawaro|drag-source)$' | tr '\n' ' ')"
  hive_line="$("$TOOL" stacking | grep -n '^Kawaro$' | head -1 | cut -d: -f1)"
  src_line="$("$TOOL" stacking | grep -n '^drag-source$' | head -1 | cut -d: -f1)"
  if [ -n "$hive_line" ] && [ -n "$src_line" ] && [ "$hive_line" -lt "$src_line" ]; then
    pass "run $run AC2: the hive is above the raised source window ($order)"
  else
    fail "run $run AC2: the hive is not above the source ($order)"
  fi
  # Shift is held for the whole drag: a file manager reads it as move.
  "$TOOL" down Shift_L >/dev/null
  "$TOOL" move 130 130 >/dev/null
  sleep 0.2
  "$TOOL" move 160 160 >/dev/null
  sleep 0.2
  # The drag enters the hive at its centre; winit reports the hovered file.
  "$TOOL" move $((bx + bw / 2)) $((by + bh / 2)) >/dev/null
  sleep 0.5
  cells_line="$(tail -n +$((DMARK + 1)) "$DLOG" | grep -a -F -m1 'drop cells:' || true)"
  idx=-1
  i=0
  for id in ${cells_line#drop cells:}; do
    [ "$idx" -lt 0 ] && [[ "$id" == *webp* ]] && idx=$i
    i=$((i + 1))
  done
  if [ "$idx" -lt 0 ] || [ "$idx" -gt 5 ]; then
    fail "run $run: no ring 1 webp cell in '${cells_line:-no drop cells line}'"
    "$TOOL" up Shift_L >/dev/null; "$TOOL" up button1 >/dev/null
    "$TOOL" key Escape >/dev/null
    continue
  fi
  read -r cx cy <<<"$(cell_xy "$bx" "$by" "$bw" "$bh" "$idx")"
  "$TOOL" move "$cx" "$cy" >/dev/null
  t0="$(now_ms)"
  seen=""
  while [ $(($(now_ms) - t0)) -le 100 ]; do
    px="$("$TOOL" pixel $((cx + 26)) $((cy - 14)))"
    for a in $accent; do
      p="${px,,}"; q="${a,,}"; q="${q#\#}"
      [ "$p" = "$q" ] && seen="$px"
    done
    [ -n "$seen" ] && break
  done
  if [ -n "$seen" ]; then
    pass "run $run AC4: pixel inside the hovered cell is accent $seen $(($(now_ms) - t0)) ms after the move"
  else
    fail "run $run AC4: pixel inside the hovered cell is $px, accent is one of $(echo "$accent" | tr '\n' ' ')"
  fi
  "$TOOL" up button1 >/dev/null
  "$TOOL" up Shift_L >/dev/null
  if wait_until 5000 test -s "$D/sample.webp"; then
    pass "run $run AC5: sample.webp exists $(($(now_ms) - t0)) ms after the drop"
  else
    fail "run $run AC5: no sample.webp within 5 s"
  fi
  after="$(sha256sum "$D/sample.png" 2>/dev/null | cut -d' ' -f1)"
  [ "$before" = "$after" ] && pass "run $run AC5: original sha256 unchanged ($before)" \
    || fail "run $run AC5: original sha256 $before became '${after:-missing}'"
  out_mime="$(file --mime-type -b "$D/sample.webp" 2>/dev/null)"
  [ "$out_mime" = "image/webp" ] && pass "run $run AC5: output is $out_mime, input was $mime" \
    || fail "run $run AC5: output is '${out_mime:-missing}'"
  source_end() { tail -n +$((SMARK + 1)) "$SLOG" | grep -a 'drag-end' | tail -1; }
  wait_until 3000 test -n "$(source_end)"
  end="$(source_end)"
  case "$end" in
    *deleted=False*) pass "run $run AC5: source reported '${end#source: }'" ;;
    *) fail "run $run AC5: source reported '${end:-no drag-end line}'" ;;
  esac
  wait_until 3000 hive_gone || "$TOOL" key Escape >/dev/null
  sleep 0.5
done

if [ "$FAILS" -ne 0 ]; then
  echo "---- daemon log (tail)"; tail -n 25 "$DLOG"
  echo "---- source log (tail)"; tail -n 15 "$SLOG"
fi
echo "x11 drop checks done, $FAILS failed"
[ "$FAILS" -eq 0 ]
