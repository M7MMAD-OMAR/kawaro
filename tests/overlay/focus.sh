#!/usr/bin/env bash
# S-018c AC10 on KWin, layer path: does the hive take the keyboard with the
# real xdg-activation token from tests/focus/client.py, at focus stealing
# prevention levels 1 (Low) and 4 (Extreme), and without one?
# Modelled on tests/focus/kwin.sh. Per case one private kwin_wayland --virtual:
# the daemon (SBARHIVE_DEBUG_FOCUS=1, WAYLAND_DEBUG=1) shows a warm-up hive,
# which is closed with Esc through org_kde_kwin_fake_input (the layer path
# does not close on focus loss); then client.py takes focus. In the token
# cases the client is maximized and one primary click is injected on it at
# the screen centre 3 s after start, and it launches `sbarhive open` on the
# release with a token carrying that click's serial, as a file manager does
# after the user's click (KWin at level 4 accepts a token only if no
# interaction is newer than its serial, src/activation.cpp:604). The other
# cases launch 3 s after start (CLIENT_AUTO) with no pointer event, so the
# pointer is still for the level 4 notoken and bogus cases.
# Checks:
#   token at levels 1 and 4: activate(<client's token>, <catcher>) on the
#     show's catcher, and "hive focus: true".
#   notoken and bogus at level 4: no "hive focus: true" (S-013 AC4).
#   notoken at level 1: wl_keyboard.enter on the catcher within 500 ms of
#     the show's get_layer_surface, and "hive focus: true".
#   notoken at levels 2 and 3: measured and printed in a table, no check
#     (assumed in ADR 0004; a difference from level 1 is the maintainer's call).
# Runs with a throwaway HOME, XDG dirs, runtime dir and session bus. Never
# touches the live desktop or profile.
# Usage: tests/overlay/focus.sh [LEVEL:CASE ...] [target/release]
#   default cases: 1:token 1:notoken 4:token 4:notoken 4:bogus 2:notoken 3:notoken
# Conditions go to check() in single quotes and are expanded by eval.
# shellcheck disable=SC2016,SC2034,SC2329
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
CASES=()
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in [0-4]:token|[0-4]:notoken|[0-4]:bogus) CASES+=("$a") ;; *) BIN="$a" ;; esac
done
[ ${#CASES[@]} -eq 0 ] && CASES=(1:token 1:notoken 4:token 4:notoken 4:bogus 2:notoken 3:notoken)
INPUT="$ROOT/tests/drop/kwin-input/target/release/kwin-input"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
[ -x "$INPUT" ] || { echo "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"; exit 2; }
OUT="${OVERLAY_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT INPUT

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE SBARHIVE_SURFACE
  local log="$OUT/focus-l$LEVEL-$CASE"
  local FAILS=0
  check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi; }
  mkdir -p "$XDG_CONFIG_HOME"
  printf '[Windows]\nFocusStealingPreventionLevel=%s\n' "$LEVEL" >"$XDG_CONFIG_HOME/kwinrc"
  # KWIN_WAYLAND_NO_PERMISSION_CHECKS lets the test's own input client use
  # fake input in this private compositor only (the warm-up Esc).
  KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 \
    kwin_wayland --virtual --no-lockscreen --socket "$WAYLAND_DISPLAY" --width 1280 --height 800 >"$log-kwin.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "kwin did not start"; tail "$log-kwin.log"; kill $kwin; return 1; }
  sleep 2
  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  "$INPUT" <"$f" >"$log-input.log" 2>&1 &
  exec 3>"$f"

  local W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/b.png"
  local DLOG="$log-daemon.log"
  SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$DLOG" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
  # Warm-up show, so the measured show is not the slow first one (ADR 0001,
  # 8), closed with Esc; then a pause so the key does not count as recent
  # user activity for the client.
  "$BIN/sbarhive" open "$W/b.png"; sleep 4
  echo "keydown 1" >&3; echo "sleep 40" >&3; echo "keyup 1" >&3
  sleep 3

  local mark; mark=$(wc -l <"$DLOG")
  local click=0
  [ "$CASE" = token ] && click=1
  if [ "$click" = 1 ]; then
    # WAYLAND_DEBUG=client to a separate file: the click's serials and the
    # token's set_serial, without flooding the client log.
    CLIENT_CLICK=1 CLIENT_AUTO="$CASE" SBARHIVE="$BIN/sbarhive" WAYLAND_DEBUG=client       python3 "$ROOT/tests/focus/client.py" "$W/b.png" >"$log-client.log" 2>"$log-client-wayland.log" &
  else
    CLIENT_AUTO="$CASE" SBARHIVE="$BIN/sbarhive" python3 "$ROOT/tests/focus/client.py" "$W/b.png" >"$log-client.log" 2>&1 &
  fi
  local client=$!
  if [ "$click" = 1 ]; then
    sleep 3
    echo "move 640 400" >&3; echo "sleep 100" >&3
    echo "down" >&3; echo "sleep 40" >&3; echo "up" >&3
    sleep 3
  else
    sleep 6
  fi
  local seg; seg=$(tail -n +$((mark + 1)) "$DLOG")
  kill $client 2>/dev/null

  local R='-> '
  ts_ms() { sed -E 's/^\[([0-9]+):([0-9]+):([0-9]+)\.([0-9]{3}).*/\1 \2 \3 \4/' <<<"$1" | { read -r h m s ms; echo $(( (10#$h * 3600 + 10#$m * 60 + 10#$s) * 1000 + 10#$ms )); }; }
  local gl cat enter dt=none tok acts focus_true
  gl=$(grep -E -- "${R}zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+" <<<"$seg" | head -1)
  cat=$(grep -oE 'wl_surface[#@][0-9]+' <<<"$gl" | head -1)
  enter=""
  [ -n "$cat" ] && enter=$(grep -E -- "^\[[0-9:.]+\] wl_keyboard[#@][0-9]+\.enter\([0-9]+, $cat," <<<"$seg" | head -1)
  [ -n "$gl" ] && [ -n "$enter" ] && dt=$(( $(ts_ms "$enter") - $(ts_ms "$gl") ))
  tok=$(grep -oE 'client: token [^ ]+' "$log-client.log" | head -1 | cut -d' ' -f3)
  acts=$(grep -E -- "${R}xdg_activation_v1[#@][0-9]+\.activate\(" <<<"$seg" | sed -E 's/^\[[^]]*\] +//')
  focus_true=$(grep -c '^hive focus: true$' <<<"$seg")
  echo "INFO level $LEVEL $CASE: layer show ${gl:+yes}${gl:-none}, catcher ${cat:-none}, token ${tok:-none}, activate: $(tr '\n' ' ' <<<"$acts"), keyboard enter after ${dt} ms, hive focus lines: $(grep '^hive focus' <<<"$seg" | tr '\n' ' ')"
  echo "TABLE level=$LEVEL case=$CASE focus=$([ "$focus_true" -ge 1 ] && echo true || echo false) enter_ms=$dt"
  check "AC10 level $LEVEL $CASE: the measured show is a layer surface" '[ -n "$cat" ]'
  case "$LEVEL:$CASE" in
  1:token | 4:token)
    check "AC10 level $LEVEL token: client.py got a real token" '[ -n "$tok" ] && [ "$tok" != none ]'
    check "AC10 level $LEVEL token: activate(\"<that token>\", <catcher>) sent" \
      '[ -n "$tok" ] && [ -n "$cat" ] && grep -qF "activate(\"$tok\", $cat)" <<<"$acts"'
    check "AC10 level $LEVEL token: the hive takes the keyboard (hive focus: true)" '[ "$focus_true" -ge 1 ]'
    ;;
  4:notoken | 4:bogus)
    check "AC10 level 4 $CASE, pointer still: the hive does not take the keyboard (no hive focus: true)" '[ -n "$cat" ] && [ "$focus_true" -eq 0 ]'
    ;;
  1:notoken)
    check "AC10 level 1 notoken: wl_keyboard.enter on the catcher within 500 ms of get_layer_surface" '[ "$dt" != none ] && [ "$dt" -le 500 ]'
    check "AC10 level 1 notoken: hive focus: true" '[ "$focus_true" -ge 1 ]'
    ;;
  *) echo "INFO level $LEVEL $CASE: measured only (AC10, levels 2 and 3)" ;;
  esac
  [ "$CASE" = bogus ] && check "AC10 level $LEVEL bogus: activate(\"sbarhive-bogus-token\", <catcher>) still sent" \
    '[ -n "$cat" ] && grep -qF "activate(\"sbarhive-bogus-token\", $cat)" <<<"$acts"'

  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi

  exec 3>&-
  "$BIN/sbarhive" quit >/dev/null 2>&1
  sleep 0.5
  kill $d 2>/dev/null
  kill $kwin 2>/dev/null
  timeout 5 tail --pid=$kwin -f /dev/null || kill -9 $kwin
}
export -f inner

ALL=0
SUMMARY="$OUT/focus.txt"; : >"$SUMMARY"
for C in "${CASES[@]}"; do
  LEVEL=${C%%:*} CASE=${C#*:}
  export LEVEL CASE
  echo "== kwin focus level $LEVEL $CASE" | tee -a "$SUMMARY"
  SANDBOX="$(mktemp -d)"
  RUNTIME="$(mktemp -d)"
  chmod 700 "$RUNTIME"
  # Environment set BEFORE dbus-run-session, so nothing the private bus
  # activates can reach the live desktop or the real profile.
  env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
    HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
    XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
    XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY=wayland-sbh-overlay-focus \
    dbus-run-session -- bash -c inner 2>"$OUT/focus-l$LEVEL-$CASE-bus.log" | tee -a "$SUMMARY" | tee "$OUT/focus-l$LEVEL-$CASE.txt" >/dev/null
  tail -n +1 "$OUT/focus-l$LEVEL-$CASE.txt" | grep -E '^(INFO|PASS|FAIL|RESULT)'
  grep -q '^RESULT PASS' "$OUT/focus-l$LEVEL-$CASE.txt" || ALL=1
  rm -rf "$SANDBOX" "$RUNTIME"
done
echo "== levels 2 and 3 without a token, and the rest (AC10 table)"
grep '^TABLE' "$SUMMARY" | sed 's/^TABLE /  /'
if [ "$ALL" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL"; fi
exit "$ALL"
