#!/usr/bin/env bash
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
export CARGO_BUILD_JOBS=2
export PATH="/usr/bin:$PATH"
export DROP_OUT="$ROOT/out"
mkdir -p "$DROP_OUT"
rpm -q xdg-desktop-portal xdg-desktop-portal-gnome gnome-shell mutter gnome-control-center
bash tests/drop/gnome.sh smart portal-real
rc=$?
printf 'GNOME_ALT_PORTAL_EXIT=%s\n' "$rc"
exit "$rc"
