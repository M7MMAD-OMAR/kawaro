#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# WinGet channel (S-103b, replaces S-078e): opens the manifest update pull
# request for the release MSI from the maintainer's fork of winget-pkgs.
#
#   scripts/channels/winget.sh vX.Y.Z [--dry-run] [--out DIR]
#
# Tool: Komac (GPL-3.0, runs on Linux, recorded in docs/engines.md). It reads
# the token from the GITHUB_TOKEN environment variable, which this script sets
# from WINGET_CREATE_GITHUB_TOKEN for the one child process only. The token is
# never put on a command line and never printed.
#
# Exit 0 success, 10 clean SKIP (token not configured), anything else failure.
# --dry-run builds a preview of the four manifests (version, installer, en-US,
# ar) into --out, validates them, prints what would happen and touches nothing
# external. Komac builds the real set itself from the previous version.
#
# GATE: do not enable this channel before the first WinGet submission is merged
# (microsoft/winget-pkgs#462805). `komac update` needs the package to exist in
# the community repository; until then the token secret stays unset and this
# script is a SKIP. Owner step: create the fork and a classic token with only
# public_repo, store it as WINGET_CREATE_GITHUB_TOKEN in the winget-publishing
# environment.
#
# Environment: WINGET_CREATE_GITHUB_TOKEN (secret), CANONICAL_REPO (OWNER/NAME,
# default OWNER/kawaro), KAWARO_DIST (directory holding SHA256SUMS for the
# dry run, default dist).
set -u

PKG="Sbarah.Kawaro"
SECRET="WINGET_CREATE_GITHUB_TOKEN"
dry=0
out=""
tag=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --dry-run) dry=1; shift ;;
    --out) [ "$#" -ge 2 ] || { echo "usage: $0 vX.Y.Z [--dry-run] [--out DIR]" >&2; exit 2; }; out="$2"; shift 2 ;;
    -*) echo "usage: $0 vX.Y.Z [--dry-run] [--out DIR]" >&2; exit 2 ;;
    *) tag="$1"; shift ;;
  esac
done
if ! [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "winget: needs a stable tag vX.Y.Z, got: ${tag:-none}" >&2
  exit 2
fi
version="${tag#v}"
repo="${CANONICAL_REPO:-OWNER/kawaro}"
msi="kawaro-${version}-windows-x64.msi"
url="https://github.com/${repo}/releases/download/${tag}/${msi}"

# Preview manifests, built from SHA256SUMS only.
build_manifests() { # build_manifests DIR
  local dir="$1" sums sha
  sums="${KAWARO_DIST:-dist}/SHA256SUMS"
  [ -f "$sums" ] || { echo "winget: no $sums" >&2; return 1; }
  sha="$(awk -v f="$msi" '$2 == f { print $1 }' "$sums")"
  if ! [[ "$sha" =~ ^[0-9a-fA-F]{64}$ ]]; then
    echo "winget: no valid SHA-256 for $msi in $sums" >&2
    return 1
  fi
  sha="${sha^^}"
  mkdir -p "$dir" || return 1
  local head="# Preview from scripts/channels/winget.sh (S-103b); Komac builds the real set."
  cat > "$dir/$PKG.yaml" <<M
$head
PackageIdentifier: $PKG
PackageVersion: $version
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.12.0
M
  cat > "$dir/$PKG.installer.yaml" <<M
$head
PackageIdentifier: $PKG
PackageVersion: $version
InstallerType: msi
Scope: user
Installers:
  - Architecture: x64
    InstallerUrl: $url
    InstallerSha256: $sha
ManifestType: installer
ManifestVersion: 1.12.0
M
  cat > "$dir/$PKG.locale.en-US.yaml" <<M
$head
PackageIdentifier: $PKG
PackageVersion: $version
PackageLocale: en-US
Publisher: Kawaro
PackageName: Kawaro
License: GPL-3.0-or-later
ShortDescription: A local, open source file converter with a honeycomb UI.
ManifestType: defaultLocale
ManifestVersion: 1.12.0
M
  cat > "$dir/$PKG.locale.ar.yaml" <<M
$head
PackageIdentifier: $PKG
PackageVersion: $version
PackageLocale: ar
Publisher: Kawaro
PackageName: كوارو
License: GPL-3.0-or-later
ShortDescription: محوّل ملفات محلي ومفتوح المصدر بواجهة خلية نحل.
ManifestType: locale
ManifestVersion: 1.12.0
M
}

validate_manifests() { # validate_manifests DIR
  local dir="$1" f key
  for f in "$PKG.yaml" "$PKG.installer.yaml" "$PKG.locale.en-US.yaml" "$PKG.locale.ar.yaml"; do
    [ -s "$dir/$f" ] || { echo "winget: missing manifest $f" >&2; return 1; }
    grep -qx "PackageIdentifier: $PKG" "$dir/$f" || { echo "winget: wrong identifier in $f" >&2; return 1; }
    grep -qx "PackageVersion: $version" "$dir/$f" || { echo "winget: wrong version in $f" >&2; return 1; }
    grep -qx "ManifestVersion: 1.12.0" "$dir/$f" || { echo "winget: wrong schema in $f" >&2; return 1; }
  done
  for key in "InstallerType: msi" "Scope: user" "    InstallerUrl: $url"; do
    grep -qxF "$key" "$dir/$PKG.installer.yaml" || { echo "winget: installer lacks '$key'" >&2; return 1; }
  done
  grep -qE '^    InstallerSha256: [0-9A-F]{64}$' "$dir/$PKG.installer.yaml" || { echo "winget: bad installer hash" >&2; return 1; }
}

if [ "$dry" -eq 1 ]; then
  if [ -z "$out" ]; then
    out="$(mktemp -d)"
    trap 'rm -rf "$out"' EXIT
  fi
  build_manifests "$out" || exit 1
  validate_manifests "$out" || exit 1
  echo "winget: preview manifests built and validated in $out"
  if [ -z "${!SECRET:-}" ]; then
    echo "SKIP winget: $SECRET not configured"
    exit 10
  fi
  echo "DRYRUN winget: would open a manifest update PR for $version ($PKG, $msi) from the fork"
  exit 0
fi

if [ -z "${!SECRET:-}" ]; then
  echo "SKIP winget: $SECRET not configured"
  exit 10
fi
if ! command -v komac >/dev/null 2>&1; then
  echo "winget: komac is not installed" >&2
  exit 1
fi
GITHUB_TOKEN="${!SECRET}" komac update "$PKG" --version "$version" --urls "$url" --submit
rc=$?
[ "$rc" -eq 0 ] || { echo "winget: komac failed with exit $rc" >&2; exit 1; }
echo "winget: update PR for $version submitted (not published until Microsoft merges it)"
