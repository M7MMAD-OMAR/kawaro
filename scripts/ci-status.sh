#!/usr/bin/env bash
# Shows the state of the CI workflow run for a commit, using only the gh CLI.
#   scripts/ci-status.sh [<sha>] [--wait]
# Prints `run <id> <conclusion|status>`, then `<job> <conclusion|status>` per
# job. Exit 0: run and all jobs succeeded. Exit 1: a job failed, or the run is
# still going. Exit 2: gh is missing, or there is no run for the commit.
# --wait polls every CI_STATUS_POLL_SECONDS (default 20) for at most
# CI_STATUS_MAX_WAIT_SECONDS (default 1200) and prints only the final state.
set -eu
cd "$(dirname "$0")/.."

sha=""
wait=0
for a in "$@"; do
  case "$a" in
    --wait) wait=1 ;;
    *) sha="$a" ;;
  esac
done
poll="${CI_STATUS_POLL_SECONDS:-20}"
limit="${CI_STATUS_MAX_WAIT_SECONDS:-1200}"

if ! command -v gh >/dev/null 2>&1; then
  echo "ci-status: the gh CLI is not installed" >&2
  exit 2
fi
[ -n "$sha" ] || sha="$(git rev-parse HEAD)"

state='if (.conclusion // "") == "" then .status else .conclusion end'
id="$(gh run list --workflow CI --commit "$sha" --limit 1 \
  --json databaseId,status,conclusion --jq '.[0].databaseId // empty')"
if [ -z "$id" ]; then
  echo "ci-status: no CI run for commit $sha" >&2
  exit 2
fi

start=$SECONDS
while :; do
  status="$(gh run view "$id" --json status --jq .status)"
  if [ "$wait" = 0 ] || [ "$status" = completed ] || [ $((SECONDS - start)) -ge "$limit" ]; then
    break
  fi
  echo "ci-status: run $id is $status, waiting" >&2
  sleep "$poll"
done

lines="$(gh run view "$id" --json status,conclusion,jobs --jq \
  "\"run \(.databaseId // \"$id\") \(${state})\", (.jobs[] | \"\(.name) \(${state})\")")"
printf '%s\n' "$lines"
ok=1
while IFS= read -r l; do
  [ "${l##* }" = success ] || ok=0
done <<<"$lines"
[ "$ok" = 1 ]
