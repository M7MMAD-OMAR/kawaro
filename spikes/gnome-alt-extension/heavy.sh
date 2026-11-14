#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Heavy wrapper (CLAUDE.md, "Running heavy work"): systemd-run substitutes $
# in its arguments, so the commands live in this script. Run with:
#   flock /home/sbarah/.cache/kawaro-heavy.lock systemd-run --user --scope \
#     -p MemoryMax=6G -p CPUQuota=200% bash spikes/gnome-alt-extension/heavy.sh
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
export CARGO_BUILD_JOBS=2
# Raw logs land beside the report
# /home/sbarah/.hermes/checkpoint-runs/kawaro-stable-20261026/gnome-alt-extension-spike.md.
export SPIKE_OUT=/home/sbarah/.hermes/checkpoint-runs/kawaro-stable-20261026/gnome-alt-extension-logs
mkdir -p "$SPIKE_OUT"
bash "$HERE/run.sh"
