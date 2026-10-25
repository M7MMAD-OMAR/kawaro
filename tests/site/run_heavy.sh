#!/usr/bin/env bash
# Runs run_all.sh inside the memory/CPU scope required by CLAUDE.md.
exec systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200% env CARGO_BUILD_JOBS=1 \
  bash "$(cd "$(dirname "$0")" && pwd)/run_all.sh"
