#!/usr/bin/env bash
# Fails on a release download link that cannot work (S-061 AC6).
#
# Usage: scripts/site/check-links.sh [--online] [--base URL] [ROOT]
#   ROOT (default: the repository) holds site/**/*.html, README*.md and the git repo.
#   It reads every https://github.com/OWNER/REPO/releases/download/TAG/FILE URL from
#   ROOT/site/**/*.html and ROOT/README*.md.
#   Offline (default): the tag must exist in the local repository (git tag -l).
#   --online: every URL is requested; HTTP 404 fails. --base URL replaces the scheme
#   and host of the request only (for tests against a local server).
# Prints each bad URL and exits 1; exits 0 when all pass or there are none.
set -u
online=0
base=""
root=""
while [ $# -gt 0 ]; do
  case "$1" in
    --online) online=1 ;;
    --base) shift; base="${1:-}" ;;
    *) root="$1" ;;
  esac
  shift
done
root="${root:-$(cd "$(dirname "$0")/../.." && pwd)}"

urls=$(
  { find "$root/site" -name '*.html' -not -path '*/pagefind/*' -print0 2>/dev/null | xargs -0 -r cat
    cat "$root"/README*.md 2>/dev/null; } |
    grep -oE 'https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/releases/download/[^/"'"'"'<> )]+/[^"'"'"'<> )]+' |
    sort -u
)
[ -z "$urls" ] && { echo "check-links: no release download links"; exit 0; }

fail=0
while IFS= read -r url; do
  if [ "$online" = 1 ]; then
    target="$url"
    [ -n "$base" ] && target="${base%/}/${url#https://github.com/}"
    code=$(curl -sIL -o /dev/null -w '%{http_code}' --max-time 30 "$target" || echo 000)
    if [ "$code" = 404 ]; then echo "check-links: HTTP 404 $url"; fail=1; fi
  else
    tag=${url#*/releases/download/}
    tag=${tag%%/*}
    if [ -z "$(git -C "$root" tag -l "$tag" 2>/dev/null)" ]; then
      echo "check-links: tag $tag is missing locally: $url"; fail=1
    fi
  fi
done <<< "$urls"
[ "$fail" = 0 ] && echo "check-links: ok"
exit "$fail"
