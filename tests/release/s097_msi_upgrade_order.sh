#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-097: an MSI upgrade must keep the program files. wixl puts
# RemoveExistingProducts after InstallValidate; the new install then costs the
# old files as present and skips them, and the removal of the old product
# deletes them (measured on Windows 11: 0.4.0 to 0.4.1 left only `actions`).
# The installer definition schedules RemoveExistingProducts after
# InstallFinalize instead. This test reads the real InstallExecuteSequence
# table of an MSI built from the project's own .wxs. Without wixl and msidump
# it skips.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
WXS="$REPO_ROOT/installer/windows/kawaro.wxs"

check "S-097 the .wxs schedules RemoveExistingProducts after InstallFinalize" \
  grep -q '<Custom Action="RemoveExistingProducts" After="InstallFinalize">' "$WXS"

if ! command -v wixl >/dev/null 2>&1 || ! command -v msidump >/dev/null 2>&1; then
  echo "SKIP no wixl or msidump, the sequence check needs msitools"
  exit 0
fi

dir="$(mktemp -d -p "$TMP")"
printf 'MZ stand-in' > "$dir/kawaro.exe"
printf 'MZ stand-in' > "$dir/kawarod.exe"
printf 'stand-in' > "$dir/kawaro.ico"
for t in audio documents images pdf video; do printf 'name = "x"\n' > "$dir/$t.toml"; done
sed -e 's|<Directory Id="ActionsDir" Name="actions" />||' \
    -e 's|<DirectoryRef Id="ActionsDir">|<DirectoryRef Id="INSTALLDIR">|' \
    -e 's|<Icon Id="kawaro.ico" SourceFile="$(var.Ico)" />||' \
    "$WXS" > "$dir/kawaro.wxs"
wixl -a x64 -D "Version=0.5.0" -D "Release=$dir" -D "Actions=$dir" \
     -D "Ico=$dir/kawaro.ico" -o "$dir/k.msi" "$dir/kawaro.wxs" >/dev/null 2>&1
check "S-097 the MSI builds" test -f "$dir/k.msi"
mkdir -p "$dir/t"
(cd "$dir/t" && msidump -t "$dir/k.msi" >/dev/null 2>&1)
seq="$dir/t/InstallExecuteSequence.idt"
num() { awk -F'\t' -v a="$1" '$1==a {print $NF}' "$seq" | head -1; }
rem="$(num RemoveExistingProducts)"
fin="$(num InstallFinalize)"
check "S-097 both actions are in the sequence" test -n "$rem" -a -n "$fin"
check "S-097 RemoveExistingProducts runs after InstallFinalize in the built MSI" \
  test "${rem:-0}" -gt "${fin:-99999}"
