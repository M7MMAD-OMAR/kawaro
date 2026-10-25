#!/usr/bin/env bash
# Builds the website and publishes it as a Cloudflare Worker with static assets
# (worker "sbarhive-site", custom domain https://hive.sbarah.com/ attached by
# wrangler.jsonc). Needs `wrangler login` once; no secrets in the repo.
# Usage: scripts/deploy-site.sh
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/build-site.sh
rm -rf .deploy-out
mkdir .deploy-out
rsync -a --exclude 'build.py' --exclude '__pycache__' --exclude '.nojekyll' site/ .deploy-out/
python3 scripts/site-headers.py .deploy-out
npx --yes wrangler@4 deploy
