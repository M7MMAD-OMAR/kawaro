#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-100: an MSI upgrade must install the new files. The component ids of two
# builds must differ (else the new product costs them as already installed
# by the old one and StartDaemon fails with 2753), and the old product must be
# removed before the new files are written. Reads the real tables of MSIs
# built from the project's own .wxs. Without wixl and msidump the table
# checks skip.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
WXS="$REPO_ROOT/installer/windows/kawaro.wxs"
TAIL="$REPO_ROOT/scripts/windows/guid-tail.sh"
MAKE="$REPO_ROOT/scripts/windows/make-msi.sh"
FIXUP="$REPO_ROOT/scripts/windows/msi-fixup.sh"

check "S-100 guid-tail.sh exists and is executable" test -x "$TAIL"
check "S-100 make-msi.sh passes GuidTail to wixl" grep -q 'GuidTail=' "$MAKE"
check "S-100 make-msi.sh uses guid-tail.sh" grep -q 'guid-tail.sh' "$MAKE"
check "S-100 make-msi.sh runs msi-fixup.sh on the built MSI" grep -q 'msi-fixup.sh' "$MAKE"
check "S-100 the .wxs no longer forces RemoveExistingProducts after InstallFinalize" \
  bash -c '! grep -q "RemoveExistingProducts\" After=\"InstallFinalize\"" "$1"' _ "$WXS"
check "S-100 every ComponentGuid ends in the build tail" \
  test "$(grep -c 'Guid="[0-9a-f-]*\$(var.GuidTail)"' "$WXS")" -eq 4

d="$(mktemp -d -p "$TMP")"
printf 'one' > "$d/a"; printf 'two' > "$d/b"
if [ -x "$TAIL" ]; then
  t1="$("$TAIL" 0.5.0 "$d/a" "$d/b")"
  t2="$("$TAIL" 0.5.0 "$d/a" "$d/b")"
  t3="$("$TAIL" 0.5.1 "$d/a" "$d/b")"
  printf 'three' > "$d/b"
  t4="$("$TAIL" 0.5.0 "$d/a" "$d/b")"
  check "S-100 the tail is 12 hex digits" bash -c '[[ "$1" =~ ^[0-9a-f]{12}$ ]]' _ "$t1"
  check "S-100 the tail is deterministic" test "$t1" = "$t2"
  check "S-100 the tail changes with the version" test "$t1" != "$t3"
  check "S-100 the tail changes with a file" test "$t1" != "$t4"
fi

if ! command -v wixl >/dev/null 2>&1 || ! command -v msidump >/dev/null 2>&1; then
  echo "SKIP no wixl or msidump, the table checks need msitools"
  
finish
  exit $?
fi

build() { # build NAME VERSION TAIL
  local dir="$d/$1"
  mkdir -p "$dir/t"
  printf 'MZ stand-in %s' "$2" > "$dir/kawaro.exe"
  printf 'MZ stand-in %s' "$2" > "$dir/kawarod.exe"
  printf 'stand-in' > "$dir/kawaro.ico"
  for t in audio documents images pdf video; do printf 'name = "x"\n' > "$dir/$t.toml"; done
  sed -e 's|<Directory Id="ActionsDir" Name="actions" />||' \
      -e 's|<DirectoryRef Id="ActionsDir">|<DirectoryRef Id="INSTALLDIR">|' \
      -e 's|<Icon Id="kawaro.ico" SourceFile="$(var.Ico)" />||' \
      "$WXS" > "$dir/kawaro.wxs"
  wixl -a x64 -D "Version=$2" -D "GuidTail=$3" -D "Release=$dir" -D "Actions=$dir" \
       -D "Ico=$dir/kawaro.ico" -o "$dir/k.msi" "$dir/kawaro.wxs" >/dev/null 2>&1
  "$FIXUP" "$dir/k.msi" "$2" >/dev/null 2>&1
  (cd "$dir/t" && msidump -t "$dir/k.msi" >/dev/null 2>&1)
}
build old 0.5.0 aaaaaaaaaaaa
build new 0.5.1 bbbbbbbbbbbb
check "S-100 both MSIs build" test -f "$d/old/k.msi" -a -f "$d/new/k.msi"

ids() { awk -F'\t' 'NR>3 {print $2}' "$d/$1/t/Component.idt" | sort; }
old_ids="$(ids old)"
new_ids="$(ids new)"
check "S-100 the MSI has four components" test "$(echo "$old_ids" | grep -c .)" -eq 4
check "S-100 no ComponentId is shared between two builds" \
  test -z "$(comm -12 <(echo "$old_ids") <(echo "$new_ids"))"

seq="$d/new/t/InstallExecuteSequence.idt"
num() { awk -F'\t' -v a="$1" '$1==a {print $NF}' "$seq" | head -1; }
rem="$(num RemoveExistingProducts)"
init="$(num InstallInitialize)"
check "S-100 RemoveExistingProducts runs before InstallInitialize" \
  test -n "$rem" -a "${rem:-99999}" -lt "${init:-0}"

# The exes must carry File.Version, else an installed copy whose mtime is
# later than its creation time is kept and the upgrade fails with 2753.
ver() { awk -F'\t' -v k="$2" 'NR>3 && $1==k {print $5}' "$d/$1/t/File.idt"; }
check "S-100 kawaro.exe has File.Version 0.5.1.0" test "$(ver new CliExe)" = "0.5.1.0"
check "S-100 kawarod.exe has File.Version 0.5.1.0" test "$(ver new DaemonExe)" = "0.5.1.0"
finish
