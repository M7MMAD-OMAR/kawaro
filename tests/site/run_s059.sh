#!/usr/bin/env bash
# S-059 tests only. Heavy: run through run_s059_heavy.sh (systemd-run scope).
# Every test stages a COPY of the repo in a temp dir; site/ and .deploy-out/ are never touched.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
export S061_TMP="$(mktemp -d)"
trap 'rm -rf "$S061_TMP"' EXIT
cd "$S061_TMP"
rc=0
echo "=== test_s059_deploy.py"; /usr/bin/python3 -I "$HERE/test_s059_deploy.py" || rc=1
echo "=== test_s059_browser.py"; /usr/bin/python3 "$HERE/test_s059_browser.py" || rc=1
exit $rc
