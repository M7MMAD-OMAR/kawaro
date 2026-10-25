#!/usr/bin/env bash
# Builds kawaro.exe and kawarod.exe for Windows (x86_64-pc-windows-gnu, release) with two
# jobs. Outside CI it re-runs itself inside a 6 GB, two CPU systemd scope,
# so a cross build never starves the desktop. The first line of output is
# the cgroup the build runs in.
#   scripts/windows/build.sh
# CARGO_TARGET_DIR defaults to ${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target.
set -eu
# shellcheck source=/dev/null
. "$(dirname "$0")/scope.sh"
win_scope "$@"
echo "cgroup: $(cut -d: -f3 /proc/self/cgroup | head -n 1)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$KAWARO_WIN_TARGET_DEFAULT}"
cd "$(dirname "$0")/../.."
exec cargo build -j 2 --release --target x86_64-pc-windows-gnu -p kawaro-cli -p kawaro-daemon
