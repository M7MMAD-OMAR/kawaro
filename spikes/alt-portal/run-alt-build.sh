#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
export CARGO_BUILD_JOBS=2
cargo build --release
cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml
cargo test --release -p kawaro-daemon ac13_one_shortcut_drop_with_alt_in_the_shortcuts_spec_form
