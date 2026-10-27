#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Checks that a per-user WinGet manifest matches a per-user MSI (S-078).
#
#   scripts/release/check-winget-scope.sh --manifests DIR --msi FILE
#
# The manifest declares Scope: user, so winget reads the Add and Remove
# Programs entry under HKEY_CURRENT_USER
# (src/AppInstallerRepositoryCore/Microsoft/ARPHelper.cpp). Windows Installer
# only registers there when the package is per-user for that user. A package
# without the ALLUSERS property is not per-user by construction: msiexec
# decides, and an elevated install registers under HKEY_LOCAL_MACHINE
# instead, where winget will not find it.
#
# wixl (msitools) accepts InstallScope="perUser" and drops it, so the property
# has to be written explicitly in the .wxs:
#
#   <Property Id="ALLUSERS" Value="2" />
#   <Property Id="MSIINSTALLPERUSER" Value="1" />
#
# This script answers one question: does the MSI carry those properties, so
# that Scope: user in the manifest is true for every user and not only for
# the non-elevated one the Microsoft pipeline tests?
#
# Prints one line per manifest, then a summary. Exit 0 when every manifest
# that says Scope: user has a per-user MSI, 1 when it does not, 2 on misuse.
set -euo pipefail

usage() { echo "usage: $0 --manifests DIR --msi FILE" >&2; exit 2; }

[ "$#" -ge 1 ] || usage
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
[ -d "$MANIFESTS" ] || { echo "check-winget-scope: no manifest directory: $MANIFESTS" >&2; exit 2; }
[ -f "$MSIF" ] || { echo "check-winget-scope: no MSI: $MSIF" >&2; exit 2; }

# The properties the MSI carries. ALLUSERS=2 means per-user, and
# MSIINSTALLPERUSER=1 says the same for the current user.
props() {
  msiinfo export "$1" Property 2>/dev/null | tr -d '\r' | awk -F'\t' -v k="$2" '$1==k{print $2}'
}

ALLUSERS="$(props "$MSIF" ALLUSERS || true)"
PERUSER="$(props "$MSIF" MSIINSTALLPERUSER || true)"

echo "MSI ALLUSERS: ${ALLUSERS:-<absent>}"
echo "MSI MSIINSTALLPERUSER: ${PERUSER:-<absent>}"

# A per-user MSI must say so with both properties. Either one alone leaves
# msiexec free to register the product per-machine.
per_user_msi=no
if [ "$ALLUSERS" = "2" ] && [ "$PERUSER" = "1" ]; then
  per_user_msi=yes
fi

total=0; bad=""
for f in "$MANIFESTS"/*.installer.yaml; do
  [ -f "$f" ] || continue
  total=$((total + 1))
  scope="$(awk '/^Scope:/{gsub(/"/,"",$2); print $2; exit}' "$f")"
  if [ "$scope" != "user" ]; then
    echo "ok $(basename "$f"): Scope ${scope:-<unset>} is not per-user, the MSI scope does not apply to it"
    continue
  fi
  if [ "$per_user_msi" = yes ]; then
    echo "ok $(basename "$f"): Scope user and the MSI is per-user (ALLUSERS=2, MSIINSTALLPERUSER=1)"
  else
    bad="$bad $(basename "$f")"
    echo "FAIL $(basename "$f"): Scope user but the MSI carries no ALLUSERS=2 and MSIINSTALLPERUSER=1, so an elevated install registers under HKEY_LOCAL_MACHINE where winget does not look"
  fi
done

if [ "$total" -eq 0 ]; then
  echo "check-winget-scope: no installer manifest in $MANIFESTS" >&2
  exit 2
fi

if [ -n "$bad" ]; then
  badcount="$(printf '%s' "$bad" | wc -w)"
  echo "FAIL $total manifest(s) checked, $((total - badcount)) consistent"
  exit 1
fi
echo "ok $total manifest(s) checked, every Scope: user manifest has a per-user MSI"
