#!/usr/bin/env bash
# Candidate-only S-139 matrix, 60 actual full 393x362 Slint frames.
# No builds, reference writes, live compositor, user config or installed app.
# Usage: bash crates/ui/tests/options_snapshots.sh OUT [--compare REF_DIR] [--bin BIN]
# OUT must not exist. Crop numeric-invalid is explicitly N/A.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
[[ $# -ge 1 ]] || { echo "usage: $0 OUT [--compare REF_DIR] [--bin BIN]" >&2; exit 2; }
OUT="$(realpath -m "$1")"; shift
BIN="$ROOT/target/debug/examples/options_snapshot"
REF=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --compare) [[ $# -ge 2 ]] || exit 2; REF="$(realpath -e "$2")"; shift 2 ;;
    --bin) [[ $# -ge 2 ]] || exit 2; BIN="$(realpath -e "$2")"; shift 2 ;;
    *) echo "unknown argument: $1 (no --update mode)" >&2; exit 2 ;;
  esac
done
[[ -x "$BIN" ]] || { echo "Build example only after Lead GO: $BIN" >&2; exit 2; }
[[ ! -e "$OUT" ]] || { echo "refusing existing output directory: $OUT" >&2; exit 2; }
mkdir -m 700 "$OUT"
OUT="$(realpath -e "$OUT")"
mkdir -m 700 "$OUT/home" "$OUT/runtime" "$OUT/tmp"
FIXTURE="$ROOT/tests/fixtures/images/kawaro-sample.png"
fingerprint() {
    sha256sum "$BIN" "$FIXTURE" "$ROOT/design/tokens.json" \
  "$ROOT/crates/ui/ui/hive.slint" "$ROOT/crates/ui/src/lib.rs" \
  "$ROOT/crates/ui/src/stage_two.rs" "$ROOT/crates/ui/examples/options_snapshot.rs"
}
fingerprint >"$OUT/before.sha256"
inner() {
  set -uo pipefail
  ulimit -c 0
  kwin_wayland --virtual --socket "$WAYLAND_DISPLAY" --width 1280 --height 800 >"$OUT/kwin.log" 2>&1 &
  local cpid=$!
  trap "kill $cpid 2>/dev/null || :; wait $cpid 2>/dev/null || :" EXIT
  local ready=0
  for ((i=0; i<100; i++)); do
    if [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]]; then ready=1; break; fi
    kill -0 "$cpid" 2>/dev/null || break
    sleep 0.1
  done
  [[ $ready = 1 ]] || { echo "private KWin failed, see $OUT/kwin.log" >&2; return 2; }
  local fail=0 action lang scheme frame name
  for action in resize-50 resize-to crop compress-to; do
    for lang in en ar; do
      for scheme in dark light; do
        for frame in initial focused invalid bottom; do
          if [[ $action = crop && $frame = invalid ]]; then
            echo "N/A crop-$lang-$scheme-invalid: no numeric schema field" >>"$OUT/matrix.log"
            continue
          fi
          name="options-$action-$lang-$scheme-$frame"
          local compare=()
          [[ -z "$REF" ]] || compare=(--compare "$REF/$name.png")
          if KAWARO_COLOR_SCHEME="$scheme" timeout 25 "$BIN" "$OUT/$name.png" \
              --action "image.$action" --lang "$lang" --frame "$frame" --file "$FIXTURE" \
              "${compare[@]}" >"$OUT/$name.log" 2>&1; then
            echo "PASS $name" >>"$OUT/matrix.log"
          else
            echo "FAIL $name, see $OUT/$name.log" | tee -a "$OUT/matrix.log" >&2
            fail=1
          fi
        done
      done
    done
  done
  return "$fail"
}
export -f inner
export BIN OUT REF FIXTURE
status=0
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS -u XDG_ACTIVATION_TOKEN \
  -u HYPRLAND_INSTANCE_SIGNATURE -u SWAYSOCK -u I3SOCK \
  -u XDG_CURRENT_DESKTOP -u XDG_SESSION_DESKTOP -u DESKTOP_SESSION \
  HOME="$OUT/home" XDG_CONFIG_HOME="$OUT/home/config" XDG_DATA_HOME="$OUT/home/data" \
  XDG_CACHE_HOME="$OUT/home/cache" XDG_STATE_HOME="$OUT/home/state" TMPDIR="$OUT/tmp" \
  XDG_RUNTIME_DIR="$OUT/runtime" WAYLAND_DISPLAY=wayland-kawaro-options \
  KAWARO_SURFACE=window KAWARO_REDUCED_MOTION=1 SLINT_SCALE_FACTOR=1 \
  dbus-run-session -- bash -c inner || status=$?
fingerprint >"$OUT/after.sha256"
cmp "$OUT/before.sha256" "$OUT/after.sha256" || status=1
echo "options candidates retained: $OUT (status $status)"
exit "$status"
