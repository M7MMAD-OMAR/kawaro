#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Shared helpers for the S-053c release tests. Sourced, never executed.
# Every test works in a throwaway temp dir with throwaway minisign keys
# (minisign -G -W or a sentinel passphrase). The real signing key is never
# read, copied or named here.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export REPO_ROOT
FAILS=0
CHECKS=0
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

pass() { CHECKS=$((CHECKS + 1)); echo "PASS $*"; }
fail() { CHECKS=$((CHECKS + 1)); FAILS=$((FAILS + 1)); echo "FAIL $*" >&2; }
# check "AC1 description" command [args...]
check() { local d="$1"; shift; if "$@"; then pass "$d"; else fail "$d"; fi; }
# expect_status "AC1 description" WANT command [args...]  (output is discarded)
expect_status() {
  local d="$1" want="$2" got
  shift 2
  "$@" >/dev/null 2>&1
  got=$?
  if [ "$got" = "$want" ]; then pass "$d"; else fail "$d (exit $got, want $want)"; fi
}
finish() {
  echo "$CHECKS checks, $FAILS failed"
  [ "$FAILS" -eq 0 ]
}
