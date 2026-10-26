#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Renames files to the names GitHub stores for release assets (S-073).
#
#   scripts/release/github-names.sh DIR
#
# GitHub stores `~` in an asset name as `.`, so kawaro-0.4.0~beta.1-1.x86_64.rpm
# is published as kawaro-0.4.0.beta.1-1.x86_64.rpm. Every file directly in DIR
# whose name contains `~` is renamed in place with each `~` replaced by `.`, and
# its new name is printed, one per line. Nothing to rename prints nothing.
# Exit 0 on success, 2 on misuse or when a new name would overwrite an existing
# file or another renamed file; then nothing at all is renamed.
set -eu

[ "$#" -eq 1 ] && [ -d "$1" ] || { echo "usage: $0 DIR" >&2; exit 2; }
dir="$1"

srcs=()
dsts=()
for path in "$dir"/*~*; do
  [ -e "$path" ] || continue
  base="${path##*/}"
  new="${base//\~/.}"
  if [ -e "$dir/$new" ]; then
    echo "$0: $base would overwrite $new, nothing renamed" >&2
    exit 2
  fi
  for d in ${dsts[@]+"${dsts[@]}"}; do
    if [ "$d" = "$new" ]; then
      echo "$0: two files would become $new, nothing renamed" >&2
      exit 2
    fi
  done
  srcs+=("$base")
  dsts+=("$new")
done

i=0
while [ "$i" -lt "${#srcs[@]}" ]; do
  mv -- "$dir/${srcs[$i]}" "$dir/${dsts[$i]}"
  echo "${dsts[$i]}"
  i=$((i + 1))
done
