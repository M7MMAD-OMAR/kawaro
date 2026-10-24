#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Writes the schema 1 update manifest of ADR 0006.
#
#   scripts/make-update-manifest.sh --dist DIR CHANNEL VERSION ASSETS_JSON OUT_JSON
#
# CHANNEL is stable or beta; VERSION has no leading v (a stable manifest for
# a prerelease version is refused). ASSETS_JSON is the output of
# `gh release view --json assets`: name, url and size are copied from it, the
# SHA-256 is computed from the file of the same name in DIR. Only .msi, .rpm
# and .deb assets become entries, in input order.
#
# Every asset url must start with https://github.com/M7MMAD-OMAR/sbarhive/releases/download/
# (the client enforces the same prefix, ADR 0006), otherwise exit 2.
#
# Exit codes: 0 ok; 2 bad usage or bad input; 3 a package has no local file in
# DIR; 4 no package asset. On a non zero exit OUT_JSON is not created.
set -euo pipefail

die() { echo "make-update-manifest: $2" >&2; exit "$1"; }

if [ "$#" -ne 6 ] || [ "$1" != "--dist" ]; then
  die 2 "usage: $0 --dist DIR CHANNEL VERSION ASSETS_JSON OUT_JSON"
fi
DIST="$2"; CHANNEL="$3"; VERSION="$4"; ASSETS="$5"; OUT="$6"

[ -d "$DIST" ] || die 2 "not a directory: $DIST"
case "$CHANNEL" in stable | beta) ;; *) die 2 "channel must be stable or beta" ;; esac
SEMVER='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'
[[ "$VERSION" =~ $SEMVER ]] || die 2 "version must look like 1.2.3 or 1.2.3-beta.1, without a leading v"
if [ "$CHANNEL" = stable ] && [[ "$VERSION" == *-* ]]; then
  die 2 "the stable channel takes no prerelease version"
fi
[ -f "$ASSETS" ] || die 2 "not a file: $ASSETS"
jq -e '(.assets | type) == "array"' "$ASSETS" >/dev/null 2>&1 || die 2 "$ASSETS has no assets array"

# One tab separated row per package asset, validated by jq.
PREFIX="https://github.com/M7MMAD-OMAR/sbarhive/releases/download/"
ROWS="$(jq -r --arg prefix "$PREFIX" '
  .assets[] | select(.name | type == "string" and test("\\.(msi|rpm|deb)$"))
  | if (.size | type) == "number" and .size > 0 and .size == (.size | floor)
         and (.url | type) == "string" and (.url | startswith($prefix))
         and (.name | test("[\t\n]") | not)
    then [.name, .url, (.size | tostring)] | @tsv
    else error("bad asset " + .name) end' "$ASSETS" 2>/dev/null)" || die 2 "invalid package asset in $ASSETS"
[ -n "$ROWS" ] || die 4 "no .msi, .rpm or .deb asset in $ASSETS"

TMPF="$(mktemp)"
trap 'rm -f "$TMPF"' EXIT
ENTRIES=()
while IFS=$'\t' read -r name url size; do
  [ -f "$DIST/$name" ] || die 3 "no local file for $name in $DIST"
  sha="$(sha256sum "$DIST/$name" | cut -d' ' -f1)"
  case "$name" in
    *.msi) kind=msi os=windows ;;
    *.rpm) kind=rpm os=linux ;;
    *) kind=deb os=linux ;;
  esac
  ENTRIES+=("$(jq -n --arg kind "$kind" --arg os "$os" --arg name "$name" --arg url "$url" \
    --argjson size "$size" --arg sha "$sha" \
    '{kind: $kind, os: $os, arch: "x86_64", name: $name, url: $url, size: $size, sha256: $sha}')")
done <<<"$ROWS"

printf '%s\n' "${ENTRIES[@]}" | jq -s \
  --arg channel "$CHANNEL" --arg version "$VERSION" \
  --arg published "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg notes "https://github.com/M7MMAD-OMAR/sbarhive/releases/tag/v$VERSION" \
  '{schema: 1, channel: $channel, version: $version, published: $published,
    notes_url: $notes, assets: .}' >"$TMPF"
mv "$TMPF" "$OUT"
