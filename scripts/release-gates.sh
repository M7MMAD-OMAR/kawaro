#!/usr/bin/env bash
# Runs every quality gate from CLAUDE.md for a release and writes one report.
# Detached-safe: run with `setsid -f scripts/release-gates.sh OUT_DIR`; the
# report ends with a line "GATES DONE" when everything has run.
# GUI checks use the existing scripts, which run in private headless
# compositors with a throwaway HOME, never on the live desktop.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/target/release-gates}"
mkdir -p "$OUT"
REPORT="$OUT/report.txt"
cd "$ROOT" || exit 2
: >"$REPORT"
log() { echo "$*" | tee -a "$REPORT"; }
step() {
  local name="$1"; shift
  local t0; t0=$(date +%s)
  "$@" >"$OUT/$name.log" 2>&1
  local code=$?
  log "$(printf '%-28s exit=%-3s %4ss  %s' "$name" "$code" "$(( $(date +%s) - t0 ))" "$OUT/$name.log")"
}

log "release gates at $(git rev-parse --short HEAD) on $(date -Iseconds)"
step fmt cargo fmt --all --check
step clippy cargo clippy --workspace --all-targets -- -D warnings
step tests cargo test --workspace --release --no-fail-fast
grep -h "test result" "$OUT/tests.log" | awk '{p+=$4; f+=$6; i+=$8} END {print "  tests passed", p, "failed", f, "ignored", i}' | tee -a "$REPORT"
step design-lint scripts/design-lint.sh crates/ui
step long-dashes bash -c "! grep -rIlP '\x{2014}|\x{2013}' --exclude-dir=target --exclude-dir=.git --exclude-dir=vendor --exclude-dir=.claude --exclude-dir=graft --exclude-dir=mutants.out . "
step integrations bash tests/integrations/run_all.sh
step build-release cargo build --release --workspace
step e2e tests/e2e/run.sh
step hive-snapshots crates/ui/tests/snapshots.sh
step settings-snapshots crates/ui/tests/settings_snapshots.sh
step focus-gnome-strict env FOCUS_OUT="$OUT/focus" tests/focus/gnome.sh strict notoken token
step focus-kwin-extreme env FOCUS_OUT="$OUT/focus" tests/focus/kwin.sh notoken token level=4

# Mutation testing on a clean copy, so worktrees and build dirs are not copied.
M="$OUT/mutants-tree"
rm -rf "$M" && mkdir -p "$M" && git archive HEAD | tar -x -C "$M"
for f in crates/daemon/src/text.rs crates/core/src/config.rs crates/cli/src/clipboard.rs \
         crates/cli/src/main.rs crates/core/src/engine.rs crates/core/src/i18n.rs \
         crates/core/src/ipc.rs crates/core/src/action.rs crates/ui/src/layout.rs; do
  name="mutants-$(echo "$f" | tr '/' '_')"
  step "$name" bash -c "cd '$M' && cargo mutants --file '$f' -j 4 --timeout 90 --output '$OUT/$name'"
  grep -hE "MISSED" "$OUT/$name.log" | sed 's/^/  /' | tee -a "$REPORT"
done
log "GATES DONE"
