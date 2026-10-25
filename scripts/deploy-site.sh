#!/usr/bin/env bash
# Builds the website and publishes it as a Cloudflare Worker with static assets
# (worker "kawaro-site", custom domain https://kawaro.sbarah.com/ attached by
# wrangler.jsonc). Needs `wrangler login` once; no secrets in the repo.
# Normally Cloudflare Workers Builds does this on every push to main; this
# script is the manual fallback.
# Usage: scripts/deploy-site.sh
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/prepare-site-deploy.sh
npx --yes wrangler@4 deploy
