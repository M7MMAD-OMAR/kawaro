#!/usr/bin/env bash
# Runs every integration and packaging test. Exit code 0 only when all pass.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
rc=0
for t in test_static.sh test_merge_uca.sh test_hyprland.sh test_install.sh; do
  echo "### $t"
  bash "$HERE/$t" || rc=1
done
for mode in real stub; do
  echo "### test_nautilus.py $mode"
  env -u WAYLAND_DISPLAY -u DISPLAY python3 "$HERE/test_nautilus.py" "$mode" || rc=1
done
if [ "$rc" -eq 0 ]; then echo "ALL PASS"; else echo "SOME FAILED"; fi
exit "$rc"
