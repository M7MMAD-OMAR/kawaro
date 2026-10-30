#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-055 AC1 (check-versions on real output), AC6 and AC7: scripts/release/dry-run.sh
# and scripts/release/check-versions.sh. Light: no cargo, no compile; the
# Linux and Windows "binaries" are small fixtures.
#
# Contract fixed here (the story leaves it open; DECISION marks a gap):
#   dry-run.sh VERSION [--no-build]
#     env BIN_DIR           Linux release binaries (kawaro, kawarod); default target/release
#     env CARGO_TARGET_DIR  Windows tree, uses $CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release
#     env DRYRUN_OUT        output dir (DECISION), default a fresh temp dir; it holds exactly:
#       rpm, deb, zip, msi, SHA256SUMS (4 lines: rpm, deb, zip, msi), BETA/STABLE manifests
#       (beta.json always; stable.json only for a version with no prerelease part), a
#       .minisig for each manifest, and dry-run.pub (the throwaway public key)
#     stdout: the full path of each file in DRYRUN_OUT, one per line, plus the 6 lines of
#       check-versions.sh; exit 0 ok, 2 missing tool (message names it) or bad version.
#     Never calls gh, git push or any network tool; never reads the real signing key directory;
#     works on a copy of `git ls-files`; the source tree is left untouched.
#   check-versions.sh VERSION --out DIR [--root DIR]
#     --root holds Cargo.toml (default: the repo root), --out is a DRYRUN_OUT style dir.
#     stdout is always exactly 6 lines, `ok SOURCE: VALUE` or `FAIL SOURCE: found X, want Y`,
#     with SOURCE in this order: Cargo.toml, RPM, DEB, MSI, names, manifest.
#     Exit 0 only when all 6 are ok; 1 on any mismatch; 2 on misuse (no VERSION, no --out,
#     not semver).
# shellcheck disable=SC2016,SC2012
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT" || exit 1
DRY=scripts/release/dry-run.sh
CHK=scripts/release/check-versions.sh
SHIMS="$REPO_ROOT/tests/release/shims"
V="0.4.0-beta.1"; M="0.4.0~beta.1"

check "AC7 $DRY exists and is executable" test -x "$DRY"
check "AC1 $CHK exists and is executable" test -x "$CHK"
if [ ! -x "$DRY" ] || [ ! -x "$CHK" ]; then
  echo "scripts missing: the rest of this file needs them" >&2
  finish; exit 1
fi
for t in rpmbuild wixl msiinfo minisign jq ar unzip zip; do
  command -v "$t" >/dev/null || { echo "SKIP all: $t not installed here (CI installs it)"; finish; exit $?; }
done

# fixtures: Linux binaries and a Windows release tree
mkdir -p "$TMP/bin" "$TMP/wt/x86_64-pc-windows-gnu/release" "$TMP/home"
cp /usr/bin/true "$TMP/bin/kawaro"; cp /usr/bin/true "$TMP/bin/kawarod"
printf 'MZ fake cli' > "$TMP/wt/x86_64-pc-windows-gnu/release/kawaro.exe"
printf 'MZ fake daemon' > "$TMP/wt/x86_64-pc-windows-gnu/release/kawarod.exe"
# shims that record any network or publishing call
NETLOG="$TMP/net.log"; : > "$NETLOG"
mkdir -p "$TMP/net"
for t in gh curl wget ssh scp; do
  printf '#!/bin/sh\necho "%s $*" >> "%s"\nexit 1\n' "$t" "$NETLOG" > "$TMP/net/$t"; chmod +x "$TMP/net/$t"
done
REAL_GIT="$(command -v git)"
cat > "$TMP/net/git" <<EOS
#!/bin/sh
case "\$1" in push|fetch|pull|clone|remote|ls-remote) echo "git \$*" >> "$NETLOG"; exit 1 ;; esac
exec "$REAL_GIT" "\$@"
EOS
chmod +x "$TMP/net/git"

dry() { # dry OUT_NAME VERSION [args]  -> RC in $TMP/rc-NAME, stdout $TMP/out-NAME.txt, files in $TMP/NAME
  local n="$1" v="$2"; shift 2
  local s=$SECONDS
  (cd "$REPO_ROOT" && HOME="$TMP/home" CI=true KAWARO_WIN_SCOPE=1 BIN_DIR="$TMP/bin" \
    CARGO_TARGET_DIR="$TMP/wt" DRYRUN_OUT="$TMP/$n" PATH="$TMP/net:$SHIMS:$PATH" \
    timeout 120 "$REPO_ROOT/$DRY" "$v" "$@" >"$TMP/out-$n.txt" 2>"$TMP/err-$n.txt")
  echo $? > "$TMP/rc-$n"
  echo $((SECONDS - s)) > "$TMP/secs-$n"
}
status_before="$(git status --porcelain)"
worktrees_before="$(git worktree list)"
branches_before="$(git branch --list)"

# ---- success: a prerelease version
dry beta "$V" --no-build
check "AC7 --no-build $V exits 0" test "$(cat "$TMP/rc-beta")" = 0
check "AC7 --no-build finishes in under 60 s" test "$(cat "$TMP/secs-beta")" -lt 60
EXPECT_BETA="SHA256SUMS
beta.json
beta.json.minisig
dry-run.pub
kawaro-0.4.0-beta.1-windows-x64.msi
kawaro-0.4.0-beta.1-windows-x64.zip
kawaro-0.4.0~beta.1-1.x86_64.rpm
kawaro_0.4.0~beta.1_amd64.deb"
check "AC7 the output dir holds exactly the expected files" \
  test "$(ls -1 "$TMP/beta" 2>/dev/null | LC_ALL=C sort)" = "$(LC_ALL=C sort <<<"$EXPECT_BETA")"
for f in $EXPECT_BETA; do
  check "AC7 stdout lists $TMP/beta/$f" grep -qxF "$TMP/beta/$f" "$TMP/out-beta.txt"
done
check "AC7 stdout has 6 ok lines from check-versions (Cargo.toml RPM DEB MSI names manifest)" \
  test "$(grep -cE '^ok (Cargo\.toml|RPM|DEB|MSI|names|manifest): ' "$TMP/out-beta.txt")" -eq 6
check "AC7 stdout has no FAIL line" bash -c '! grep -q "^FAIL " "$1"' _ "$TMP/out-beta.txt"
check "AC7 no network, gh or git push call was attempted" test ! -s "$NETLOG"
check "AC7 the source tree is unchanged (git status same as before)" test "$(git status --porcelain)" = "$status_before"
check "AC7 no branch or worktree was created" test "$(git worktree list)" = "$worktrees_before" -a "$(git branch --list)" = "$branches_before"

B="$TMP/beta"
check "AC9 SHA256SUMS has exactly 4 lines (rpm, deb, zip, msi)" test "$(wc -l < "$B/SHA256SUMS")" -eq 4
expect_status "AC9 sha256sum -c SHA256SUMS" 0 bash -c "cd '$B' && sha256sum -c SHA256SUMS"
check "AC9 SHA256SUMS names the four packages by base name" \
  test "$(awk '{print $2}' "$B/SHA256SUMS" | LC_ALL=C sort | tr '\n' ' ')" = \
  "kawaro-0.4.0-beta.1-windows-x64.msi kawaro-0.4.0-beta.1-windows-x64.zip kawaro-0.4.0~beta.1-1.x86_64.rpm kawaro_0.4.0~beta.1_amd64.deb "
# manifest and signature made with the throwaway key
check "AC7 beta.json version" test "$(jq -r .version "$B/beta.json")" = "$V"
check "AC7 beta.json channel" test "$(jq -r .channel "$B/beta.json")" = beta
check "AC7 beta.json has the rpm, deb and msi entries only" \
  test "$(jq -r '[.assets[].kind]|sort|join(",")' "$B/beta.json")" = "deb,msi,rpm"
check "AC7 every manifest url is under the release download prefix" \
  test "$(jq -r '[.assets[].url|startswith("https://github.com/M7MMAD-OMAR/kawaro/releases/download/")]|all' "$B/beta.json")" = true
expect_status "AC7 beta.json verifies with the throwaway public key" 0 \
  minisign -V -p "$B/dry-run.pub" -m "$B/beta.json"
check "AC7 the throwaway key is not the real update key" \
  bash -c '! cmp -s "$1/dry-run.pub" "$2/packaging/keys/kawaro-update.pub"' _ "$B" "$REPO_ROOT"
# AC6
RPMF="$B/kawaro-$M-1.x86_64.rpm"
check "AC5 rpm -qp VERSION is $M" test "$(rpm -qp --qf '%{VERSION}' "$RPMF" 2>/dev/null)" = "$M"
check "AC5 deb Version is $M" test "$("$SHIMS/dpkg-deb" -f "$B/kawaro_${M}_amd64.deb" Version)" = "$M"
MSIF="$B/kawaro-$V-windows-x64.msi"
check "AC6 msiinfo export Property: ProductVersion is 0.4.0" \
  test "$(msiinfo export "$MSIF" Property 2>/dev/null | tr -d '\r' | awk -F'\t' '$1=="ProductVersion"{print $2}')" = 0.4.0
ZIPF="$B/kawaro-$V-windows-x64.zip"
mkdir -p "$TMP/unz"
unzip -q "$ZIPF" -d "$TMP/unz"
check "AC6 the zip folder is kawaro-$V-windows-x64" test -f "$TMP/unz/kawaro-$V-windows-x64/SHA256SUMS"
expect_status "AC6 the zip's own SHA256SUMS verifies" 0 bash -c "cd '$TMP/unz/kawaro-$V-windows-x64' && sha256sum -c SHA256SUMS"

# ---- check-versions on that output (AC1)
RT="$TMP/root-beta"; mkdir -p "$RT"
printf '[workspace.package]\nversion = "%s"\n' "$V" > "$RT/Cargo.toml"
"$CHK" "$V" --root "$RT" --out "$B" > "$TMP/chk.txt" 2>"$TMP/chk.err"; RC=$?
check "AC1 check-versions on the dry-run output exits 0" test "$RC" = 0
check "AC1 check-versions prints exactly 6 lines" test "$(wc -l < "$TMP/chk.txt")" -eq 6
check "AC1 the six sources in order" \
  test "$(sed 's/^[a-zA-Z]* \([^:]*\):.*/\1/' "$TMP/chk.txt" | tr '\n' ',')" = "Cargo.toml,RPM,DEB,MSI,names,manifest,"
check "AC1 Cargo.toml line" grep -qxF "ok Cargo.toml: $V" "$TMP/chk.txt"
check "AC1 RPM line" grep -qxF "ok RPM: $M" "$TMP/chk.txt"
check "AC1 DEB line" grep -qxF "ok DEB: $M" "$TMP/chk.txt"
check "AC1 MSI line" grep -qxF "ok MSI: 0.4.0" "$TMP/chk.txt"
check "AC1 manifest line" grep -qxF "ok manifest: $V" "$TMP/chk.txt"
check "AC1 names line" grep -qE "^ok names: .*$V" "$TMP/chk.txt"

mismatch() { # mismatch "desc" SOURCE ROOT OUTDIR [VERSION]
  local d="$1" src="$2" root="$3" out="$4" ver="${5:-$V}" rc
  "$CHK" "$ver" --root "$root" --out "$out" > "$TMP/m.txt" 2>/dev/null; rc=$?
  if [ "$rc" = 1 ] && [ "$(wc -l < "$TMP/m.txt")" -eq 6 ] && grep -q "^FAIL $src: " "$TMP/m.txt" \
     && [ "$(grep -c '^FAIL ' "$TMP/m.txt")" -eq 1 ]; then pass "$d"; else fail "$d (exit $rc: $(tr '\n' '|' < "$TMP/m.txt"))"; fi
}
BADROOT="$TMP/root-bad"; mkdir -p "$BADROOT"
printf '[workspace.package]\nversion = "0.3.3"\n' > "$BADROOT/Cargo.toml"
mismatch "AC1 wrong Cargo.toml version: exit 1 naming Cargo.toml" Cargo.toml "$BADROOT" "$B"
# a manifest with another version
cp -a "$B" "$TMP/o-manifest"; jq '.version="0.4.0"' "$B/beta.json" > "$TMP/o-manifest/beta.json"
mismatch "AC1 wrong manifest version: exit 1 naming manifest" manifest "$RT" "$TMP/o-manifest"
# a zip with the wrong file name
cp -a "$B" "$TMP/o-names"; mv "$TMP/o-names/kawaro-$V-windows-x64.zip" "$TMP/o-names/kawaro-0.4.0-windows-x64.zip"
mismatch "AC1 zip name without the version: exit 1 naming names" names "$RT" "$TMP/o-names"
# rpm and deb built for the plain version (spec define 0.4.0 instead of 0.4.0~beta.1) and msi built for 0.5.0
mktree() { # mktree DIR VERSION
  mkdir -p "$1"
  (cd "$REPO_ROOT" && git ls-files -z --cached | tar --null -T - --ignore-failed-read -cf - 2>/dev/null) | tar -xf - -C "$1"
  sed -i "0,/^version = \".*\"/s//version = \"$2\"/" "$1/Cargo.toml"
}
mktree "$TMP/t-plain" 0.4.0
PATH="$SHIMS:$PATH" "$TMP/t-plain/packaging/linux/build-packages.sh" "$TMP/bin" "$TMP/plain-out" >/dev/null 2>&1
cp -a "$B" "$TMP/o-rpm"; cp "$TMP/plain-out/kawaro-0.4.0-1.x86_64.rpm" "$TMP/o-rpm/kawaro-$M-1.x86_64.rpm"
mismatch "AC1 spec define 0.4.0 instead of $M: exit 1 naming RPM" RPM "$RT" "$TMP/o-rpm"
cp -a "$B" "$TMP/o-deb"; cp "$TMP/plain-out/kawaro_0.4.0_amd64.deb" "$TMP/o-deb/kawaro_${M}_amd64.deb"
mismatch "AC1 deb with Version 0.4.0: exit 1 naming DEB" DEB "$RT" "$TMP/o-deb"
mktree "$TMP/t-msi" 0.5.0
(cd "$TMP/t-msi" && CI=true KAWARO_WIN_SCOPE=1 CARGO_TARGET_DIR="$TMP/wt" scripts/windows/make-msi.sh >/dev/null 2>&1)
cp -a "$B" "$TMP/o-msi"; cp "$TMP/wt/dist/kawaro-0.5.0-windows-x64.msi" "$TMP/o-msi/kawaro-$V-windows-x64.msi"
mismatch "AC1 MSI ProductVersion 0.5.0: exit 1 naming MSI" MSI "$RT" "$TMP/o-msi"
# misuse
expect_status "AC1 check-versions without arguments exits 2" 2 "$CHK"
expect_status "AC1 check-versions with a non semver version exits 2" 2 "$CHK" foo --root "$RT" --out "$B"
expect_status "AC1 check-versions without --out exits 2" 2 "$CHK" "$V" --root "$RT"

# ---- success: a plain version gets both channels
dry plain 0.4.0 --no-build
check "AC7 --no-build 0.4.0 exits 0" test "$(cat "$TMP/rc-plain")" = 0
EXPECT_PLAIN="SHA256SUMS
beta.json
beta.json.minisig
dry-run.pub
kawaro-0.4.0-1.x86_64.rpm
kawaro-0.4.0-windows-x64.msi
kawaro-0.4.0-windows-x64.zip
kawaro_0.4.0_amd64.deb
stable.json
stable.json.minisig"
check "AC7 plain version: exactly the expected files, with stable.json" \
  test "$(ls -1 "$TMP/plain" 2>/dev/null | LC_ALL=C sort)" = "$(LC_ALL=C sort <<<"$EXPECT_PLAIN")"
expect_status "AC7 stable.json verifies with the throwaway key" 0 \
  minisign -V -p "$TMP/plain/dry-run.pub" -m "$TMP/plain/stable.json"
check "AC7 stable.json channel" test "$(jq -r .channel "$TMP/plain/stable.json" 2>/dev/null)" = stable

# ---- failures: exit 2
dry bad1 foo --no-build
check "AC7 version 'foo' exits 2" test "$(cat "$TMP/rc-bad1")" = 2
dry bad2 v0.4.0 --no-build
check "AC7 version 'v0.4.0' exits 2" test "$(cat "$TMP/rc-bad2")" = 2
dry bad3 0.4 --no-build
check "AC7 version '0.4' exits 2" test "$(cat "$TMP/rc-bad3")" = 2
"$DRY" >/dev/null 2>&1; check "AC7 no version exits 2" test "$?" = 2
check "AC7 a bad version creates no output dir" test ! -e "$TMP/bad1"

hidden() { # hidden TOOL: PATH without TOOL (and without the dpkg-deb shim)
  local d="$TMP/path-no-$1"
  mkdir -p "$d"
  find /usr/bin -mindepth 1 ! -name "$1" -exec ln -sf -t "$d" {} + 2>/dev/null
  printf '%s' "$d"
}
for tool in wixl minisign rpmbuild dpkg-deb; do
  P="$(hidden "$tool")"
  [ "$tool" = dpkg-deb ] || cp "$SHIMS/dpkg-deb" "$P/dpkg-deb"
  HOME="$TMP/home" CI=true KAWARO_WIN_SCOPE=1 BIN_DIR="$TMP/bin" CARGO_TARGET_DIR="$TMP/wt" \
    DRYRUN_OUT="$TMP/h-$tool" PATH="$P" "$REPO_ROOT/$DRY" "$V" --no-build >"$TMP/h-$tool.out" 2>"$TMP/h-$tool.err"
  rc=$?
  check "AC7 $tool hidden: exit 2 (got $rc)" test "$rc" = 2
  check "AC7 $tool hidden: the message names $tool" grep -qF "$tool" "$TMP/h-$tool.err" "$TMP/h-$tool.out"
done
check "AC7 a missing tool leaves no output files" test -z "$(ls -A "$TMP/h-wixl" 2>/dev/null)"

# ---- static properties of the script
check "AC7 no gh, push or curl in the script" \
  bash -c '! grep -Eq "(^|[^a-zA-Z_.-])(gh |git push|curl|wget)" "$1"' _ "$DRY"
check "AC7 the script never names the real signing key location" bash -c '! grep -q "win-test-signing" "$1"' _ "$DRY"
check "AC7 the script uses a throwaway key (minisign -G -W)" grep -Eq 'minisign -G[^|]* -W|minisign -W[^|]* -G' "$DRY"
check "AC7 the script calls make-update-manifest.sh with --dist" grep -Eq 'make-update-manifest\.sh"? +--dist' "$DRY"
check "AC7 the script calls sign-update-manifest.sh" grep -q 'sign-update-manifest\.sh' "$DRY"
check "AC7 the script names the mingw target for its tool check" grep -q 'x86_64-pc-windows-gnu' "$DRY"
check "AC7 the script calls check-versions.sh" grep -q 'check-versions\.sh' "$DRY"
# the sign script gets the passphrase through the environment (stdin inside it)
check "AC7 the script sets UPDATE_SIGNING_PASSWORD for the sign script (empty for -W keys)" \
  grep -q 'UPDATE_SIGNING_PASSWORD' "$DRY"
for s in $DRY $CHK; do
  expect_status "AC9 shellcheck $s" 0 shellcheck "$s"
  check "AC9 $s starts with a bash shebang" test "$(head -1 "$s")" = '#!/usr/bin/env bash'
  check "AC9 $s has the SPDX header" grep -q 'SPDX-License-Identifier: GPL-3.0-or-later' "$s"
  check "AC9 $s uses set -euo pipefail" grep -q '^set -euo pipefail' "$s"
done
check "AC7 CI installs msitools so these tests do not skip" \
  bash -c 'sed ":a;/\\\\$/{N;s/\\\\\n//;ba}" .github/workflows/ci.yml | grep "apt-get install" | grep -Eqw "msitools"'
check "AC7 the working tree is still unchanged at the end" test "$(git status --porcelain)" = "$status_before"
finish
