#!/usr/bin/env bash
# Builds sbarhive.exe for Windows (x86_64-pc-windows-gnu, release) with two
# jobs. Outside CI it re-runs itself inside a 6 GB, two CPU systemd scope,
# so a cross build never starves the desktop. The first line of output is
# the cgroup the build runs in.
#   scripts/windows/build.sh
# CARGO_TARGET_DIR defaults to /home/sbarah/.cache/sbarhive-win-target.
set -eu
if [ "${CI:-}" != true ] && [ -z "${SBARHIVE_WIN_SCOPE:-}" ]; then
  exec systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200% \
    env SBARHIVE_WIN_SCOPE=1 "$0" "$@"
fi
echo "cgroup: $(cut -d: -f3 /proc/self/cgroup | head -n 1)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/home/sbarah/.cache/sbarhive-win-target}"
cd "$(dirname "$0")/../.."
exec cargo build -j 2 --release --target x86_64-pc-windows-gnu -p sbarhive-cli
