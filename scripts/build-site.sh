#!/usr/bin/env bash
# Builds the website into site/: design tokens, every page, then the search index.
# Pagefind (MIT, pinned) is fetched by npx at build time and is not shipped.
# Usage: scripts/build-site.sh [--no-search]
set -euo pipefail
cd "$(dirname "$0")/.."
python3 scripts/tokens-to-css.py
python3 site/build.py
if [ "${1:-}" != "--no-search" ]; then
  npx --yes pagefind@1.5.2 --site site --glob "{docs,en/docs,releases,en/releases}/**/*.html" --output-path site/pagefind
fi
