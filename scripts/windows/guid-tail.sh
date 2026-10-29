#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Prints 12 hex digits for the ComponentGuid tails of kawaro.wxs (S-100).
#   scripts/windows/guid-tail.sh VERSION FILE...
# The tail depends on the version and on the content of every file, so two
# different builds never share component ids. An MSI upgrade only installs
# the new files when the new product's components are new to the machine.
set -eu
[ "$#" -ge 2 ] || { echo "usage: guid-tail.sh VERSION FILE..." >&2; exit 2; }
version="$1"
shift
for f in "$@"; do [ -f "$f" ] || { echo "guid-tail: missing $f" >&2; exit 2; }; done
{
  printf '%s\n' "$version"
  for f in "$@"; do sha256sum < "$f"; done
} | sha256sum | cut -c1-12
