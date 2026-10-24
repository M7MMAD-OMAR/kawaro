#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Checks that every place that carries the release version agrees (S-055).
#
#   scripts/release/check-versions.sh VERSION --out DIR [--root DIR]
#
# VERSION is the cargo version without a leading v (0.4.0-beta.1). DIR holds the
# built files (rpm, deb, zip, msi and the manifests); --root holds Cargo.toml
# (default: the repository). Prints exactly six lines in this order, each
# `ok SOURCE: VALUE` or `FAIL SOURCE: found X, want Y`:
# Cargo.toml, RPM, DEB, MSI, names, manifest.
# Exit 0 when all six agree, 1 on any mismatch, 2 on misuse.
set -euo pipefail

usage() { echo "usage: $0 VERSION --out DIR [--root DIR]" >&2; exit 2; }

[ "$#" -ge 1 ] || usage
VERSION="$1"; shift
OUT=""; ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --out) [ "$#" -ge 2 ] || usage; OUT="$2"; shift 2 ;;
    --root) [ "$#" -ge 2 ] || usage; ROOT="$2"; shift 2 ;;
    *) usage ;;
  esac
done
[ -n "$OUT" ] || usage
SEMVER='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'
[[ "$VERSION" =~ $SEMVER ]] || { echo "check-versions: not a version: $VERSION" >&2; exit 2; }
[ -d "$OUT" ] || { echo "check-versions: not a directory: $OUT" >&2; exit 2; }

PKG="${VERSION//-/\~}"   # rpm and deb: a tilde sorts below the final release
MSIV="${VERSION%%[-+]*}" # MSI versions are numeric
RC=0

report() { # report SOURCE FOUND WANT
  if [ "$2" = "$3" ]; then
    echo "ok $1: $3"
  else
    echo "FAIL $1: found ${2:-nothing}, want $3"
    RC=1
  fi
}
first() { local f; for f in "$@"; do [ -f "$f" ] && { printf '%s' "$f"; return; }; done; return 0; }

CARGO="$(awk '/^\[workspace.package\]/{p=1;next} /^\[/{p=0} p && /^version *=/{gsub(/"/,"",$3); print $3; exit}' "$ROOT/Cargo.toml" 2>/dev/null || true)"
report Cargo.toml "$CARGO" "$VERSION"

RPMF="$(first "$OUT"/*.rpm)"
RPMV=""
[ -z "$RPMF" ] || RPMV="$(rpm -qp --qf '%{VERSION}' "$RPMF" 2>/dev/null || true)"
report RPM "$RPMV" "$PKG"

deb_version() { # deb_version FILE: the Version field, with or without dpkg-deb
  local f="$1"
  if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb -f "$f" Version 2>/dev/null || true
    return
  fi
  local m flag
  for m in control.tar.gz:z control.tar.xz:J control.tar.zst:--zstd; do
    flag="${m#*:}"; m="${m%:*}"
    if ar t "$f" 2>/dev/null | grep -qx "$m"; then
      [ "$flag" = "--zstd" ] || flag="-$flag"
      ar p "$f" "$m" | tar "$flag" -xO ./control 2>/dev/null | sed -n 's/^Version: //p' | head -1 || true
      return
    fi
  done
}
DEBF="$(first "$OUT"/*.deb)"
DEBV=""
[ -z "$DEBF" ] || DEBV="$(deb_version "$DEBF")"
report DEB "$DEBV" "$PKG"

MSIF="$(first "$OUT"/*.msi)"
MSIPV=""
[ -z "$MSIF" ] || MSIPV="$(msiinfo export "$MSIF" Property 2>/dev/null | tr -d '\r' | awk -F'\t' '$1=="ProductVersion"{print $2}' || true)"
report MSI "$MSIPV" "$MSIV"

# names: the zip file, the folder inside it and the MSI file carry the version
WANT_ZIP="sbarhive-$VERSION-windows-x64.zip"
WANT_MSI="sbarhive-$VERSION-windows-x64.msi"
ZIPF="$(first "$OUT"/*.zip)"
FOUND_NAMES=""
if [ -n "$ZIPF" ] && [ -n "$MSIF" ]; then
  top="$(unzip -Z1 "$ZIPF" 2>/dev/null | head -1 | cut -d/ -f1 || true)"
  if [ "$top" = "${WANT_ZIP%.zip}" ]; then
    FOUND_NAMES="$(basename "$ZIPF") $(basename "$MSIF")"
  else
    FOUND_NAMES="$(basename "$ZIPF") (folder ${top:-none}) $(basename "$MSIF")"
  fi
elif [ -n "$ZIPF" ]; then FOUND_NAMES="$(basename "$ZIPF") (no msi)"
elif [ -n "$MSIF" ]; then FOUND_NAMES="(no zip) $(basename "$MSIF")"
fi
if [ "$FOUND_NAMES" = "$WANT_ZIP $WANT_MSI" ]; then
  echo "ok names: $WANT_ZIP $WANT_MSI"
else
  echo "FAIL names: found ${FOUND_NAMES:-nothing}, want $WANT_ZIP $WANT_MSI"
  RC=1
fi

# manifest: every manifest in the directory names this version
MANV=""
for m in "$OUT"/*.json; do
  [ -f "$m" ] || continue
  v="$(jq -r '.version // empty' "$m" 2>/dev/null || true)"
  if [ -z "$MANV" ]; then MANV="$v"; elif [ "$v" != "$VERSION" ]; then MANV="$v"; fi
done
report manifest "$MANV" "$VERSION"

exit "$RC"
