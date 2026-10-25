#!/usr/bin/env bash
# S-063 tests only. Heavy: run through systemd-run (see run_s063_heavy.sh).
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
export S061_TMP="$(mktemp -d)"
trap 'rm -rf "$S061_TMP"' EXIT
cd "$S061_TMP"
rc=0
echo "=== test_s063_static.py"; python3 -I "$HERE/test_s063_static.py" || rc=1
echo "=== test_s063_browser.py"; python3 "$HERE/test_s063_browser.py" || rc=1
exit $rc
