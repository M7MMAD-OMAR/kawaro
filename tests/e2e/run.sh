#!/usr/bin/env bash
# End-to-end test of the built binaries inside a private headless KWin with a
# throwaway HOME and session bus. Never touches the live desktop or profile.
# Usage: tests/e2e/run.sh [target/release]
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${1:-$ROOT/target/release}"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
SANDBOX="$(mktemp -d)"
export BIN SANDBOX ROOT

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP
  kwin_wayland --virtual --socket "$WAYLAND_DISPLAY" --width 1920 --height 1080 >"$SANDBOX/kwin.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  local fail=0
  check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; fail=1; fi; }

  local W="$SANDBOX/work"; mkdir -p "$W/sub dir"
  # Fixtures made with ffmpeg so the test needs nothing committed.
  ffmpeg -loglevel error -f lavfi -i testsrc=size=96x64:rate=1 -frames:v 1 "$W/sub dir/صورة 1.png"
  ffmpeg -loglevel error -f lavfi -i testsrc=size=80x60:rate=1 -frames:v 1 "$W/b.jpg"

  # Auto-start: open with no daemon running must start one within 2 s.
  check "open starts the daemon" '"$BIN/sbarhive" open "$W/b.jpg" && "$BIN/sbarhive" ping | grep -q pong'
  "$BIN/sbarhive" quit >/dev/null 2>&1
  for _ in $(seq 30); do [ -S "$XDG_RUNTIME_DIR/sbarhive.sock" ] || break; sleep 0.1; done

  "$BIN/sbarhived" >"$SANDBOX/daemon.log" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
  check "daemon answers ping" '"$BIN/sbarhive" ping | grep -q pong'
  local inode; inode=$(stat -c %i "$XDG_RUNTIME_DIR/sbarhive.sock")
  check "single instance" '"$BIN/sbarhived" 2>&1 | grep -q "already running"'
  check "second instance leaves the socket alone" '[ "$(stat -c %i "$XDG_RUNTIME_DIR/sbarhive.sock")" = "$inode" ] && "$BIN/sbarhive" ping | grep -q pong'
  notes() { grep -c "sbarhived: notify:" "$SANDBOX/daemon.log"; }

  check "actions listed for images" '"$BIN/sbarhive" actions "$W/b.jpg" | grep -q "image.to-webp\|webp"'
  out=$("$BIN/sbarhive" run image.to-webp --wait "$W/sub dir/صورة 1.png" 2>&1)
  check "run --wait prints output path" '[ -f "$W/sub dir/صورة 1.webp" ] && grep -q "صورة 1.webp" <<<"$out"'
  check "output is WebP" '[ "$(head -c 4 "$W/sub dir/صورة 1.webp")" = RIFF ]'
  check "original untouched" '[ -f "$W/sub dir/صورة 1.png" ]'
  "$BIN/sbarhive" run image.to-webp --wait "$W/sub dir/صورة 1.png" >/dev/null 2>&1
  check "second run does not overwrite" '[ -f "$W/sub dir/صورة 1 (1).webp" ]'
  check "no temp files left" '[ -z "$(find "$W" -name ".*" -type f)" ]'

  pdf_id=$("$BIN/sbarhive" actions "$W/b.jpg" "$W/sub dir/صورة 1.png" | awk -F'\t' '$1 ~ /pdf/ {print $1; exit}')
  [ -n "$pdf_id" ] && "$BIN/sbarhive" run "$pdf_id" --wait "$W/b.jpg" "$W/sub dir/صورة 1.png" >/dev/null 2>&1
  check "combined PDF from two images" '[ -n "$pdf_id" ] && ls "$W"/b*.pdf >/dev/null 2>&1 && [ "$(head -c 4 "$W"/b*.pdf)" = %PDF ]'

  check "open shows the hive" '"$BIN/sbarhive" open "$W/b.jpg"'
  local n0; n0=$(notes)
  check "open with no matching actions fails" '! "$BIN/sbarhive" open "$ROOT/LICENSE"'
  check "and sends exactly one notification" '[ "$(notes)" -eq $((n0 + 1)) ]'

  n0=$(notes)
  "$BIN/sbarhive" run image.to-png --wait "$W/b.jpg" >/dev/null 2>&1
  # The notification follows the Finished event the client waits for.
  for _ in $(seq 20); do [ "$(notes)" -gt "$n0" ] && break; sleep 0.1; done
  sleep 0.3
  check "a finished job sends one notification" '[ "$(notes)" -eq $((n0 + 1)) ]'

  mkdir -p "$XDG_CONFIG_HOME/sbarhive"
  printf 'notify = false\noutput = "subfolder"\noutput-folder = "out"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"
  "$BIN/sbarhive" reload
  n0=$(notes)
  "$BIN/sbarhive" run image.to-gif --wait "$W/b.jpg" >/dev/null 2>&1
  check "subfolder setting puts output in out/" '[ -f "$W/out/b.gif" ] && [ ! -f "$W/b.gif" ]'
  sleep 1
  check "notify = false sends no notification" '[ "$(notes)" -eq "$n0" ]'
  rm -f "$XDG_CONFIG_HOME/sbarhive/config.toml"; "$BIN/sbarhive" reload
  check "convert works without daemon" '"$BIN/sbarhive" convert --to png "$W/b.jpg" >/dev/null && [ -f "$W/b.png" ]'

  "$BIN/sbarhive" quit >/dev/null 2>&1
  for _ in $(seq 30); do kill -0 $d 2>/dev/null || break; sleep 0.1; done
  check "quit stops the daemon and removes the socket" '! kill -0 $d 2>/dev/null && [ ! -S "$XDG_RUNTIME_DIR/sbarhive.sock" ]'
  kill $d 2>/dev/null
  kill $kwin 2>/dev/null
  timeout 5 tail --pid=$kwin -f /dev/null || kill -9 $kwin
  [ $fail -eq 0 ] && echo "e2e: all passed" || { echo "e2e: failures, logs in $SANDBOX"; cat "$SANDBOX/daemon.log" | tail -20; }
  return $fail
}
export -f inner

HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh-e2e \
  dbus-run-session -- bash -c inner
