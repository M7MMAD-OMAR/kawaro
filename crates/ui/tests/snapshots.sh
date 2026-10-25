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
#
# S-018 AC7: every reference is compared twice, with SBARHIVE_SURFACE=layer
# (the layer-shell adapter) and with SBARHIVE_SURFACE=window (winit). Each run
# keeps a WAYLAND_DEBUG=1 trace, so a layer run that silently fell back to a
# window (or a window run that made a layer surface) fails. SNAP_SURFACES
# narrows the list; --update writes the references from the window run only.
#
# S-018 AC26 (S-018h): four more references show the selected files in the
# centre, with no cell highlighted (--highlight -1):
#   hive-en-dark-thumb.png   one photo.png with a 256 px "large" thumbnail
#   hive-ar-light-icon.png   one report.pdf, no thumbnail: kind icon and PDF
#   hive-en-light-stack.png  a.png b.png c.png: stack and count
#   hive-ar-dark-stack.png   the same, mirrored
# The files and the thumbnail are made in the sandbox (tests/overlay/thumbs.py
# writes into the sandbox's XDG_CACHE_HOME, never the owner's cache), with a
# fixed mtime, and XDG_DATA_DIRS is the MIME fixture. hive_snapshot takes
# each selected file as "--file PATH" (repeatable) and then presents them with
# Hive::present_files, reading thumbnails and globs from its environment like
# the daemon. The new references are written only by --update and need the
# owner's (or the manager's) approval before they are committed. Also checked:
# the four older references are byte for byte the ones S-018h started from,
# and the thumb snapshot shows the thumbnail's sky and sun in the centre.
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
  local fail=0 surface
  local surfaces; read -ra surfaces <<<"${SNAP_SURFACES:-layer window}"
  [ "$UPDATE" = 1 ] && surfaces=(window)
  for surface in "${surfaces[@]}"; do
    for lang in en ar; do
      for scheme in dark light; do
        local name="hive-$lang-$scheme.png"
        local out="$OUT/$surface-$name" trace="$OUT/$surface-${name%.png}.trace"
        local cmp=(--compare "$REF/$name")
        [ "$UPDATE" = 1 ] && { cmp=(); out="$OUT/$name"; }
        if ! SBARHIVE_SURFACE=$surface WAYLAND_DEBUG=1 SBARHIVE_COLOR_SCHEME=$scheme SBARHIVE_REDUCED_MOTION=1 \
            "$BIN" "$out" --lang "$lang" --highlight 0 "${cmp[@]}" 2>"$trace"; then
          echo "FAIL $surface $name (actual: $out)"; fail=1
        fi
        # S-062 AC6, AC7, AC9, AC10: opaque plate, edge band, cell centres and
        # complete labels, read from the rendered pixels.
        if ! python3 -I "$HERE/high_contrast.py" "$out" "$lang" "$scheme" "$ROOT/design/tokens.json" \
            "$REF/hive-cell-centres.txt" "$ROOT/crates/ui/fonts/IBMPlexSansArabic-SemiBold.otf"; then
          echo "FAIL S-062 $surface $name"; fail=1
        fi
        local layer xdg
        layer=$(grep -cE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$trace")
        xdg=$(grep -cE -- '-> xdg_wm_base[#@][0-9]+\.get_xdg_surface\(' "$trace")
        if [ "$surface" = layer ] && { [ "$layer" -lt 1 ] || [ "$xdg" -ne 0 ]; }; then
          echo "FAIL AC7 $surface $name: drawn on get_layer_surface $layer, get_xdg_surface $xdg (want >= 1 and 0)"; fail=1
        elif [ "$surface" = window ] && { [ "$layer" -ne 0 ] || [ "$xdg" -lt 1 ]; }; then
          echo "FAIL AC7 $surface $name: drawn on get_layer_surface $layer, get_xdg_surface $xdg (want 0 and >= 1)"; fail=1
        fi
      done
    done
  done
  preview "${surfaces[@]}" || fail=1
  kill $cpid 2>/dev/null
  timeout 5 tail --pid=$cpid -f /dev/null || kill -9 $cpid 2>/dev/null
  return $fail
}

# S-018 AC26: the four preview snapshots, for every surface in "$@".
preview() {
  local P="$HOME/preview" fail=0 surface entry
  mkdir -p "$P"
  for f in photo.png report.pdf a.png b.png c.png; do
    printf 'selected file %s\n' "$f" >"$P/$f"
    touch -d @1700000000 "$P/$f"
  done
  python3 -I "$ROOT/tests/overlay/thumbs.py" "$P/photo.png" "$XDG_CACHE_HOME" large 256 256 photo >/dev/null || return 1
  local entries=(
    "hive-en-dark-thumb.png en dark photo.png"
    "hive-ar-light-icon.png ar light report.pdf"
    "hive-en-light-stack.png en light a.png b.png c.png"
    "hive-ar-dark-stack.png ar dark a.png b.png c.png"
  )
  for surface in "$@"; do
    for entry in "${entries[@]}"; do
      local name lang scheme files args=()
      read -r name lang scheme files <<<"$entry"
      for f in $files; do args+=(--file "$P/$f"); done
      local out="$OUT/$surface-$name" trace="$OUT/$surface-${name%.png}.trace"
      local cmp=(--compare "$REF/$name")
      [ "$UPDATE" = 1 ] && { cmp=(); out="$OUT/$name"; }
      # The --file options go first, so the output path is the last bare word.
      if ! XDG_DATA_DIRS="$ROOT/tests/fixtures/mime/system" SBARHIVE_SURFACE=$surface WAYLAND_DEBUG=1 \
          SBARHIVE_COLOR_SCHEME=$scheme SBARHIVE_REDUCED_MOTION=1 \
          "$BIN" "${args[@]}" "$out" --lang "$lang" --highlight -1 "${cmp[@]}" 2>"$trace"; then
        echo "FAIL AC26 $surface $name (actual: $out)"; fail=1
      fi
      if [ "$name" = hive-en-dark-thumb.png ] && [ -f "$out" ]; then
        if ! python3 -I "$ROOT/tests/overlay/centre.py" "$out" 70,130,200 250,200,40; then
          echo "FAIL AC24 $surface $name: the thumbnail's sky and sun are not in the centre square"; fail=1
        fi
      fi
    done
  done
  # S-062 AC9 regenerated every reference, so the AC26 checksums of the four
  # older ones are gone; their cell positions are held by hive-cell-centres.txt.
  return $fail
}

export -f inner preview
export BIN OUT REF UPDATE SNAP_SURFACES ROOT HERE
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
