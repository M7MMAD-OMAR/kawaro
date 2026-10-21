#!/usr/bin/env bash
# End-to-end test of the built binaries inside a private headless KWin with a
# throwaway HOME and session bus. Never touches the live desktop or profile.
# S-019c (backlog/S-019-hive-at-pointer.md, AC7): a second private KWin at
# 1920x1200, scale 1.5 (1280x800 logical, seeded in kwinoutputconfig.json as
# tests/overlay/kwin.sh 1.5 does), shows the largest hive 20 times: the
# pointer at the middle of the top edge, (640, 0) logical, and photo.png with
# 18 actions (15 built-in, three user ones). Each show closes with Esc; then
# the daemon's RSS must be 40 MB or less. Every show must be anchored there:
# a "hive anchor" line within 1 px of (640, 0), and a "hive surface" line
# equal to the AC3 placement for it (tests/overlay/anchor.py, 1 px).
# Usage: tests/e2e/run.sh [target/release]
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${1:-$ROOT/target/release}"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
# S-018 AC18: Esc closes the shown hive before the memory checks; on the layer
# path focus loss no longer closes it. Input comes from fake input.
INPUT="$ROOT/tests/drop/kwin-input/target/release/kwin-input"
[ -x "$INPUT" ] || { echo "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"; exit 2; }
SANDBOX="$(mktemp -d)"
export BIN SANDBOX ROOT INPUT

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

  # S-014a AC4: drop with no daemon running must start one within 2 s, like open.
  check "drop starts the daemon" '"$BIN/sbarhive" drop && "$BIN/sbarhive" ping | grep -q pong'
  "$BIN/sbarhive" quit >/dev/null 2>&1
  for _ in $(seq 30); do [ -S "$XDG_RUNTIME_DIR/sbarhive.sock" ] || break; sleep 0.1; done

  # S-018 AC18: the hive surface and focus lines and the Wayland trace show
  # the hive as a layer and its close by Esc below.
  SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$SANDBOX/daemon.log" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
  check "daemon answers ping" '"$BIN/sbarhive" ping | grep -q pong'
  local inode; inode=$(stat -c %i "$XDG_RUNTIME_DIR/sbarhive.sock")
  check "single instance" '"$BIN/sbarhived" 2>&1 | grep -q "already running"'
  check "second instance leaves the socket alone" '[ "$(stat -c %i "$XDG_RUNTIME_DIR/sbarhive.sock")" = "$inode" ] && "$BIN/sbarhive" ping | grep -q pong'
  notes() { grep -c "sbarhived: notify:" "$SANDBOX/daemon.log"; }
  rss_mb() { awk '/^VmRSS:/ {print int($2 / 1024)}' "/proc/$d/status"; }
  # S-001 AC2: the idle service stays within 40 MB.
  sleep 2
  local rss0; rss0=$(rss_mb)
  echo "INFO idle memory at start: ${rss0} MB"
  check "idle memory at start within 40 MB" '[ "$rss0" -le 40 ]'

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
  # S-018 AC18: close the hive explicitly, so the memory check below measures
  # the daemon after a hide. On KWin the hive is a layer surface (AC1) and
  # takes the keyboard (AC4), so Esc from fake input closes it (AC9).
  for _ in $(seq 30); do grep -q '^hive focus: true$' "$SANDBOX/daemon.log" && break; sleep 0.1; done
  check "S-018 AC18: the hive is a layer surface with the keyboard" \
    'grep -qE "^hive surface: layer " "$SANDBOX/daemon.log" && grep -q "^hive focus: true$" "$SANDBOX/daemon.log"'
  local inf="$XDG_RUNTIME_DIR/in"; mkfifo "$inf"
  "$INPUT" <"$inf" >"$SANDBOX/input.log" 2>&1 &
  exec 3>"$inf"
  sleep 0.5
  echo "keydown 1" >&3; echo "sleep 40" >&3; echo "keyup 1" >&3
  exec 3>&-
  sleep 1
  check "S-018 AC18: Esc closes the hive before the memory checks (zwlr_layer_surface_v1.destroy)" \
    'grep -qE -- "-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\(" "$SANDBOX/daemon.log"'

  # v0.2 engines through the daemon.
  ffmpeg -loglevel error -f lavfi -i testsrc=size=160x120:rate=10 -f lavfi -i sine=frequency=440 -t 1 -shortest "$W/clip.mp4"
  "$BIN/sbarhive" run video.extract-mp3 --wait "$W/clip.mp4" >/dev/null 2>&1
  check "video: extract audio to MP3" '[ -f "$W/clip.mp3" ] && ffprobe -v error -show_entries stream=codec_type -of csv=p=0 "$W/clip.mp3" | grep -qx audio'
  check "video actions offered for mp4" '"$BIN/sbarhive" actions "$W/clip.mp4" | grep -q "^video.to-webm"'
  cp "$W"/b*.pdf "$W/one.pdf"; cp "$W/one.pdf" "$W/two.pdf"
  "$BIN/sbarhive" run pdf.merge --wait "$W/one.pdf" "$W/two.pdf" >/dev/null 2>&1
  check "pdf: merge two files" 'ls "$W"/one*merged*.pdf >/dev/null 2>&1'
  printf 'SbarHive e2e\nسطر عربي\n' >"$W/note.txt"
  if command -v soffice >/dev/null; then
    timeout 120 "$BIN/sbarhive" run documents.to-pdf --wait "$W/note.txt" >/dev/null 2>&1
    check "documents: text to PDF" '[ "$(head -c 4 "$W/note.pdf" 2>/dev/null)" = %PDF ]'
  fi
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
  # No `sbarhive reload`: the daemon notices the edited file by itself.
  sleep 3
  n0=$(notes)
  "$BIN/sbarhive" run image.to-gif --wait "$W/b.jpg" >/dev/null 2>&1
  check "edited settings apply without reload: output in out/" '[ -f "$W/out/b.gif" ] && [ ! -f "$W/b.gif" ]'
  sleep 1
  check "notify = false sends no notification" '[ "$(notes)" -eq "$n0" ]'
  rm -f "$XDG_CONFIG_HOME/sbarhive/config.toml"; "$BIN/sbarhive" reload
  check "convert works without daemon" '"$BIN/sbarhive" convert --to png "$W/b.jpg" >/dev/null && [ -f "$W/b.png" ]'

  sleep 2
  local rss1; rss1=$(rss_mb)
  echo "INFO idle memory after the hive and jobs: ${rss1} MB"
  check "idle memory after use within 40 MB" '[ "$rss1" -le 40 ]'

  # S-018 AC27: 20 shows of one file whose only thumbnail is a 1024 px
  # "xx-large" one (written into the sandbox cache by tests/overlay/thumbs.py),
  # each closed by Esc, then the memory again. With SBARHIVE_DEBUG_FOCUS the
  # hive prints "hive preview: thumb W H" per show (the size handed to Slint).
  mkdir -p "$W/preview"
  ffmpeg -loglevel error -f lavfi -i testsrc=size=1024x1024:rate=1 -frames:v 1 "$W/preview/photo.png"
  python3 -I "$ROOT/tests/overlay/thumbs.py" "$W/preview/photo.png" "$XDG_CACHE_HOME" xx-large 1024 1024 noise >/dev/null
  layers() { grep -cE "^hive surface: layer " "$SANDBOX/daemon.log"; }
  closes() { grep -cE -- "-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\(" "$SANDBOX/daemon.log"; }
  local p0 l0 c0 shows=0; p0=$(grep -c "^hive preview: thumb " "$SANDBOX/daemon.log"); l0=$(layers); c0=$(closes)
  local pf="$XDG_RUNTIME_DIR/in-preview"; mkfifo "$pf"
  "$INPUT" <"$pf" >"$SANDBOX/input-preview.log" 2>&1 &
  exec 4>"$pf"
  sleep 0.5
  for i in $(seq 20); do
    "$BIN/sbarhive" open "$W/preview/photo.png" || break
    for _ in $(seq 30); do [ "$(layers)" -ge $((l0 + i)) ] && break; sleep 0.1; done
    sleep 0.3
    echo "keydown 1" >&4; echo "sleep 40" >&4; echo "keyup 1" >&4
    for _ in $(seq 30); do [ "$(closes)" -ge $((c0 + i)) ] && break; sleep 0.1; done
    [ "$(closes)" -ge $((c0 + i)) ] && shows=$i
  done
  exec 4>&-
  local pv; pv=$(grep "^hive preview: thumb " "$SANDBOX/daemon.log" | tail -n +$((p0 + 1)))
  echo "INFO S-018 AC27: $shows shows closed, preview lines: $(sort <<<"$pv" | uniq -c | tr '\n' ';')"
  check "S-018 AC27: 20 shows of the file with a 1024 px xx-large thumbnail, each closed by Esc" '[ "$shows" -eq 20 ]'
  check "S-018 AC27: every show drew the thumbnail downscaled before Slint (20 \"hive preview: thumb W H\" lines, W and H 1 to 128)" \
    '[ "$(grep -c . <<<"$pv")" -eq 20 ] && awk '"'"'{ if ($4 < 1 || $5 < 1 || $4 > 128 || $5 > 128) bad = 1 } END { exit bad }'"'"' <<<"$pv"'
  sleep 2
  local rss2; rss2=$(rss_mb)
  echo "INFO memory after 20 preview shows: ${rss2} MB"
  check "S-018 AC27: memory after 20 preview shows within 40 MB" '[ "$rss2" -le 40 ]'
  # After the idle measurement: the settings window stays open until closed.
  check "settings opens from the launcher command" '"$BIN/sbarhive" settings && sleep 0.5 && ! grep -q "cannot .*settings" "$SANDBOX/daemon.log"'

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

# S-019 AC7: memory after 20 shows of the largest anchored hive.
rss_inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP
  local fail=0
  check() { if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; fail=1; fi; }
  mkdir -p "$XDG_CONFIG_HOME/sbarhive/actions"
  cat >"$XDG_CONFIG_HOME/kwinoutputconfig.json" <<'JSON'
[{"name": "outputs", "data": [{"connectorName": "Virtual-0", "mode": {"width": 1920, "height": 1200, "refreshRate": 60000}, "scale": 1.5, "transform": "Normal"}]},
 {"name": "setups", "data": [{"lidClosed": false, "outputs": [{"enabled": true, "outputIndex": 0, "position": {"x": 0, "y": 0}, "priority": 0}]}]}]
JSON
  printf 'language = "en"\ntheme = "light"\n' >"$XDG_CONFIG_HOME/sbarhive/config.toml"
  local xa
  for xa in xa xb xc; do
    printf '[[action]]\nid = "extra.%s"\nengine = "image"\nop = "convert"\nring = 1\norder = 900\naccepts = ["image"]\noutput = { mode = "per-file", ext = "webp" }\ncode = { ar = "%s", en = "%s" }\nlabel = { ar = "%s", en = "%s" }\nicon = "image"\n\n' \
      "$xa" "${xa^^}" "${xa^^}" "${xa^^}" "${xa^^}"
  done >"$XDG_CONFIG_HOME/sbarhive/actions/extra.toml"
  kwin_wayland --virtual --socket "$WAYLAND_DISPLAY" --width 1920 --height 1200 >"$SANDBOX/kwin-rss.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  sleep 2
  local W="$SANDBOX/work-rss"; mkdir -p "$W"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/photo.png"
  local log="$SANDBOX/daemon-rss.log"
  SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$log" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
  local nact; nact=$("$BIN/sbarhive" actions "$W/photo.png" 2>/dev/null | grep -c .)
  check "S-019 AC7 setup: photo.png has 18 actions" '[ "$nact" -eq 18 ]'
  local inf="$XDG_RUNTIME_DIR/in-rss"; mkfifo "$inf"
  "$INPUT" <"$inf" >"$SANDBOX/input-rss.log" 2>&1 &
  local ip=$!
  exec 3>"$inf"
  sleep 0.5
  # The middle of the top edge, logical coordinates.
  echo "move 640 0" >&3; sleep 0.5
  layers() { grep -cE "^hive surface: layer " "$log"; }
  closes() { grep -cE -- "-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\(" "$log"; }
  local l0 c0 shows=0 i
  l0=$(layers); c0=$(closes)
  for i in $(seq 20); do
    "$BIN/sbarhive" open "$W/photo.png" || break
    for _ in $(seq 30); do [ "$(layers)" -ge $((l0 + i)) ] && break; sleep 0.1; done
    sleep 0.3
    echo "keydown 1" >&3; echo "sleep 40" >&3; echo "keyup 1" >&3
    for _ in $(seq 30); do [ "$(closes)" -ge $((c0 + i)) ] && break; sleep 0.1; done
    [ "$(closes)" -ge $((c0 + i)) ] && shows=$i
  done
  exec 3>&-
  local anchors surfaces orc
  anchors=$(grep -E '^hive anchor: ' "$log")
  surfaces=$(grep -E '^hive surface: layer ' "$log")
  orc=$(python3 -I "$ROOT/tests/overlay/anchor.py" "$ROOT/design/tokens.json" 1280 800 640 0 18)
  echo "INFO S-019 AC7: $shows shows closed; anchor lines: $(sort <<<"$anchors" | uniq -c | tr '\n' ';'); surface lines: $(sort <<<"$surfaces" | uniq -c | tr '\n' ';'); oracle for (640, 0): $(tr '\n' '|' <<<"$orc")"
  check "S-019 AC7: 20 shows of 18 cells with the pointer at (640, 0) on 1280x800 at scale 1.5, each closed by Esc" '[ "$shows" -eq 20 ]'
  check "S-019 AC7: every show is anchored at the pointer (20 \"hive anchor\" lines within 1 px of (640, 0))" \
    '[ "$(grep -c . <<<"$anchors")" -eq 20 ] && awk '"'"'{ dx = $3 - 640; dy = $4; if (dx < 0) dx = -dx; if (dy < 0) dy = -dy; if ($3 == "none" || dx > 1 || dy > 1) bad = 1 } END { exit bad }'"'"' <<<"$anchors"'
  # The box for the anchor (640, 0): the same for min_rings 1 and 2 here.
  local ox oy ow oh; read -r _ _ _ _ ox oy ow oh <<<"$(head -1 <<<"$orc")"
  check "S-019 AC7: every show is the largest box, plan_at for (640, 0) ($ox $oy $ow $oh, 1 px)" \
    '[ "$(grep -c . <<<"$surfaces")" -eq 20 ] && awk -v X="$ox" -v Y="$oy" -v W="$ow" -v H="$oh" '"'"'function a(v) { return v < 0 ? -v : v } { if (a($4 - X) > 1 || a($5 - Y) > 1 || a($6 - W) > 1 || a($7 - H) > 1) bad = 1 } END { exit bad }'"'"' <<<"$surfaces"'
  sleep 2
  local rss; rss=$(awk '/^VmRSS:/ {print int($2 / 1024)}' "/proc/$d/status")
  echo "INFO S-019 AC7: memory after 20 shows of the largest hive at scale 1.5: ${rss} MB"
  check "S-019 AC7: memory after 20 shows of the largest hive within 40 MB" '[ -n "$rss" ] && [ "$rss" -le 40 ]'
  "$BIN/sbarhive" quit >/dev/null 2>&1
  for _ in $(seq 30); do kill -0 $d 2>/dev/null || break; sleep 0.1; done
  kill $d $ip 2>/dev/null
  kill $kwin 2>/dev/null
  timeout 5 tail --pid=$kwin -f /dev/null || kill -9 $kwin
  [ $fail -eq 0 ] && echo "e2e S-019 AC7: all passed" || { echo "e2e S-019 AC7: failures, logs in $SANDBOX"; tail -20 "$log"; }
  return $fail
}
export -f rss_inner

# Environment set BEFORE dbus-run-session, so nothing the private bus
# activates (a desktop portal, for one) can reach the live desktop.
RUN1="$(mktemp -d)"
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  -u XDG_SESSION_DESKTOP -u DESKTOP_SESSION \
HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
XDG_RUNTIME_DIR="$RUN1" WAYLAND_DISPLAY=wayland-sbh-e2e \
KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 \
  dbus-run-session -- bash -c inner
rc1=$?
# The S-019 AC7 session: its own config dir and runtime dir in the sandbox.
RUN2="$(mktemp -d)"
mkdir -p "$SANDBOX/rss/.config" "$SANDBOX/rss/.local/share" "$SANDBOX/rss/.cache" "$SANDBOX/rss/.local/state"
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  -u XDG_SESSION_DESKTOP -u DESKTOP_SESSION \
HOME="$SANDBOX/rss" XDG_CONFIG_HOME="$SANDBOX/rss/.config" XDG_DATA_HOME="$SANDBOX/rss/.local/share" \
XDG_CACHE_HOME="$SANDBOX/rss/.cache" XDG_STATE_HOME="$SANDBOX/rss/.local/state" \
XDG_RUNTIME_DIR="$RUN2" WAYLAND_DISPLAY=wayland-sbh-e2e-rss \
KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 \
  dbus-run-session -- bash -c rss_inner
rc2=$?
# The private bus's FUSE mounts (gvfs, doc) can outlive it for a moment.
rm -rf "$RUN1" "$RUN2" 2>/dev/null || { sleep 1; rm -rf "$RUN1" "$RUN2"; }
[ "$rc1" -eq 0 ] && [ "$rc2" -eq 0 ]
