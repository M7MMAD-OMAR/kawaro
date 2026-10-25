#!/usr/bin/env bash
# Builds the website and stages exactly what gets published into .deploy-out/
# (the assets directory in wrangler.jsonc): no build.py, no caches, plus the
# security headers and security.txt. Used by scripts/deploy-site.sh and by the
# Cloudflare Workers Builds build command.
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/build-site.sh
rm -rf .deploy-out
mkdir .deploy-out
cp -R site/. .deploy-out/
rm -rf .deploy-out/build.py .deploy-out/.nojekyll
find .deploy-out -name __pycache__ -type d -prune -exec rm -rf {} +
python3 scripts/site-headers.py .deploy-out
