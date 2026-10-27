#!/usr/bin/env bash
# S-051, tests from the acceptance criteria only: the portable zip and the
# per-user MSI. Needs unzip, sha256sum, file, perl, wixl and msiinfo (msitools,
# a build tool only). Run it inside the memory scope, because AC5 compiles and
# runs the ignored Rust test `crates/core/tests/s051_msi_registry.rs`:
#   systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% \
#     env CARGO_BUILD_JOBS=2 bash tests/windows/test_s051.sh
#
# Contracts the acceptance criteria leave open, fixed here:
# - the release tree is "$CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release";
# - the scripts are started with CI=true and KAWARO_WIN_SCOPE=1 and a fake
#   cargo first on PATH; the fake only logs, so any cargo call is visible;
# - the output goes to the working directory or the repository root or the
#   target dir (the test looks for the new file name in those three places and
#   moves it away), or is the last stdout line;
# - the zip has one layout: all five names of AC1 and the action files under
#   one common prefix (empty or one folder) that the zip chooses.
# Deferred (S-048 and S-049 are not done): the icon content and listing, the
# Icon row in the zip and the MSI File table are not asserted; the registry
# Icon and IconUri rows ARE compared with install_commands, which always
# writes them. A placeholder design/brand/kawaro.ico is created for the run
# when the real file is absent, and removed at the end.
# shellcheck disable=SC2016
set -u
# shellcheck source=tests/windows/lib.sh
. "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT" || exit 2
export LC_ALL=C.UTF-8
T="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-s051.XXXXXX")"
ICO="$REPO_ROOT/design/brand/kawaro.ico"
MADE_ICO=0
if [ ! -e "$ICO" ]; then printf 'placeholder ico for tests' > "$ICO"; MADE_ICO=1; fi
cleanup () {
  [ "$MADE_ICO" = 1 ] && rm -f "$ICO"
  case "$T" in "${TMPDIR:-/tmp}"/kawaro-s051.*) rm -rf -- "$T" ;; esac
}
trap cleanup EXIT

VERSION="$(awk '/^\[workspace.package\]/{p=1;next} /^\[/{p=0} p && /^version *=/{gsub(/"/,"",$3); print $3; exit}' Cargo.toml)"
ZIPNAME="kawaro-$VERSION-windows-x64.zip"
MSINAME="kawaro-$VERSION-windows-x64.msi"
ACTIONS="$(cd actions && ls -- *.toml)"
N=0

mkdir -p "$T/bin"
cat > "$T/bin/cargo" <<'EOS'
#!/bin/sh
echo "$*" >> "$CARGO_LOG"
exit 1
EOS
chmod 0755 "$T/bin/cargo"

# make_tree DIR [nodaemon]: a fake release tree with two different small exes.
make_tree () {
  mkdir -p "$1/x86_64-pc-windows-gnu/release"
  printf 'MZ fake cli %s' "$VERSION" > "$1/x86_64-pc-windows-gnu/release/kawaro.exe"
  [ "${2:-}" = nodaemon ] || printf 'MZ fake daemon %s' "$VERSION" > "$1/x86_64-pc-windows-gnu/release/kawarod.exe"
}

# run_script SCRIPT NAME TARGET [CWD]: sets RC, OUT, ERR, ART (the new file
# called NAME moved into $T, empty when none).
run_script () {
  local script="$1" name="$2" target="$3" cwd="${4:-$T/cwd}" marker found d
  N=$((N + 1)); mkdir -p "$cwd"; marker="$T/marker$N"; touch "$marker"; sleep 0.1
  rm -f "$T/cargo.log"
  OUT="$T/out$N.txt"; ERR="$T/err$N.txt"
  (cd "$cwd" && CI=true KAWARO_WIN_SCOPE=1 CARGO_LOG="$T/cargo.log" PATH="$T/bin:$PATH" \
    CARGO_TARGET_DIR="$target" timeout 300 "$REPO_ROOT/scripts/windows/$script" >"$OUT" 2>"$ERR")
  RC=$?
  found=""
  for d in "$cwd" "$REPO_ROOT" "$target"; do
    [ -d "$d" ] || continue
    found="$(find "$d" -maxdepth 2 \( -path "$REPO_ROOT/target" -o -path "$REPO_ROOT/.git" -o -path "$REPO_ROOT/vendor" \) -prune \
      -o -name "$name" -type f -newer "$marker" -print | head -n 1)"
    [ -n "$found" ] && break
  done
  ART=""
  if [ -n "$found" ]; then ART="$T/artifact$N-$name"; mv -f "$found" "$ART"; fi
}

# Tables of an MSI as tab separated rows without the three header lines.
table () { msiinfo export "$1" "$2" 2>/dev/null | tr -d '\r' | tail -n +4; }
# base NAME: the long part of "SHORT~1.EXE|long name".
base () { printf '%s\n' "${1##*|}"; }
# ancestors MSI ID: ID and its Directory parents up to the root.
ancestors () {
  local id="$2" i parent
  for i in 1 2 3 4 5 6 7 8 9 10; do
    printf '%s\n' "$id"
    parent="$(table "$1" Directory | awk -F'\t' -v id="$id" '$1==id{print $2; exit}')"
    [ -z "$parent" ] && return 0
    id="$parent"
  done
}
# dirpath MSI ID: the folder names from LocalAppDataFolder down to ID.
dirpath () {
  local id="$2" out="" i row
  for i in 1 2 3 4 5 6 7 8 9 10; do
    if [ "$id" = LocalAppDataFolder ]; then printf 'LocalAppDataFolder%s\n' "$out"; return 0; fi
    row="$(table "$1" Directory | awk -F'\t' -v id="$id" '$1==id{print $2 "\t" $3; exit}')"
    [ -z "$row" ] && { printf '?%s\n' "$out"; return 0; }
    out="\\$(base "${row#*	}")$out"
    id="${row%%	*}"
  done
}

ac1_zip_name_listing_and_exit_codes () {
  echo "== AC1: make-zip.sh"
  local tree="$T/tree1" list prefix f d dirs hits
  make_tree "$tree"
  check "AC1: scripts/windows/make-zip.sh is executable" test -x "$WIN/make-zip.sh"
  run_script make-zip.sh "$ZIPNAME" "$tree"
  assert_eq "AC1: exit code" "$RC" "0"
  assert_eq "AC1: no cargo call" "$(cat "$T/cargo.log" 2>/dev/null)" ""
  check "AC1: $ZIPNAME written ($ERR)" test -n "$ART"
  [ -n "$ART" ] || return 0
  list="$(unzip -Z1 "$ART" 2>&1)"
  prefix="$(printf '%s\n' "$list" | grep -E '(^|/)kawaro\.exe$' | head -n 1)"; prefix="${prefix%kawaro.exe}"
  for f in kawaro.exe kawarod.exe README-windows.txt SHA256SUMS; do
    assert_eq "AC1: $f listed once" "$(printf '%s\n' "$list" | grep -cxF "$prefix$f")" "1"
  done
  mkdir -p "$T/x1"; unzip -q "$ART" -d "$T/x1"
  assert_eq "AC1: README-windows.txt is the file from installer/windows" \
    "$(sha256sum < "$T/x1/${prefix}README-windows.txt")" "$(sha256sum < installer/windows/README-windows.txt 2>/dev/null)"
  dirs=""
  for f in $ACTIONS; do
    hits="$(printf '%s\n' "$list" | grep -E "(^|/)actions/$f\$")"
    assert_eq "AC1: actions/$f listed once" "$(printf '%s\n' "$hits" | grep -c .)" "1"
    d="${hits%/*}"; dirs="$dirs$d
"
    check "AC1: $f is byte identical to the repository copy" cmp -s "actions/$f" "$T/x1/$hits"
  done
  assert_eq "AC1: all action files share one folder" "$(printf '%s' "$dirs" | sort -u | grep -c .)" "1"
  assert_eq "AC1: the archive holds the 4 named files plus the actions only" \
    "$(printf '%s\n' "$list" | grep -v '/$' | grep -vE '(^|/)kawaro\.ico$' | grep -c .)" "$((4 + $(printf '%s\n' "$ACTIONS" | grep -c .)))"
  assert_eq "AC1: unzip -t" "$(unzip -tq "$ART" | tail -n 1)" "No errors detected in compressed data of $ART."

}

ac1_missing_daemon_exits_2 () {
  echo "== AC1: missing kawarod.exe"
  make_tree "$T/tree1b" nodaemon
  run_script make-zip.sh "$ZIPNAME" "$T/tree1b" "$T/cwd1b"
  assert_eq "AC1: exit code without kawarod.exe" "$RC" "2"
  check "AC1: the message names kawarod.exe" grep -q 'kawarod\.exe' "$ERR" "$OUT"
  assert_eq "AC1: no cargo call without kawarod.exe" "$(cat "$T/cargo.log" 2>/dev/null)" ""
  assert_eq "AC1: no zip written without kawarod.exe" "$ART" ""
}

ac2_sha256sums_and_reproducible_zip () {
  echo "== AC2: SHA256SUMS and a second run"
  local tree="$T/tree2" list prefix first second n files
  make_tree "$tree"
  run_script make-zip.sh "$ZIPNAME" "$tree" "$T/cwd2a"
  first="$ART"
  check "AC2: first zip written" test -n "$first"
  [ -n "$first" ] || return 0
  mkdir -p "$T/x2"; unzip -q "$first" -d "$T/x2"
  list="$(unzip -Z1 "$first")"
  prefix="$(printf '%s\n' "$list" | grep -E '(^|/)SHA256SUMS$' | head -n 1)"; prefix="${prefix%SHA256SUMS}"
  files="$(printf '%s\n' "$list" | grep -v '/$' | grep -vxF "${prefix}SHA256SUMS" | sed "s|^$prefix||" | sort)"
  n="$(printf '%s\n' "$files" | grep -c .)"
  assert_eq "AC2: one SHA256SUMS line per file" "$(grep -c . "$T/x2/${prefix}SHA256SUMS")" "$n"
  assert_eq "AC2: SHA256SUMS names exactly the other files" \
    "$(sed -E 's/^[0-9a-f]{64} [ *]//' "$T/x2/${prefix}SHA256SUMS" | sort)" "$files"
  check "AC2: every hash line is 64 hex digits" test "$(grep -cE '^[0-9a-f]{64} [ *][^ ]' "$T/x2/${prefix}SHA256SUMS")" = "$n"
  assert_eq "AC2: sha256sum -c passes in the extracted folder" \
    "$(cd "$T/x2/$prefix" && sha256sum -c SHA256SUMS >/dev/null 2>&1; echo $?)" "0"
  assert_eq "AC2: hash of kawarod.exe" \
    "$(grep -F ' kawarod.exe' "$T/x2/${prefix}SHA256SUMS" | cut -c1-64)" "$(sha256sum < "$tree/x86_64-pc-windows-gnu/release/kawarod.exe" | cut -c1-64)"
  # A second run on unchanged content: older mtimes, later wall clock, other cwd and umask.
  touch -d '2001-01-01 00:00:00' "$tree"/x86_64-pc-windows-gnu/release/*.exe
  sleep 3
  umask 077
  run_script make-zip.sh "$ZIPNAME" "$tree" "$T/cwd2b"
  umask 022
  second="$ART"
  check "AC2: second zip written" test -n "$second"
  assert_eq "AC2: the two zips have the same SHA-256" "$(sha256sum < "$second")" "$(sha256sum < "$first")"
}

ac3_windows_readme_text () {
  echo "== AC3: README-windows.txt"
  local f=installer/windows/README-windows.txt en ar
  check "AC3: $f exists" test -f "$f"
  [ -f "$f" ] || return 0
  check "AC3: the three steps: unzip" grep -qi 'unzip' "$f"
  check "AC3: the command" grep -qF 'kawaro integration install' "$f"
  check "AC3: the command is named in both languages" test "$(grep -cF 'kawaro integration install' "$f")" -ge 2
  check "AC3: Show more options" grep -qF 'Show more options' "$f"
  check "AC3: SmartScreen" grep -qF 'SmartScreen' "$f"
  check "AC3: More info" grep -qF 'More info' "$f"
  check "AC3: Run anyway" grep -qF 'Run anyway' "$f"
  check "AC3: Arabic text is present" perl -CSD -ne 'BEGIN{$f=0} $f=1 if /[\x{0600}-\x{06FF}]/; END{exit !$f}' "$f"
  en="$(perl -CSD -ne 'if (/[\x{0600}-\x{06FF}]/) {print "$.\n"; exit}' "$f")"
  ar="$(grep -nF 'Run anyway' "$f" | head -n 1 | cut -d: -f1)"
  check "AC3: English comes first, then Arabic (first Arabic line $en, Run anyway line $ar)" test -n "$en" -a -n "$ar" -a "${ar:-0}" -lt "${en:-0}"
  check "AC3: no long dash" bash -c '! LC_ALL=C grep -qP "\xE2\x80[\x93\x94]" "$1"' _ "$f"
}

# build_msi NAME TARGET: runs make-msi.sh, sets MSI (empty on failure).
build_msi () {
  run_script make-msi.sh "$MSINAME" "$2" "$T/cwd-$1"
  MSI="$ART"
}

ac4_msi_properties_and_install_folder () {
  echo "== AC4: make-msi.sh"
  local tree="$T/tree4" ver
  make_tree "$tree"
  check "AC4: scripts/windows/make-msi.sh is executable" test -x "$WIN/make-msi.sh"
  check "AC4: installer/windows/kawaro.wxs exists" test -f installer/windows/kawaro.wxs
  build_msi 4 "$tree"
  assert_eq "AC4: exit code" "$RC" "0"
  assert_eq "AC4: no cargo call" "$(cat "$T/cargo.log" 2>/dev/null)" ""
  check "AC4: $MSINAME written ($ERR)" test -n "$MSI"
  [ -n "$MSI" ] || return 0
  check "AC4: file says composite document" bash -c 'file -b "$1" | grep -q "Composite Document File V2 Document"' _ "$MSI"
  check "AC4: file says MSI Installer" bash -c 'file -b "$1" | grep -q "MSI Installer"' _ "$MSI"
  ver="$(table "$MSI" Property | awk -F'\t' '$1=="ProductVersion"{print $2}')"
  assert_eq "AC4: ProductVersion (numeric part only, an MSI version has no prerelease)" "$ver" "${VERSION%%[-+]*}"
  assert_eq "AC4: no ALLUSERS=1 row" "$(table "$MSI" Property | awk -F'\t' '$1=="ALLUSERS" && $2=="1"' | grep -c .)" "0"
  assert_eq "AC4: the install folder is LocalAppDataFolder\\Programs\\Kawaro" \
    "$(dirpath "$MSI" INSTALLDIR)" 'LocalAppDataFolder\Programs\Kawaro'
}

ac4_msi_from_the_real_cross_built_exes () {
  echo "== AC4: with the real cross built exes"
  local real="${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target"
  check "AC4: the real release tree exists (build it with scripts/windows/build.sh)" test -f "$real/x86_64-pc-windows-gnu/release/kawarod.exe"
  build_msi 4r "$real"
  assert_eq "AC4: exit code with the real exes" "$RC" "0"
  check "AC4: MSI from the real exes written" test -n "$MSI"
  assert_eq "AC4: ProductVersion with the real exes" "$(table "$MSI" Property | awk -F'\t' '$1=="ProductVersion"{print $2}')" "${VERSION%%[-+]*}"
}

ac5_registry_rows_match_s050 () {
  echo "== AC5: Registry table against install_commands"
  local tree="$T/tree5" log="$T/cargo-test.log"
  make_tree "$tree"
  build_msi 5 "$tree"
  check "AC5: MSI written" test -n "$MSI"
  [ -n "$MSI" ] || return 0
  msiinfo export "$MSI" Registry > "$T/registry.txt" 2>/dev/null
  # S-085 D2: --include-ignored also runs the check that the projection only
  # maps the bilingual DisplayName, so five Rust checks run here.
  (S051_REGISTRY_EXPORT="$T/registry.txt" CARGO_BUILD_JOBS=2 cargo test -p kawaro-core --test s051_msi_registry -- --include-ignored >"$log" 2>&1)
  assert_eq "AC5: cargo test exit code" "$?" "0"
  check "AC5: all five Rust checks ran and passed" grep -q 'test result: ok. 5 passed; 0 failed' "$log"
  [ -s "$log" ] && grep -E 'panicked|assert|left|right|FAILED|failed' "$log" | head -n 12 >&2
}

ac6_files_and_start_menu_shortcut () {
  echo "== AC6: File and Shortcut tables"
  local tree="$T/tree6" names f
  make_tree "$tree"
  build_msi 6 "$tree"
  check "AC6: MSI written" test -n "$MSI"
  [ -n "$MSI" ] || return 0
  names="$(table "$MSI" File | while IFS=$'\t' read -r _ _ fn _; do base "$fn"; done)"
  for f in kawaro.exe kawarod.exe $ACTIONS; do
    assert_eq "AC6: File table lists $f once" "$(printf '%s\n' "$names" | grep -cxF "$f")" "1"
  done
  local sc row icon
  # S-085 AC2: the MSI keeps the ASCII name (wixl segfaults on anything else);
  # the daemon renames the shortcut to the bilingual name after install.
  sc="$(table "$MSI" Shortcut | while IFS=$'\t' read -r _ dir name _; do [ "$(base "$name")" = Kawaro ] && echo "$dir"; done)"
  assert_eq "AC6: exactly one shortcut named Kawaro (ASCII, S-085 AC2)" "$(printf '%s\n' "$sc" | grep -c .)" "1"
  if [ -n "$sc" ]; then
    assert_eq "AC6: the shortcut folder reaches ProgramMenuFolder" \
      "$(ancestors "$MSI" "$sc" | grep -cxF ProgramMenuFolder)" "1"
  fi
  # S-085 AC2, D6: Shortcut columns 9 and 10 are Icon_ and IconIndex.
  row="$(table "$MSI" Shortcut | while IFS=$'\t' read -r _ _ name rest; do [ "$(base "$name")" = Kawaro ] && printf '%s\n' "$rest"; done | head -n 1)"
  icon="$(printf '%s\n' "$row" | cut -f6)"
  assert_eq "S-085 AC2: the Kawaro shortcut references the icon kawaro.ico" "$icon" "kawaro.ico"
  assert_eq "S-085 AC2: the Kawaro shortcut uses icon index 0" "$(printf '%s\n' "$row" | cut -f7)" "0"
  # msiinfo export writes the streams of a binary table into ./Icon, so it runs in $T.
  assert_eq "S-085 AC2: one Icon table row for kawaro.ico" \
    "$( (cd "$T" && table "$MSI" Icon) | awk -F'\t' '$1=="kawaro.ico"' | grep -c .)" "1"
  check "S-085 AC2: the Icon table data is design/brand/kawaro.ico byte for byte" \
    bash -c 'msiinfo extract "$1" "Icon.kawaro.ico" 2>/dev/null | cmp -s - "$2"' _ "$MSI" "$ICO"
  # S-085 D6: RemoveFile columns: FileKey, Component_, FileName, DirProperty,
  # InstallMode (2 is on uninstall, 3 is both).
  assert_eq "S-085 D6: StartMenuShortcut removes Kawaro*.lnk from ProgramMenuFolder on uninstall" \
    "$(table "$MSI" RemoveFile | awk -F'\t' '$2=="StartMenuShortcut" && $3=="Kawaro*.lnk" && $4=="ProgramMenuFolder" && ($5=="2" || $5=="3")' | grep -c .)" "1"
  # S-086 AC2 (backlog/S-086-icon-opens-window.md): the Start entry runs
  # `kawarod.exe settings`, so a click opens the window instead of the silent
  # daemon mode of today. Checked on the built MSI's Shortcut table row, not
  # on the source text. awk keeps the empty Arguments column, unlike read.
  row="$(table "$MSI" Shortcut | awk -F'\t' '{n=$3; sub(/^.*\|/, "", n)} n=="Kawaro" {print $5 "\t" $6; exit}')"
  assert_eq "AC6 (S-086): the shortcut target is [INSTALLDIR]kawarod.exe" \
    "$(printf '%s\n' "$row" | cut -f1)" "[INSTALLDIR]kawarod.exe"
  assert_eq "AC6 (S-086): the shortcut arguments are settings" \
    "$(printf '%s\n' "$row" | cut -f2)" "settings"
}
# S-085 D6: the icon is a required build input. A copy of the MSI inputs in a
# root without design/brand/kawaro.ico must exit 2 and name the icon.
d6_msi_needs_the_icon () {
  echo "== S-085 D6: make-msi.sh without kawaro.ico"
  local root="$T/noico" tree="$T/tree-noico"
  mkdir -p "$root/scripts/windows" "$root/installer/windows" "$root/design/brand"
  cp scripts/windows/make-msi.sh scripts/windows/scope.sh "$root/scripts/windows/"
  cp installer/windows/kawaro.wxs "$root/installer/windows/"
  cp Cargo.toml "$root/"
  cp -r actions "$root/actions"
  make_tree "$tree"
  rm -f "$T/cargo.log"
  (cd "$T" && CI=true KAWARO_WIN_SCOPE=1 CARGO_LOG="$T/cargo.log" PATH="$T/bin:$PATH" \
    CARGO_TARGET_DIR="$tree" timeout 300 "$root/scripts/windows/make-msi.sh" >"$T/noico.out" 2>"$T/noico.err")
  assert_eq "S-085 D6: make-msi.sh exits 2 without kawaro.ico" "$?" "2"
  check "S-085 D6: the message names kawaro.ico" grep -q 'kawaro\.ico' "$T/noico.err" "$T/noico.out"
  assert_eq "S-085 D6: no MSI written without kawaro.ico" \
    "$(find "$tree" -name '*.msi' | grep -c .)" "0"
}

ac7_docs_changelog_and_lint () {
  echo "== AC7: docs, CHANGELOG, lint"
  local ver lic l w
  ver="$(rpm -q --qf '%{VERSION}' msitools)"; lic="$(rpm -q --qf '%{LICENSE}' msitools)"
  w="$(grep -i -A3 'msitools' docs/engines.md)"
  check "AC7: docs/engines.md names msitools" test -n "$w"
  check "AC7: msitools version $ver" grep -qF "$ver" <<<"$w"
  for l in ${lic//AND/}; do check "AC7: msitools licence $l" grep -qF "$l" <<<"$w"; done
  check "AC7: states build tool only" grep -qi 'build tool only' <<<"$w"
  check "AC7: README.md unsigned warning with the SHA-256 note, about Windows" \
    perl -CSD -00 -ne 'BEGIN{$f=0} $f=1 if /unsigned/i && /SHA-?256/i && /Windows/; END{exit !$f}' README.md
  check "AC7: README.ar.md unsigned warning with the SHA-256 note" \
    perl -CSD -Mutf8 -00 -ne 'BEGIN{$f=0} $f=1 if /(غير موقّع|غير موقع|غير موقعة|غير موقّعة|بدون توقيع)/ && /SHA-?256/i && /(ويندوز|Windows)/; END{exit !$f}' README.ar.md
  assert_eq "AC7: one CHANGELOG line for S-051" "$(grep -c 'S-051' CHANGELOG.md)" "1"
  check "AC7: that line also has the Arabic sentence" bash -c 'grep "S-051" CHANGELOG.md | perl -CSD -ne "BEGIN{\$f=0} \$f=1 if /[\x{0600}-\x{06FF}]/; END{exit !\$f}"'
  assert_eq "AC7: design-lint exit code" "$(scripts/design-lint.sh >/dev/null 2>&1; echo $?)" "0"
  for l in scripts/windows/make-zip.sh scripts/windows/make-msi.sh installer/windows/kawaro.wxs installer/windows/README-windows.txt docs/engines.md README.md README.ar.md CHANGELOG.md; do
    check "AC7: $l exists without a long dash" bash -c '[ -f "$1" ] && ! LC_ALL=C grep -qP "\xE2\x80[\x93\x94]" "$1"' _ "$l"
  done
}

ac1_zip_name_listing_and_exit_codes
ac1_missing_daemon_exits_2
ac2_sha256sums_and_reproducible_zip
ac3_windows_readme_text
ac4_msi_properties_and_install_folder
ac4_msi_from_the_real_cross_built_exes
ac5_registry_rows_match_s050
ac6_files_and_start_menu_shortcut
d6_msi_needs_the_icon
ac7_docs_changelog_and_lint
finish "S-051 windows installer"
