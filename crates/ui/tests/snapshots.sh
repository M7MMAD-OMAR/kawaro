#!/usr/bin/env bash
# Snapshot tests for the hive: renders it in Arabic and English, dark and
# light, and compares with the reference PNGs in crates/ui/tests/snapshots/.
#
# Everything runs inside a private headless KWin (kwin_wayland --virtual) with
# a throwaway HOME and XDG directories, its own XDG_RUNTIME_DIR and Wayland
# socket, and its own session bus. Nothing touches the live desktop.
#
# Usage: crates/ui/tests/snapshots.sh [--update]
#   --update  rewrites the references (review them before committing).
# Exit code 0 when every snapshot matches.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
REF="$HERE/snapshots"
UPDATE=0
[ "${1:-}" = "--update" ] && UPDATE=1

(cd "$ROOT" && cargo build --quiet --release -p sbarhive-ui --example hive_snapshot) || exit 2
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/release/examples/hive_snapshot"
OUT="$(mktemp -d)"
mkdir -p "$REF"

inner() {
  unset DISPLAY XDG_ACTIVATION_TOKEN
  kwin_wayland --virtual --socket "$WAYLAND_DISPLAY" --width 1920 --height 1080 >"$OUT/kwin.log" 2>&1 &
  local cpid=$!
  for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  if [ ! -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]; then
    echo "private compositor did not start"; tail "$OUT/kwin.log"; kill $cpid 2>/dev/null; return 2
  fi
  sleep 1
  local fail=0
  for lang in en ar; do
    for scheme in dark light; do
      local name="hive-$lang-$scheme.png"
      local cmp=(--compare "$REF/$name")
      [ "$UPDATE" = 1 ] && cmp=()
      if ! SBARHIVE_COLOR_SCHEME=$scheme SBARHIVE_REDUCED_MOTION=1 \
          "$BIN" "$OUT/$name" --lang "$lang" --highlight 0 "${cmp[@]}"; then
        echo "FAIL $name (actual: $OUT/$name)"; fail=1
      fi
    done
  done
  kill $cpid 2>/dev/null
  timeout 5 tail --pid=$cpid -f /dev/null || kill -9 $cpid 2>/dev/null
  return $fail
}

export -f inner
export BIN OUT REF UPDATE
SANDBOX="$(mktemp -d)"
RUNTIME="$(mktemp -d)"
chmod 700 "$RUNTIME"
# Point the private session bus at the private compositor before it starts,
# so nothing it activates can reach the live desktop. Desktop identity
# variables go too, so portals do not try the live Hyprland or GNOME.
env -u WAYLAND_DISPLAY -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE \
  -u XDG_CURRENT_DESKTOP -u XDG_SESSION_DESKTOP -u DESKTOP_SESSION \
  -u SWAYSOCK -u I3SOCK -u DBUS_SESSION_BUS_ADDRESS \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY=wayland-sbh-snap \
  dbus-run-session -- bash -c inner
status=$?

if [ "$UPDATE" = 1 ] && [ $status -eq 0 ]; then
  cp "$OUT"/hive-*.png "$REF"/
  echo "references updated in $REF"
fi
rm -rf "$SANDBOX" "$RUNTIME"
if [ $status -eq 0 ]; then
  rm -rf "$OUT"; echo "snapshots: ok"
else
  echo "snapshots: FAILED (outputs in $OUT)"
fi
exit $status
