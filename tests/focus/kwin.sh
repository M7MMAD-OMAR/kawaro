#!/usr/bin/env bash
# Does the hive get keyboard focus on KWin Wayland when the daemon opens it,
# with and without an xdg-activation token, at a given focus stealing
# prevention level (0 none, 1 low default, 2 medium, 3 high, 4 extreme)?
# Runs in a private kwin_wayland --virtual with a throwaway HOME, XDG dirs,
# runtime dir and session bus. Never touches the live desktop or profile.
# Usage: tests/focus/kwin.sh [notoken|token|bogus ...] [level=N] [target/release]
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
# shellcheck source=../no_portal.sh
source "$ROOT/tests/no_portal.sh"
CASES=()
LEVEL=1
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in notoken|token|bogus) CASES+=("$a") ;; level=*) LEVEL="${a#level=}" ;; *) BIN="$a" ;; esac
done
[ ${#CASES[@]} -eq 0 ] && CASES=(notoken token)
[ -x "$BIN/kawaro" ] && [ -x "$BIN/kawarod" ] || { echo "build first: cargo build --release"; exit 2; }
[ -x "$ROOT/tests/drop/kwin-input/target/release/kwin-input" ] || { echo "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"; exit 2; }
OUT="${FOCUS_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT LEVEL

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  local log="$OUT/kwin-l$LEVEL-$CASE"
  mkdir -p "$XDG_CONFIG_HOME"
  printf '[Windows]\nFocusStealingPreventionLevel=%s\n' "$LEVEL" >"$XDG_CONFIG_HOME/kwinrc"
  KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 QT_FORCE_STDERR_LOGGING=1 QT_LOGGING_RULES='kwin_scripting.debug=true;js.debug=true' kwin_wayland --virtual --no-lockscreen --socket "$WAYLAND_DISPLAY" --width 1280 --height 800 \
    >"$log-kwin.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "kwin did not start"; tail "$log-kwin.log"; return 1; }
  sleep 2

  # Observation through KWin scripting: the script prints to KWin's log.
  local n=0
  state() {
    n=$((n + 1))
    local js="$XDG_RUNTIME_DIR/state$n.js" tag="sbh-state-$n"
    cat >"$js" <<EOF
const a = workspace.activeWindow;
const ws = workspace.windowList().filter(w => w.normalWindow || w.dialog);
print("$tag " + JSON.stringify({
  focus: a ? a.caption : null,
  attention: ws.filter(w => w.demandsAttention).map(w => w.caption),
  windows: ws.map(w => w.caption),
}));
EOF
    local id
    id=$(gdbus call --session --dest org.kde.KWin --object-path /Scripting \
      --method org.kde.kwin.Scripting.loadScript "$js" "$tag" | grep -o '[0-9]\+' | head -1)
    gdbus call --session --dest org.kde.KWin --object-path "/Scripting/Script$id" \
      --method org.kde.kwin.Script.run >/dev/null 2>&1
    sleep 0.5
    gdbus call --session --dest org.kde.KWin --object-path /Scripting \
      --method org.kde.kwin.Scripting.unloadScript "$tag" >/dev/null 2>&1
    grep -o "$tag .*" "$log-kwin.log" | tail -1 | cut -d' ' -f2-
  }

  local W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  ffmpeg -loglevel error -f lavfi -i testsrc=size=80x60:rate=1 -frames:v 1 "$W/b.jpg"

  KAWARO_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/kawarod" >"$log-daemon.log" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/kawaro" ping >/dev/null 2>&1 && break; sleep 0.1; done
  # Injected input through org_kde_kwin_fake_input (this private KWin only,
  # KWIN_WAYLAND_NO_PERMISSION_CHECKS above): the warm-up Esc and, in the
  # token case, the click on the client.
  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  "$ROOT/tests/drop/kwin-input/target/release/kwin-input" <"$f" >"$log-input.log" 2>&1 &
  exec 3>"$f"
  # Warm-up show, so the measured show is not the slow first one (ADR 0001, 8).
  # NO_WARMUP=1 skips it, so the measured show is the first one.
  if [ -z "${NO_WARMUP:-}" ]; then
    "$BIN/kawaro" open "$W/b.jpg"; sleep 10
    echo "warm-up: $(state)"
    # S-018c: on the layer path focus loss no longer closes the hive, so the
    # warm-up hive is closed with Esc, then a pause so the key is not recent
    # user activity for the client.
    echo "keydown 1" >&3; echo "sleep 40" >&3; echo "keyup 1" >&3
    sleep 3
  fi

  # The client takes focus. notoken and bogus: it launches `kawaro open`
  # 3 s later. token: it is maximized, one primary click is injected on it at
  # the screen centre about 3 s after start, and it launches on the release
  # with a token carrying that click's serial, as a file manager does after
  # the user's click. KWin at level 4 accepts a token only if no interaction
  # is newer than its serial (src/activation.cpp:604), and the warm-up Esc
  # is newer than a token taken with no click (serial 0).
  local mark; mark=$(wc -l <"$log-daemon.log")
  if [ "$CASE" = token ]; then
    CLIENT_CLICK=1 CLIENT_AUTO="$CASE" KAWARO="$BIN/kawaro" WAYLAND_DEBUG=client \
      python3 "$HERE/client.py" "$W/b.jpg" >"$log-client.log" 2>"$log-client-wayland.log" &
  else
    CLIENT_AUTO="$CASE" KAWARO="$BIN/kawaro" python3 "$HERE/client.py" "$W/b.jpg" >"$log-client.log" 2>&1 &
  fi
  local client=$!
  sleep 2
  echo "before: $(state)"
  if [ "$CASE" = token ]; then
    echo "move 640 400" >&3; echo "sleep 100" >&3
    echo "down" >&3; echo "sleep 40" >&3; echo "up" >&3
  fi
  sleep 4
  echo "after:  $(state)"
  echo "daemon focus lines: $(tail -n +$((mark + 1)) "$log-daemon.log" | grep 'hive focus' | tr '\n' ' ')"
  echo "client: $(grep -E 'token|focus' "$log-client.log" | tr '\n' ' ')"
  grep -i "activation\|token" "$log-kwin.log" | grep -v sbh-state | tail -5 | sed 's/^/kwin: /'
  kill $client 2>/dev/null
  exec 3>&-

  "$BIN/kawaro" quit >/dev/null 2>&1
  sleep 0.5
  kill $d 2>/dev/null
  kill $kwin 2>/dev/null
  timeout 5 tail --pid=$kwin -f /dev/null || kill -9 $kwin
}
export -f inner

for CASE in "${CASES[@]}"; do
  echo "== kwin level $LEVEL $CASE"
  export CASE
  SANDBOX="$(mktemp -d)"
  no_real_portal "$SANDBOX"
  # Environment set BEFORE dbus-run-session, so nothing the private bus
  # activates can reach the live desktop or the real profile.
  env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
    HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
    XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
    XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh-focus-kwin \
    dbus-run-session -- bash -c inner 2>"$OUT/kwin-l$LEVEL-$CASE-bus.log" | tee "$OUT/kwin-l$LEVEL-$CASE.txt"
done
