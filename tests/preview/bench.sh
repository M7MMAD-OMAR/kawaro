#!/usr/bin/env bash
# S-118 AC4: the preview budget bench. Builds and runs
# crates/ui/tests/preview_bench.rs in release inside a memory and CPU limited
# scope, prints probe_ms, draft_ms, mask_ms and draft_bytes, and fails on a
# budget regression (probe 5 ms, 24 MP draft 500 ms, mask 50 ms, draft 4 MB).
# Release, because the decode tables and resampling are optimised code in the
# shipped app; the test is marked ignore in the debug suite, where the same
# decode is about ten times slower. --nocapture is required: the numbers are
# stdout, the assertions are the exit code.
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"
exec systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=100% \
  nice -n 15 env CARGO_BUILD_JOBS=1 \
  cargo test --release -p kawaro-ui --test preview_bench \
    -- --ignored --nocapture --test-threads=1
