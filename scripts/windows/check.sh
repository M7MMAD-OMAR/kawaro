#!/usr/bin/env bash
# Runs clippy for Windows (x86_64-pc-windows-gnu) on core, engines and the
# CLI (every target), then on the daemon and the UI (libraries and binaries),
# warnings as errors, with two jobs. Outside CI it re-runs
# itself inside a 6 GB, two CPU systemd scope, like build.sh.
#   scripts/windows/check.sh
# CARGO_TARGET_DIR defaults to ${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target.
set -eu
# shellcheck source=/dev/null
. "$(dirname "$0")/scope.sh"
win_scope "$@"
echo "cgroup: $(cut -d: -f3 /proc/self/cgroup | head -n 1)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$KAWARO_WIN_TARGET_DEFAULT}"
cd "$(dirname "$0")/../.."
cargo clippy -j 2 --target x86_64-pc-windows-gnu \
  -p kawaro-core -p kawaro-engines -p kawaro-cli --all-targets -- -D warnings
exec cargo clippy -j 2 --target x86_64-pc-windows-gnu \
  -p kawaro-daemon -p kawaro-ui --lib --bins -- -D warnings
