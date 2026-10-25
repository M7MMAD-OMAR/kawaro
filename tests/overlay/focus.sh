#!/usr/bin/env bash
# S-018c AC10 on KWin, layer path: does the hive take the keyboard with the
# real xdg-activation token from tests/focus/client.py, at focus stealing
# prevention levels 1 (Low) and 4 (Extreme), and without one?
# Modelled on tests/focus/kwin.sh. Per case one private kwin_wayland --virtual:
# the daemon (KAWARO_DEBUG_FOCUS=1, WAYLAND_DEBUG=1) shows a warm-up hive,
# which is closed with Esc through org_kde_kwin_fake_input (the layer path
# does not close on focus loss); then client.py takes focus. In the token
# cases the client is maximized and one primary click is injected on it at
# the screen centre 3 s after start, and it launches `kawaro open` on the
# release with a token carrying that click's serial, as a file manager does
# after the user's click (KWin at level 4 accepts a token only if no
# interaction is newer than its serial, src/activation.cpp:604). The other
# cases launch 3 s after start (CLIENT_AUTO) with no pointer event, so the
# pointer is still for the level 4 notoken and bogus cases.
# Checks:
#   token at levels 1 and 4: activate(<client's token>, <catcher>) on the
#     show's catcher, and "hive focus: true".
#   notoken and bogus at level 4, pointer still (S-013 AC4, S-027 AC3): the
#     show is a layer surface, no wl_keyboard.enter on the catcher, no
#     "hive focus: true", focus=false in the TABLE line; bogus still sends
#     activate("kawaro-bogus-token", <catcher>).
#   notoken at levels 0 to 3 (S-027 AC4, hard checks): wl_keyboard.enter on
#     the catcher within 500 ms of the show's get_layer_surface, and
#     "hive focus: true".
#   every case (S-027 AC2): the first set_keyboard_interactivity on the
#     show's layer surface has value 1 and comes before the catcher's first
#     wl_surface.commit, and none follows that commit.
#   notoken at levels 0 to 3 and token at levels 1 and 4 (S-027 AC5): Esc
#     (evdev 1) injected through kwin-input while client.py runs, and the
#     layer surface's destroy follows within 1000 ms.
# Runs with a throwaway HOME, XDG dirs, runtime dir and session bus. Never
# touches the live desktop or profile.
# Usage: tests/overlay/focus.sh [LEVEL:CASE ...] [target/release]
#   default cases: 0:notoken 1:token 1:notoken 2:notoken 3:notoken 4:token 4:notoken 4:bogus
# Conditions go to check() in single quotes and are expanded by eval.
# shellcheck disable=SC2016,SC2034,SC2329
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
# shellcheck source=../no_portal.sh
source "$ROOT/tests/no_portal.sh"
CASES=()
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in [0-4]:token|[0-4]:notoken|[0-4]:bogus) CASES+=("$a") ;; *) BIN="$a" ;; esac
done
[ ${#CASES[@]} -eq 0 ] && CASES=(0:notoken 1:token 1:notoken 2:notoken 3:notoken 4:token 4:notoken 4:bogus)
INPUT="$ROOT/tests/drop/kwin-input/target/release/kwin-input"
[ -x "$BIN/kawaro" ] && [ -x "$BIN/kawarod" ] || { echo "build first: cargo build --release"; exit 2; }
[ -x "$INPUT" ] || { echo "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"; exit 2; }
OUT="${OVERLAY_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT INPUT

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE KAWARO_SURFACE
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
  KAWARO_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/kawarod" >"$DLOG" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/kawaro" ping >/dev/null 2>&1 && break; sleep 0.1; done
  # Warm-up show, so the measured show is not the slow first one (ADR 0001,
  # 8), closed with Esc; then a pause so the key does not count as recent
  # user activity for the client.
  "$BIN/kawaro" open "$W/b.png"; sleep 4
  echo "keydown 1" >&3; echo "sleep 40" >&3; echo "keyup 1" >&3
  sleep 3

  local mark; mark=$(wc -l <"$DLOG")
  local click=0
  [ "$CASE" = token ] && click=1
  if [ "$click" = 1 ]; then
    # WAYLAND_DEBUG=client to a separate file: the click's serials and the
    # token's set_serial, without flooding the client log.
    CLIENT_CLICK=1 CLIENT_AUTO="$CASE" KAWARO="$BIN/kawaro" WAYLAND_DEBUG=client       python3 "$ROOT/tests/focus/client.py" "$W/b.png" >"$log-client.log" 2>"$log-client-wayland.log" &
  else
    CLIENT_AUTO="$CASE" KAWARO="$BIN/kawaro" python3 "$ROOT/tests/focus/client.py" "$W/b.png" >"$log-client.log" 2>&1 &
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
  # S-027 AC5: in the cases where the hive must take the keyboard, Esc is
  # injected while client.py still runs, and the injection time is taken in
  # the trace's clock (WAYLAND_DEBUG stamps are UTC wall time).
  local esc_case=0 esc_at="" client_alive=no
  case "$LEVEL:$CASE" in [0-3]:notoken | 1:token | 4:token) esc_case=1 ;; esac
  if [ "$esc_case" = 1 ]; then
    kill -0 "$client" 2>/dev/null && client_alive=yes
    esc_at="[$(date -u +%H:%M:%S.%6N)]"
    echo "keydown 1" >&3; echo "sleep 40" >&3; echo "keyup 1" >&3
    sleep 1.5
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
  # S-027 AC2: order of the keyboard interactivity requests against the
  # catcher's commits, by line number in the trace (stamps can tie). The
  # window starts at the catcher's create_surface (or get_layer_surface),
  # because object ids are reused across shows.
  local lsurf="" n_gl=0 n_from=0 n_c1=0 n_k1=0 k1_val=none n_klate=0 n_destroy=0 n
  lsurf=$(grep -oE 'zwlr_layer_surface_v1[#@][0-9]+' <<<"$gl" | head -1)
  if [ -n "$gl" ] && [ -n "$cat" ] && [ -n "$lsurf" ]; then
    n_gl=$(grep -nF -- "$gl" <<<"$seg" | head -1 | cut -d: -f1)
    n_from=$n_gl
    n=$(head -n "$n_gl" <<<"$seg" | grep -nE -- "${R}wl_compositor[#@][0-9]+\.create_surface\(new id $cat\)" | tail -1 | cut -d: -f1)
    [ -n "$n" ] && n_from=$n
    n_destroy=$(tail -n +"$n_gl" <<<"$seg" | grep -nE -- "${R}$lsurf\.destroy\(\)" | head -1 | cut -d: -f1)
    if [ -n "$n_destroy" ]; then n_destroy=$((n_gl + n_destroy - 1)); else n_destroy=$(wc -l <<<"$seg"); fi
    local win; win=$(sed -n "${n_from},${n_destroy}p" <<<"$seg")
    n=$(grep -nE -- "${R}$cat\.commit\(\)" <<<"$win" | head -1 | cut -d: -f1); [ -n "$n" ] && n_c1=$n
    n=$(grep -nE -- "${R}$lsurf\.set_keyboard_interactivity\(" <<<"$win" | head -1); if [ -n "$n" ]; then
      n_k1=${n%%:*}; k1_val=$(sed -E 's/.*set_keyboard_interactivity\(([0-9]+)\).*/\1/' <<<"$n"); fi
    [ "$n_c1" -gt 0 ] && n_klate=$(tail -n +"$((n_c1 + 1))" <<<"$win" | grep -cE -- "${R}$lsurf\.set_keyboard_interactivity\(")
  fi
  # S-027 AC5: time from the injected Esc to the layer surface's destroy.
  local destroy_line="" esc_dt=none
  [ -n "$lsurf" ] && destroy_line=$(grep -E -- "${R}$lsurf\.destroy\(\)" <<<"$seg" | head -1)
  [ -n "$esc_at" ] && [ -n "$destroy_line" ] && esc_dt=$(( $(ts_ms "$destroy_line") - $(ts_ms "$esc_at") ))
  local esc_key; esc_key=$(grep -E -- "^\[[0-9:.]+\] wl_keyboard[#@][0-9]+\.key\([0-9]+, [0-9]+, 1, 1\)" <<<"$seg" | tail -1 | cut -d' ' -f1)
  local table_focus; table_focus=$([ "$focus_true" -ge 1 ] && echo true || echo false)
  echo "INFO level $LEVEL $CASE: layer show ${gl:+yes}${gl:-none}, catcher ${cat:-none}, token ${tok:-none}, activate: $(tr '\n' ' ' <<<"$acts"), keyboard enter after ${dt} ms, hive focus lines: $(grep '^hive focus' <<<"$seg" | tr '\n' ' ')"
  echo "INFO level $LEVEL $CASE: layer surface ${lsurf:-none}, first set_keyboard_interactivity(${k1_val}) at line ${n_k1}, catcher's first commit at line ${n_c1}, set_keyboard_interactivity after that commit: ${n_klate}"
  [ "$esc_case" = 1 ] && echo "INFO level $LEVEL $CASE: Esc injected at ${esc_at} (client.py running: $client_alive), Esc key event ${esc_key:-none}, destroy ${destroy_line:+at $(cut -d' ' -f1 <<<"$destroy_line")}, Esc to destroy ${esc_dt} ms"
  local table="TABLE level=$LEVEL case=$CASE focus=$table_focus enter_ms=$dt"
  echo "$table"
  check "S-027 AC2 level $LEVEL $CASE: the first set_keyboard_interactivity on the layer surface is 1 and comes before the catcher's first commit" \
    '[ "$n_k1" -gt 0 ] && [ "$n_c1" -gt 0 ] && [ "$k1_val" = 1 ] && [ "$n_k1" -lt "$n_c1" ]'
  check "S-027 AC2 level $LEVEL $CASE: no set_keyboard_interactivity on the layer surface after the catcher's first commit" \
    '[ "$n_c1" -gt 0 ] && [ "$n_klate" -eq 0 ]'
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
    check "S-027 AC3 level 4 $CASE, pointer still: no wl_keyboard.enter on the catcher" '[ -n "$cat" ] && [ -z "$enter" ]'
    check "S-027 AC3 level 4 $CASE, pointer still: focus=false in the TABLE line" '[ -n "$cat" ] && grep -q " focus=false " <<<"$table "'
    ;;
  [0-3]:notoken)
    check "S-027 AC4 level $LEVEL notoken: wl_keyboard.enter on the catcher within 500 ms of get_layer_surface" '[ "$dt" != none ] && [ "$dt" -le 500 ]'
    check "S-027 AC4 level $LEVEL notoken: hive focus: true" '[ "$focus_true" -ge 1 ]'
    ;;
  *) echo "INFO level $LEVEL $CASE: no level check for this case" ;;
  esac
  [ "$esc_case" = 1 ] && check "S-027 AC5 level $LEVEL $CASE: Esc injected while client.py runs, and the layer surface's destroy follows within 1000 ms" \
    '[ "$client_alive" = yes ] && [ "$esc_dt" != none ] && [ "$esc_dt" -ge 0 ] && [ "$esc_dt" -le 1000 ]'
  [ "$CASE" = bogus ] && check "AC10 level $LEVEL bogus: activate(\"kawaro-bogus-token\", <catcher>) still sent" \
    '[ -n "$cat" ] && grep -qF "activate(\"kawaro-bogus-token\", $cat)" <<<"$acts"'

  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi

  exec 3>&-
  "$BIN/kawaro" quit >/dev/null 2>&1
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
  no_real_portal "$SANDBOX"
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
echo "== every case (AC10 and S-027 table)"
grep '^TABLE' "$SUMMARY" | sed 's/^TABLE /  /'
if [ "$ALL" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL"; fi
exit "$ALL"
