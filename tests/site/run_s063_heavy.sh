#!/usr/bin/env bash
exec systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200% env CARGO_BUILD_JOBS=1 \
  bash "$(cd "$(dirname "$0")" && pwd)/run_s063.sh"
