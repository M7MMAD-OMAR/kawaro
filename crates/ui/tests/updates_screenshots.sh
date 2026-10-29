#!/usr/bin/env bash
# S-090 AC8 and S-099b B8: renders the settings window on the Updates page in
# each update state, in Arabic and English, into a private headless KWin.
# Nothing touches the live desktop.
#
# Usage: crates/ui/tests/updates_screenshots.sh [OUTDIR]
#   With no OUTDIR it writes the 16 S-099b images (four states: downloading,
#   ready, installing, updated; en and ar; light and dark) to
#   docs/screenshots/s099 as updates-<lang>-<scheme>-<state>.png.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
OUTDIR="$(mkdir -p "${1:-$ROOT/docs/screenshots/s099}" && cd "${1:-$ROOT/docs/screenshots/s099}" && pwd)"
(cd "$ROOT" && cargo build --quiet --release -p kawaro-ui --example settings_snapshot) || exit 2
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/release/examples/settings_snapshot"

inner() {
  unset DISPLAY XDG_ACTIVATION_TOKEN
  kwin_wayland --virtual --socket "$WAYLAND_DISPLAY" --width 1920 --height 1400 >"$OUTDIR/kwin.log" 2>&1 &
  local cpid=$!
  for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "private compositor did not start"; kill $cpid; return 2; }
  sleep 1
  local fail=0
  for lang in en ar; do
    for scheme in light dark; do
      for state in downloading ready installing updated; do
        KAWARO_COLOR_SCHEME=$scheme KAWARO_REDUCED_MOTION=1 \
          "$BIN" "$OUTDIR/updates-$lang-$scheme-$state.png" --lang "$lang" \
          --page 3 --size 880x1230 --update-state "$state" || fail=1
      done
    done
  done
  # The real default size 880x640, where the page scrolls: ready and updated
  # in each language show that the notes that matter are on screen.
  mkdir -p "$OUTDIR/default-size"
  for lang in en ar; do
    for state in ready updated; do
      KAWARO_COLOR_SCHEME=light KAWARO_REDUCED_MOTION=1 \
        "$BIN" "$OUTDIR/default-size/updates-$lang-light-$state-640.png" --lang "$lang" \
        --page 3 --size 880x640 --update-state "$state" || fail=1
    done
  done
  kill $cpid 2>/dev/null
  timeout 5 tail --pid=$cpid -f /dev/null || kill -9 $cpid 2>/dev/null
  return $fail
}
export -f inner
export BIN OUTDIR
SANDBOX="$(mktemp -d)"
RUNTIME="$(mktemp -d)"
chmod 700 "$RUNTIME"
env -u WAYLAND_DISPLAY -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE \
  -u XDG_CURRENT_DESKTOP -u XDG_SESSION_DESKTOP -u DESKTOP_SESSION \
  -u SWAYSOCK -u I3SOCK -u DBUS_SESSION_BUS_ADDRESS \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY=wayland-sbh-s090 \
  dbus-run-session -- bash -c inner
status=$?
rm -rf "$SANDBOX" "$RUNTIME" "$OUTDIR/kwin.log"
exit $status
