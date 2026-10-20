#!/usr/bin/env bash
# S-014b drop receiver on KWin (AC5, AC6, AC7). A GTK 4 window drags a PNG,
# `sbarhive drop` is run mid-drag (what the shortcut does), and the file is
# dropped: show 1 on the WEBP cell, show 2 on a transparent corner, show 3 on
# the WEBP cell with Shift held, show 4 on the centre cell. The daemon runs
# with WAYLAND_DEBUG=1 so the trace shows a wl_data_device per new surface.
# Runs in a private kwin_wayland --virtual with a throwaway HOME, XDG dirs,
# runtime dir and session bus. Never touches the live desktop or profile.
# Usage: tests/drop/kwin.sh [level=N] [target/release]
#   level: focus stealing prevention 0..4, default 1 (Low)
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
LEVEL=1
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in level=*) LEVEL="${a#level=}" ;; *) BIN="$a" ;; esac
done
INPUT="$HERE/kwin-input/target/release/kwin-input"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
[ -x "$INPUT" ] || { echo "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"; exit 2; }
OUT="${DROP_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT LEVEL INPUT

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  # shellcheck source=lib.sh
  source "$HERE/lib.sh"
  OUT_LOG="$OUT/kwin-l$LEVEL"
  local log="$OUT_LOG"
  : >"$log-cli.log"
  mkdir -p "$XDG_CONFIG_HOME"
  printf '[Windows]\nFocusStealingPreventionLevel=%s\n' "$LEVEL" >"$XDG_CONFIG_HOME/kwinrc"
  # KWIN_WAYLAND_NO_PERMISSION_CHECKS lets the test's own input client use
  # org_kde_kwin_fake_input in this private compositor only.
  KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 QT_FORCE_STDERR_LOGGING=1 \
    QT_LOGGING_RULES='kwin_scripting.debug=true;js.debug=true' \
    kwin_wayland --virtual --no-lockscreen --socket "$WAYLAND_DISPLAY" --width 1280 --height 800 >"$log-kwin.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "kwin did not start"; tail "$log-kwin.log"; return 1; }
  sleep 2

  hive_geo() { # KWin script printing the hive's frame geometry to KWin's log
    # Called in $(...) subshells, so the tag cannot come from a counter.
    local tag="sbh-geo-${EPOCHREALTIME/./}-$RANDOM" id
    local js="$XDG_RUNTIME_DIR/$tag.js"
    cat >"$js" <<JS
workspace.windowList().filter(w => w.caption === "SbarHive" && !w.minimized && !w.deleted)
  .forEach(w => { const g = w.frameGeometry; print("$tag " + Math.round(g.x) + " " + Math.round(g.y) + " " + Math.round(g.width) + " " + Math.round(g.height)); });
print("$tag end");
JS
    id=$(gdbus call --session --dest org.kde.KWin --object-path /Scripting \
      --method org.kde.kwin.Scripting.loadScript "$js" "$tag" | grep -o '[0-9]\+' | head -1)
    gdbus call --session --dest org.kde.KWin --object-path "/Scripting/Script$id" \
      --method org.kde.kwin.Script.run >/dev/null 2>&1
    for _ in $(seq 20); do grep -q "$tag end" "$log-kwin.log" && break; sleep 0.05; done
    gdbus call --session --dest org.kde.KWin --object-path /Scripting \
      --method org.kde.kwin.Scripting.unloadScript "$tag" >/dev/null 2>&1
    grep -o "$tag [0-9-]\+ [0-9-]\+ [0-9]\+ [0-9]\+" "$log-kwin.log" | tail -1 | cut -d' ' -f2-
  }

  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  "$INPUT" <"$f" >"$log-input.log" 2>&1 &
  exec 3>"$f"
  ptr_move() { echo "move $1 $2" >&3; }
  ptr_down() { echo "down" >&3; }
  ptr_up() { echo "up" >&3; }
  shift_down() { echo "keydown 42" >&3; }
  shift_up() { echo "keyup 42" >&3; }

  W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  local i
  for i in one two three four; do cp "$ROOT/tests/fixtures/images/alpha.png" "$W/photo $i.png"; done
  NEXT="$XDG_RUNTIME_DIR/next"; printf '%s\n' "$W/photo one.png" >"$NEXT"
  DLOG="$log-daemon.log"; SLOG="$log-source.log"

  # English layout, so the WEBP cell is top right of the centre. No warm-up
  # show: the first drop-mode show is part of AC5.
  SBARHIVE_LANG=en WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$DLOG" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done

  GTK_A11Y=none python3 "$HERE/source.py" "$NEXT" >"$SLOG" 2>&1 &
  local src=$!
  sleep 3

  cycle 1 "$W/photo one.png" webp 0
  cycle 2 "$W/photo two.png" corner 0
  cycle 3 "$W/photo three.png" webp 1
  cycle 4 "$W/photo four.png" centre 0
  protocol_checks
  summary

  exec 3>&-
  "$BIN/sbarhive" quit >/dev/null 2>&1
  sleep 0.5
  kill $src $d 2>/dev/null
  kill $kwin 2>/dev/null
  timeout 5 tail --pid=$kwin -f /dev/null || kill -9 $kwin
}
export -f inner

echo "== kwin level $LEVEL"
SANDBOX="$(mktemp -d)"
# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates can reach the live desktop or the real profile.
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh-drop-kwin \
  dbus-run-session -- bash -c inner 2>"$OUT/kwin-l$LEVEL-bus.log" | tee "$OUT/kwin-l$LEVEL.txt"
grep -q '^RESULT PASS' "$OUT/kwin-l$LEVEL.txt"
