#!/usr/bin/env bash
# S-090 AC8: renders the settings window with the Updates section in each of
# the update states, in Arabic and English, into a private headless KWin.
# Nothing touches the live desktop.
#
# Usage: crates/ui/tests/updates_screenshots.sh OUTDIR
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
OUTDIR="$(mkdir -p "${1:?usage: updates_screenshots.sh OUTDIR}" && cd "$1" && pwd)"
(cd "$ROOT" && cargo build --quiet --release -p kawaro-ui --example settings_snapshot) || exit 2
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/release/examples/settings_snapshot"

inner() {
  unset DISPLAY XDG_ACTIVATION_TOKEN
  kwin_wayland --virtual --socket "$WAYLAND_DISPLAY" --width 1920 --height 1080 >"$OUTDIR/kwin.log" 2>&1 &
  local cpid=$!
  for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "private compositor did not start"; kill $cpid; return 2; }
  sleep 1
  local fail=0
  for lang in en ar; do
    for state in up-to-date checking downloading ready failed unavailable off; do
      KAWARO_COLOR_SCHEME=light KAWARO_REDUCED_MOTION=1 \
        "$BIN" "$OUTDIR/updates-$lang-$state.png" --lang "$lang" --update-state "$state" || fail=1
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
