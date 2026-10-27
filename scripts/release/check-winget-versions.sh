#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Checks that the WinGet manifests for a release stay consistent with the
# DisplayVersion the MSI writes to the registry (S-078).
#
#   scripts/release/check-winget-versions.sh VERSION --manifests DIR --msi FILE
#
# The Microsoft community repository rejects a manifest whose DisplayVersion
# range overlaps the range of an adjacent version, which is what step 06
# (Catalog Content Verification) checks. The trap for us: the moment the
# installer's registry DisplayVersion stops matching the manifest
# PackageVersion, every manifest of the package needs an
# AppsAndFeaturesEntries.DisplayVersion, including the older ones. A new
# version without it is rejected, and the older manifest is then also wrong.
#
# This script answers one question for a given release: does every manifest
# in DIR name the same version the MSI writes, or does every one of them
# declare its own DisplayVersion? A partial answer is the failure it exists
# to catch.
#
# Prints one line per manifest, then a summary. Exit 0 when the set is
# consistent, 1 when it is not, 2 on misuse.
set -euo pipefail

usage() { echo "usage: $0 VERSION --manifests DIR --msi FILE" >&2; exit 2; }

[ "$#" -ge 1 ] || usage
REQUESTED="$1"; shift
MANIFESTS=""; MSIF=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifests) [ "$#" -ge 2 ] || usage; MANIFESTS="$2"; shift 2 ;;
    --msi) [ "$#" -ge 2 ] || usage; MSIF="$2"; shift 2 ;;
    *) usage ;;
  esac
done
[ -n "$MANIFESTS" ] || usage
[ -n "$MSIF" ] || usage
[ -d "$MANIFESTS" ] || { echo "check-winget-versions: no manifest directory: $MANIFESTS" >&2; exit 2; }
[ -f "$MSIF" ] || { echo "check-winget-versions: no MSI: $MSIF" >&2; exit 2; }

# The DisplayVersion the installer writes. For our MSI that is the MSI
# ProductVersion, the numeric part of the release version.
MSIDV="$(msiinfo export "$MSIF" Property 2>/dev/null | tr -d '\r' \
  | awk -F'\t' '$1=="ProductVersion"{print $2}' || true)"
[ -n "$MSIDV" ] || { echo "check-winget-versions: the MSI has no ProductVersion" >&2; exit 2; }

echo "MSI DisplayVersion: $MSIDV"
echo "requested version: $REQUESTED"

# Every installer manifest in the directory, with its PackageVersion and
# whether it declares its own DisplayVersion.
total=0; declared=0; matching=0; bad=""
for f in "$MANIFESTS"/*.installer.yaml; do
  [ -f "$f" ] || continue
  total=$((total + 1))
  pkg="$(awk '/^PackageVersion:/{gsub(/"/,"",$2); print $2; exit}' "$f")"
  if grep -q '^AppsAndFeaturesEntries:' "$f"; then
    declared=$((declared + 1))
    echo "ok $(basename "$f"): PackageVersion $pkg, declares its own DisplayVersion"
  elif [ "$pkg" = "$MSIDV" ]; then
    matching=$((matching + 1))
    echo "ok $(basename "$f"): PackageVersion $pkg matches the MSI DisplayVersion"
  else
    bad="$bad $(basename "$f")"
    echo "FAIL $(basename "$f"): PackageVersion $pkg differs from the MSI DisplayVersion $MSIDV and no AppsAndFeaturesEntries.DisplayVersion is declared"
  fi
done

if [ "$total" -eq 0 ]; then
  echo "check-winget-versions: no installer manifest in $MANIFESTS" >&2
  exit 2
fi

# The failure mode is a mixed set: some manifests carry the field and others
# do not. When the installer's DisplayVersion changes, every manifest of the
# package needs the field, the old ones included, or the catalog cannot map
# the installed version to an available one.
if [ "$declared" -gt 0 ] && [ "$matching" -gt 0 ]; then
  echo "FAIL mixed set: $declared manifest(s) declare AppsAndFeaturesEntries and $matching do not; every manifest of the package needs the same choice"
  exit 1
fi
if [ -n "$bad" ]; then
  echo "FAIL $total manifest(s) checked, $((declared + matching)) consistent"
  exit 1
fi
echo "ok $total manifest(s) checked, all consistent with the MSI DisplayVersion $MSIDV"
