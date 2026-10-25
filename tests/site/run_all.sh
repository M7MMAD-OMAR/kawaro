#!/usr/bin/env bash
# S-061 site tests. Heavy: run it through run_heavy.sh (systemd-run scope).
# Static and script tests run with python3 -I; browser tests need the user site
# (playwright), so they run without -I from a neutral cwd.
# Every test builds a COPY of the repo in a temp dir; site/ is never touched.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
export S061_TMP="$(mktemp -d)"
trap 'rm -rf "$S061_TMP"' EXIT
cd "$S061_TMP"
rc=0
for t in test_static.py test_links.py test_contrast.py test_s063_static.py test_s064_static.py test_s068_static.py test_s071_static.py; do
  echo "=== $t"; python3 -I "$HERE/$t" || rc=1
done
echo "=== test_browser.py"; python3 "$HERE/test_browser.py" || rc=1
echo "=== test_s063_browser.py"; python3 "$HERE/test_s063_browser.py" || rc=1
echo "=== test_s064_browser.py"; python3 "$HERE/test_s064_browser.py" || rc=1
echo "=== test_s068_browser.py"; python3 "$HERE/test_s068_browser.py" || rc=1
exit $rc
