#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Runs every release channel script in isolation and writes the channel status
# table (S-103a).
#
#   scripts/channels/run-all.sh vX.Y.Z [--dry-run] [--only NAME]
#       [--channels-dir DIR] [--status-file FILE] [--results-dir DIR]
#       [--date YYYY-MM-DD] [--render-only]
#
# Channel contract (every scripts/channels/NAME.sh except run-all.sh and lib.sh,
# found by glob, so a new channel only has to add its file):
#   - runs from the repository root, the tag is $1, "--dry-run" may follow
#   - secrets come from environment variables only and are never printed
#   - --dry-run prints what it would do and touches nothing external
#   - exit 0 success, exit 10 clean SKIP (one line "SKIP NAME: VAR not
#     configured"), any other code is a failure
#
# One failing channel never blocks the others. Exit 0 when none failed, 1 when
# one did, 2 on misuse. A prerelease tag (vX.Y.Z-suffix) skips every channel
# without running it.
#
# The table is written between the marker comments of --status-file; without
# --status-file a dry run prints it and a real run updates
# docs/distribution-status.md. --results-dir keeps one NAME.result per channel
# so a workflow can run channels in separate jobs and --render-only builds the
# table from the collected results.
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
channels_dir="$ROOT/scripts/channels"
status_file=""
results_dir=""
only=""
dry=0
render_only=0
date_str="$(date -u +%F)"
tag=""

usage() { echo "usage: $0 vX.Y.Z [--dry-run] [--only NAME] [--channels-dir DIR] [--status-file FILE] [--results-dir DIR] [--date D] [--render-only]" >&2; exit 2; }

while [ "$#" -gt 0 ]; do
  case "$1" in
    --dry-run) dry=1; shift ;;
    --render-only) render_only=1; shift ;;
    --only) [ "$#" -ge 2 ] || usage; only="$2"; shift 2 ;;
    --channels-dir) [ "$#" -ge 2 ] || usage; channels_dir="$2"; shift 2 ;;
    --status-file) [ "$#" -ge 2 ] || usage; status_file="$2"; shift 2 ;;
    --results-dir) [ "$#" -ge 2 ] || usage; results_dir="$2"; shift 2 ;;
    --date) [ "$#" -ge 2 ] || usage; date_str="$2"; shift 2 ;;
    -*) usage ;;
    *) [ -z "$tag" ] || usage; tag="$1"; shift ;;
  esac
done
[ -n "$tag" ] || usage
if ! [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]]; then
  echo "run-all: not a release tag: $tag" >&2
  exit 2
fi
version="${tag#v}"

own_tmp=""
if [ -z "$results_dir" ]; then
  own_tmp="$(mktemp -d)"
  results_dir="$own_tmp"
fi
# shellcheck disable=SC2329  # runs from the EXIT trap
cleanup() { [ -z "$own_tmp" ] || rm -rf "$own_tmp"; }
trap cleanup EXIT
mkdir -p "$results_dir"

record() { # record NAME STATE
  printf '%s\t%s\t%s\n' "$2" "$version" "$date_str" > "$results_dir/$1.result"
  echo "RESULT $1 $2"
}

run_channel() { # run_channel NAME
  local name="$1" script="$channels_dir/$1.sh" args out rc
  if [ ! -f "$script" ]; then
    echo "SKIP $name: no channel script"
    record "$name" skipped
    return 0
  fi
  case "$tag" in
    *-*) echo "SKIP $name: prerelease tag"; record "$name" skipped; return 0 ;;
  esac
  args=("$tag")
  [ "$dry" -eq 0 ] || args+=(--dry-run)
  out="$(cd "$ROOT" && bash "$script" "${args[@]}" 2>&1)"
  rc=$?
  [ -z "$out" ] || printf '%s\n' "$out"
  case "$rc" in
    0) if [ "$dry" -eq 1 ]; then record "$name" dry-run; else record "$name" ok; fi ;;
    10) record "$name" skipped ;;
    *) echo "FAIL $name: exit $rc"; record "$name" failed; return 1 ;;
  esac
}

render() {
  echo "<!-- channels:start -->"
  echo "| Channel | State | Version | Checked |"
  echo "| --- | --- | --- | --- |"
  local f name state ver day
  for f in "$results_dir"/*.result; do
    [ -f "$f" ] || continue
    name="$(basename "$f" .result)"
    IFS=$'\t' read -r state ver day < "$f"
    echo "| $name | $state | $ver | $day |"
  done
  echo "<!-- channels:end -->"
}

write_table() { # write_table FILE
  local file="$1" tmp
  if ! grep -qx '<!-- channels:start -->' "$file" 2>/dev/null || ! grep -qx '<!-- channels:end -->' "$file"; then
    echo "run-all: markers missing in $file" >&2
    return 2
  fi
  tmp="$(mktemp "$file.XXXXXX")"
  render > "$tmp.table"
  awk -v tbl="$tmp.table" '
    $0 == "<!-- channels:start -->" { while ((getline line < tbl) > 0) print line; skip = 1; next }
    $0 == "<!-- channels:end -->" { skip = 0; next }
    !skip { print }
  ' "$file" > "$tmp"
  mv "$tmp" "$file"
  rm -f "$tmp.table"
}

failed=0
if [ "$render_only" -eq 0 ]; then
  names=()
  if [ -n "$only" ]; then
    names=("$only")
  else
    for s in "$channels_dir"/*.sh; do
      [ -f "$s" ] || continue
      n="$(basename "$s" .sh)"
      case "$n" in run-all | lib) continue ;; esac
      names+=("$n")
    done
  fi
  for n in "${names[@]}"; do
    run_channel "$n" || failed=1
  done
fi

if [ -n "$status_file" ]; then
  write_table "$status_file" || exit 2
elif [ "$dry" -eq 1 ] || [ "$render_only" -eq 1 ] || [ -n "$only" ]; then
  render
else
  write_table "$ROOT/docs/distribution-status.md" || exit 2
fi
exit "$failed"
