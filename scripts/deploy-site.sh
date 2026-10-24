#!/usr/bin/env bash
# Builds the website and publishes it to Cloudflare Pages (project "sbarhive",
# served at https://hive.sbarah.com/). Needs `wrangler login` once; no secrets in the repo.
# Usage: scripts/deploy-site.sh
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/build-site.sh
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
rsync -a --exclude 'build.py' --exclude '__pycache__' site/ "$out/"
npx --yes wrangler@4 pages deploy "$out" --project-name sbarhive --branch main --commit-dirty=true
