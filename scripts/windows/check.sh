#!/usr/bin/env bash
# Runs clippy for Windows (x86_64-pc-windows-gnu) on core, engines and the
# CLI, every target, warnings as errors, with two jobs. Outside CI it re-runs
# itself inside a 6 GB, two CPU systemd scope, like build.sh.
#   scripts/windows/check.sh
# CARGO_TARGET_DIR defaults to ${XDG_CACHE_HOME:-$HOME/.cache}/sbarhive-win-target.
set -eu
if [ "${CI:-}" != true ] && [ -z "${SBARHIVE_WIN_SCOPE:-}" ]; then
  exec systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200% \
    env SBARHIVE_WIN_SCOPE=1 "$0" "$@"
fi
echo "cgroup: $(cut -d: -f3 /proc/self/cgroup | head -n 1)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/sbarhive-win-target}"
cd "$(dirname "$0")/../.."
exec cargo clippy -j 2 --target x86_64-pc-windows-gnu \
  -p sbarhive-core -p sbarhive-engines -p sbarhive-cli --all-targets -- -D warnings
