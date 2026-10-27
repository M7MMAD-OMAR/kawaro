#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-078 AC5: the WinGet manifests of a release stay consistent with the
# DisplayVersion the MSI writes to the registry, and the whole set makes the
# same choice.
#
# The trap this guards: the Microsoft community repository rejects a manifest
# whose DisplayVersion range overlaps the range of an adjacent version (step
# 06, Catalog Content Verification). The day the installer's registry
# DisplayVersion stops matching the manifest PackageVersion, every manifest
# of the package needs an AppsAndFeaturesEntries.DisplayVersion, the older
# ones included. A mixed set is the failure, and it is invisible until
# Microsoft rejects the next version.
#
# The test builds a real MSI with wixl (msitools) so the check reads a real
# ProductVersion, and gives it manifests in every shape the rule cares about.
# It needs wixl and msiinfo; without them it skips rather than fails, so the
# release suite still runs on a machine that has no msitools.
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
S="$REPO_ROOT/scripts/release/check-winget-versions.sh"
cd "$REPO_ROOT" || exit 1

if ! command -v wixl >/dev/null 2>&1 || ! command -v msiinfo >/dev/null 2>&1; then
  echo "SKIP no wixl or msiinfo, the manifest consistency check needs msitools"
  exit 0
fi

# A real MSI built from the project's own installer definition, with a
# stand-in payload. The version is the one the check reads.
build_msi() { # build_msi VERSION OUT.msi
  local version="$1" out="$2" dir
  dir="$(mktemp -d -p "$TMP")"
  printf 'MZ stand-in' > "$dir/kawaro.exe"
  printf 'MZ stand-in' > "$dir/kawarod.exe"
  printf 'stand-in' > "$dir/kawaro.ico"
  for t in audio documents images pdf video; do
    printf 'name = "x"\n' > "$dir/$t.toml"
  done
  # The product definition is the project's own, with the action directory
  # and the icon removed, so the test needs no product payload.
  sed -e 's|<Directory Id="ActionsDir" Name="actions" />||' \
      -e 's|<DirectoryRef Id="ActionsDir">|<DirectoryRef Id="INSTALLDIR">|' \
      -e 's|<Icon Id="kawaro.ico" SourceFile="$(var.Ico)" />||' \
      "$REPO_ROOT/installer/windows/kawaro.wxs" > "$dir/kawaro.wxs"
  wixl -a x64 -D "Version=$version" -D "Release=$dir" -D "Actions=$dir" \
        -D "Ico=$dir/kawaro.ico" -o "$out" "$dir/kawaro.wxs" >/dev/null 2>&1
  rm -rf "$dir"
  [ -f "$out" ]
}

# A manifest with the given PackageVersion and an optional
# AppsAndFeaturesEntries.DisplayVersion. The name is part of the file name,
# so a set can hold more than one version of the package.
write_manifest() { # write_manifest DIR NAME VERSION [DISPLAYVERSION]
  local dir="$1" name="$2" version="$3" dv="${4:-}"
  mkdir -p "$dir"
  {
    echo "PackageIdentifier: Sbarah.Kawaro"
    echo "PackageVersion: $version"
    if [ -n "$dv" ]; then
      echo "AppsAndFeaturesEntries:"
      echo "- DisplayName: Kawaro"
      echo "  Publisher: Kawaro"
      echo "  DisplayVersion: $dv"
    fi
  } > "$dir/$name.installer.yaml"
}

MSI="$TMP/fix.msi"
if ! build_msi 0.5.0 "$MSI"; then
  echo "SKIP wixl could not build the fixture MSI"
  exit 0
fi

MSIDV="$(msiinfo export "$MSI" Property 2>/dev/null | tr -d '\r' \
  | awk -F'\t' '$1=="ProductVersion"{print $2}')"
check "the fixture MSI reports a DisplayVersion" test -n "$MSIDV"

# 1. Every manifest names the MSI version and none declares the field: the
#    state of the 0.4.0 submission, and the only one that is consistent.
D="$TMP/all-match"; mkdir -p "$D"
write_manifest "$D" Sbarah.Kawaro 0.5.0
check "AC5 a set where every manifest matches the MSI DisplayVersion is accepted" \
  "$S" 0.5.0 --manifests "$D" --msi "$MSI"

# 2. Every manifest declares the field: the state after the installer's
#    DisplayVersion changes, where the field is required in all of them.
D="$TMP/all-declared"; mkdir -p "$D"
write_manifest "$D" Sbarah.Kawaro 0.5.0 0.5.0
check "AC5 a set where every manifest declares AppsAndFeaturesEntries is accepted" \
  "$S" 0.5.0 --manifests "$D" --msi "$MSI"

# 3. One manifest matches and another declares the field: the mixed set that
#    makes the catalog unable to map the installed version to an available one.
D="$TMP/mixed"; mkdir -p "$D"
write_manifest "$D" Sbarah.Kawaro-0.5.0 0.5.0
write_manifest "$D" Sbarah.Kawaro-0.4.0 0.4.0 0.4.0
expect_status "AC5 a mixed set is refused" 1 \
  "$S" 0.5.0 --manifests "$D" --msi "$MSI"

# 4. A manifest whose version differs from the MSI and declares no field: the
#    single manifest that the next version would ship with, and the reason
#    the check exists.
D="$TMP/mismatch"; mkdir -p "$D"
write_manifest "$D" Sbarah.Kawaro 0.4.0
expect_status "AC5 a manifest that differs from the MSI and declares no DisplayVersion is refused" 1 \
  "$S" 0.5.0 --manifests "$D" --msi "$MSI"

# 5. The same manifest with the field declared is accepted, so the failure
#    above is about the missing field and not about the version differing.
D="$TMP/mismatch-fixed"; mkdir -p "$D"
write_manifest "$D" Sbarah.Kawaro 0.4.0 0.4.0
check "AC5 a manifest that differs from the MSI but declares its DisplayVersion is accepted" \
  "$S" 0.5.0 --manifests "$D" --msi "$MSI"

# 6. Misuse is exit 2, not a silent pass.
expect_status "AC5 a missing manifest directory is a misuse" 2 \
  "$S" 0.5.0 --manifests "$TMP/nope" --msi "$MSI"
expect_status "AC5 a missing MSI is a misuse" 2 \
  "$S" 0.5.0 --manifests "$TMP/all-match" --msi "$TMP/nope.msi"

# 7. The script exists in the release scripts, is executable and passes the
#    linter, so the guard is not a file nobody runs.
check "AC5 the check script exists" test -s "$S"
check "AC5 the check script is executable" test -x "$S"
if command -v shellcheck >/dev/null 2>&1; then
  check "AC5 the check script passes shellcheck" shellcheck "$S"
fi

finish
