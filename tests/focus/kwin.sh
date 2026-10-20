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
CASES=()
LEVEL=1
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in notoken|token|bogus) CASES+=("$a") ;; level=*) LEVEL="${a#level=}" ;; *) BIN="$a" ;; esac
done
[ ${#CASES[@]} -eq 0 ] && CASES=(notoken token)
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
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

  SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$log-daemon.log" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
  # Warm-up show, so the measured show is not the slow first one (ADR 0001, 8).
  # NO_WARMUP=1 skips it, so the measured show is the first one.
  if [ -z "${NO_WARMUP:-}" ]; then
    "$BIN/sbarhive" open "$W/b.jpg"; sleep 10
    echo "warm-up: $(state)"
    # S-018c: on the layer path focus loss no longer closes the hive, so the
    # warm-up hive is closed with Esc through org_kde_kwin_fake_input (this
    # private KWin only, KWIN_WAYLAND_NO_PERMISSION_CHECKS above), then a
    # pause so the key is not recent user activity for the client.
    local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
    "$ROOT/tests/drop/kwin-input/target/release/kwin-input" <"$f" >"$log-input.log" 2>&1 &
    exec 3>"$f"
    echo "keydown 1" >&3; echo "sleep 40" >&3; echo "keyup 1" >&3
    exec 3>&-
    sleep 3
  fi

  # The client takes focus, then launches `sbarhive open` 3 s later.
  local mark; mark=$(wc -l <"$log-daemon.log")
  CLIENT_AUTO="$CASE" SBARHIVE="$BIN/sbarhive" python3 "$HERE/client.py" "$W/b.jpg" >"$log-client.log" 2>&1 &
  local client=$!
  sleep 2
  echo "before: $(state)"
  sleep 4
  echo "after:  $(state)"
  echo "daemon focus lines: $(tail -n +$((mark + 1)) "$log-daemon.log" | grep 'hive focus' | tr '\n' ' ')"
  echo "client: $(grep -E 'token|focus' "$log-client.log" | tr '\n' ' ')"
  grep -i "activation\|token" "$log-kwin.log" | grep -v sbh-state | tail -5 | sed 's/^/kwin: /'
  kill $client 2>/dev/null

  "$BIN/sbarhive" quit >/dev/null 2>&1
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
  # Environment set BEFORE dbus-run-session, so nothing the private bus
  # activates can reach the live desktop or the real profile.
  env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
    HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
    XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
    XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh-focus-kwin \
    dbus-run-session -- bash -c inner 2>"$OUT/kwin-l$LEVEL-$CASE-bus.log" | tee "$OUT/kwin-l$LEVEL-$CASE.txt"
done
