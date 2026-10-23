#!/usr/bin/env bash
# Snapshot tests for the settings window: renders it in Arabic and English, dark and
# light, and compares with the reference PNGs in crates/ui/tests/snapshots/.
#
# Everything runs inside a private headless KWin (kwin_wayland --virtual) with
# a throwaway HOME and XDG directories, its own XDG_RUNTIME_DIR and Wayland
# socket, and its own session bus. Nothing touches the live desktop.
#
# Usage: crates/ui/tests/settings_snapshots.sh [--update]
#   --update  rewrites the references (review them before committing).
# Exit code 0 when every snapshot matches.
#
# Every run writes the four renders as candidates to $CAND (printed below),
# so a missing or changed reference can be looked at and approved. A missing
# reference fails, and so does any of the four references from before
# S-014f1 (no portal_shortcut row), retired by hash below: new references
# replace them only after the manager approves the candidates (S-014 AC24).
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
REF="$HERE/snapshots"
UPDATE=0
[ "${1:-}" = "--update" ] && UPDATE=1

(cd "$ROOT" && cargo build --quiet --release -p sbarhive-ui --example settings_snapshot) || exit 2
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/release/examples/settings_snapshot"
OUT="$(mktemp -d)"
CAND="${CARGO_TARGET_DIR:-$ROOT/target}/settings-snapshot-candidates"
rm -rf "$CAND"
mkdir -p "$REF" "$CAND"
# sha256 of the references from before the S-014f1 portal_shortcut row.
RETIRED="725cb5de093858fde929ca847d672382bbfba77867777aaa97121a3f4c1a87bb
99f3bee2dcbe7537576a17109d1b88a4ab14a97190f8c4398bd22ad8c47e1e92
fa41e6257e7fd876f8275438d87e468b21dd83c55f3b4deb47c81e1f858dcc0b
92502b3f931dc9f5338fdae616e9c5286fade274061182092ed5eff0d4718f41"

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
      local name="settings-$lang-$scheme.png"
      local cmp=(--compare "$REF/$name")
      local missing=0
      if [ "$UPDATE" = 1 ]; then
        cmp=()
      elif [ ! -f "$REF/$name" ]; then
        cmp=(); missing=1
      elif printf '%s\n' "$RETIRED" | grep -qx "$(sha256sum "$REF/$name" | cut -d' ' -f1)"; then
        cmp=(); missing=2
      fi
      if ! SBARHIVE_COLOR_SCHEME=$scheme SBARHIVE_REDUCED_MOTION=1 \
          "$BIN" "$OUT/$name" --lang "$lang" "${cmp[@]}"; then
        echo "FAIL $name (actual: $OUT/$name)"; fail=1
      fi
      cp "$OUT/$name" "$CAND/$name" 2>/dev/null
      if [ $missing = 1 ]; then
        echo "FAIL $name: no approved reference (candidate: $CAND/$name, needs the manager's approval)"; fail=1
      elif [ $missing = 2 ]; then
        echo "FAIL $name: reference predates the S-014f1 portal_shortcut row (candidate: $CAND/$name, needs the manager's approval)"; fail=1
      fi
    done
  done
  kill $cpid 2>/dev/null
  timeout 5 tail --pid=$cpid -f /dev/null || kill -9 $cpid 2>/dev/null
  return $fail
}

export -f inner
export BIN OUT REF UPDATE CAND RETIRED
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
  XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY=wayland-sbh-settings \
  dbus-run-session -- bash -c inner
status=$?

if [ "$UPDATE" = 1 ] && [ $status -eq 0 ]; then
  cp "$OUT"/settings-*.png "$REF"/
  echo "references updated in $REF"
fi
rm -rf "$SANDBOX" "$RUNTIME"
echo "candidates: $(ls "$CAND"/settings-*.png 2>/dev/null | wc -l) in $CAND"
if [ $status -eq 0 ]; then
  rm -rf "$OUT"; echo "snapshots: ok"
else
  echo "snapshots: FAILED (outputs in $OUT)"
fi
exit $status
