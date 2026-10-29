#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-078 AC7: a manifest that says Scope: user has an MSI that is per-user by
# construction, so the Add and Remove Programs entry lands where winget looks
# for every user and not only for the non-elevated one.
#
# The trap this guards: wixl accepts InstallScope="perUser" and drops it, so
# the built MSI carries no ALLUSERS property. msiexec then decides the scope
# itself, and an elevated install registers the product under
# HKEY_LOCAL_MACHINE. winget reads HKEY_CURRENT_USER for ScopeEnum::User
# (src/AppInstallerRepositoryCore/Microsoft/ARPHelper.cpp), so such an install
# is invisible to winget list and to the uninstall path. The Microsoft
# pipeline does not catch it, because step 08 installs as a standard
# (non-elevated) user, where the install is per-user anyway.
#
# The test builds real MSIs with wixl (msitools) so the check reads real
# tables, in both shapes: the project's own .wxs and the same .wxs with the
# explicit ALLUSERS=2 and MSIINSTALLPERUSER=1 pair. It needs wixl and
# msiinfo; without them it skips rather than fails, so the release suite
# still runs on a machine that has no msitools.
#
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
S="$REPO_ROOT/scripts/release/check-winget-scope.sh"
cd "$REPO_ROOT" || exit 1

if ! command -v wixl >/dev/null 2>&1 || ! command -v msiinfo >/dev/null 2>&1; then
  echo "SKIP no wixl or msiinfo, the scope check needs msitools"
  exit 0
fi

# A real MSI built from the project's own installer definition, with a
# stand-in payload. When PERUSER is no the explicit per-user properties are
# removed, which is the state of the 0.4.0 submission: wixl dropped
# InstallScope="perUser", so the built MSI carried no ALLUSERS.
build_msi() { # build_msi OUT.msi [peruser]
  local out="$1" peruser="${2:-no}" dir
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
  if [ "$peruser" = no ]; then
    sed -i -e '/<Property Id="ALLUSERS"/d' -e '/<Property Id="MSIINSTALLPERUSER"/d' \
      "$dir/kawaro.wxs"
  fi
  wixl -a x64 -D "Version=0.5.0" -D "GuidTail=000000000000" -D "Release=$dir" -D "Actions=$dir" \
        -D "Ico=$dir/kawaro.ico" -o "$out" "$dir/kawaro.wxs" >/dev/null 2>&1
  rm -rf "$dir"
  [ -f "$out" ]
}

# A manifest with the given scope. The name is part of the file name, so a
# directory can hold more than one manifest of the package.
write_manifest() { # write_manifest DIR NAME SCOPE
  local dir="$1" name="$2" scope="$3"
  mkdir -p "$dir"
  {
    echo "PackageIdentifier: Sbarah.Kawaro"
    echo "PackageVersion: 0.5.0"
    echo "InstallerType: msi"
    if [ -n "$scope" ]; then
      echo "Scope: $scope"
    fi
  } > "$dir/$name.installer.yaml"
}

# The MSI built from the project's own .wxs, which is the state of the 0.4.0
# submission: wixl dropped InstallScope="perUser", so no ALLUSERS.
PLAIN="$TMP/plain.msi"
if ! build_msi "$PLAIN"; then
  echo "SKIP wixl could not build the fixture MSI"
  exit 0
fi

# The same MSI with the explicit per-user properties, which is the state the
# check asks the .wxs to reach.
PERUSER="$TMP/peruser.msi"
if ! build_msi "$PERUSER" yes; then
  echo "SKIP wixl could not build the per-user fixture MSI"
  exit 0
fi

# The fixtures must differ in exactly the way the check looks at, otherwise
# the test proves nothing.
au_plain="$(msiinfo export "$PLAIN" Property 2>/dev/null | tr -d '\r' | awk -F'\t' '$1=="ALLUSERS"{print $2}')"
au_peruser="$(msiinfo export "$PERUSER" Property 2>/dev/null | tr -d '\r' | awk -F'\t' '$1=="ALLUSERS"{print $2}')"
check "AC7 the fixture without the property carries no ALLUSERS" test -z "$au_plain"
check "AC7 the fixture with the property carries ALLUSERS=2" test "$au_peruser" = "2"

# 1. A Scope: user manifest with a per-user MSI is accepted.
D="$TMP/per-user-ok"; mkdir -p "$D"
write_manifest "$D" Sbarah.Kawaro user
check "AC7 a Scope: user manifest with a per-user MSI is accepted" \
  "$S" --manifests "$D" --msi "$PERUSER"

# 2. A Scope: user manifest with an MSI that is not per-user by construction
#    is refused: that is the state of the 0.4.0 MSI, and the reason the
#    check exists.
expect_status "AC7 a Scope: user manifest with an MSI that carries no ALLUSERS is refused" 1 \
  "$S" --manifests "$D" --msi "$PLAIN"

# 3. A manifest that is not per-user is not the check's business, so the
#    same plain MSI is accepted for it.
D="$TMP/machine-scope"; mkdir -p "$D"
write_manifest "$D" Sbarah.Kawaro machine
check "AC7 a Scope: machine manifest is accepted with any MSI" \
  "$S" --manifests "$D" --msi "$PLAIN"

# 4. A manifest with no scope at all is also not the check's business.
D="$TMP/no-scope"; mkdir -p "$D"
write_manifest "$D" Sbarah.Kawaro ""
check "AC7 a manifest with no scope is accepted with any MSI" \
  "$S" --manifests "$D" --msi "$PLAIN"

# 5. Misuse is exit 2, not a silent pass.
expect_status "AC7 a missing manifest directory is a misuse" 2 \
  "$S" --manifests "$TMP/nope" --msi "$PERUSER"
expect_status "AC7 a missing MSI is a misuse" 2 \
  "$S" --manifests "$D" --msi "$TMP/nope.msi"

# 6. The script exists in the release scripts, is executable and passes the
#    linter, so the guard is not a file nobody runs.
check "AC7 the check script exists" test -s "$S"
check "AC7 the check script is executable" test -x "$S"
if command -v shellcheck >/dev/null 2>&1; then
  check "AC7 the check script passes shellcheck" shellcheck "$S"
fi

finish
