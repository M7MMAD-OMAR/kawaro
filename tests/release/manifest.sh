#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-053c AC1 and AC2 (script half): scripts/make-update-manifest.sh.
#
# Contract (the implementer follows it; DECISION marks a gap in the story):
#   make-update-manifest.sh --dist DIR CHANNEL VERSION ASSETS_JSON OUT_JSON
#   - options come first; --dist is required (DECISION: the story names it
#     only as "--dist <dir>").
#   - CHANNEL is stable or beta; VERSION has no leading "v". A stable manifest
#     for a version with a prerelease part is refused (DECISION).
#   - ASSETS_JSON is the output of `gh release view --json assets`:
#     {"assets":[{"name","url","size",...}]}. Only names ending in .msi, .rpm
#     or .deb become entries, in input order; every other asset is ignored.
#   - OUT_JSON is schema 1: schema, channel, version, published (UTC
#     YYYY-MM-DDTHH:MM:SSZ), notes_url
#     (https://github.com/M7MMAD-OMAR/sbarhive/releases/tag/v<VERSION>, DECISION),
#     assets[] with exactly kind, os, arch, name, url, size, sha256.
#   - exit codes: 0 ok; 2 bad usage or bad input (channel, version, JSON,
#     size not a positive integer, missing assets.json, missing --dist dir);
#     3 a package has no local file in --dist; 4 no .msi, .rpm or .deb asset.
#     On every non zero exit OUT_JSON is not created and nothing else is left
#     next to it.
# shellcheck disable=SC2016,SC2012
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
S="$REPO_ROOT/scripts/make-update-manifest.sh"
V="0.4.0-beta.1"
BASE="https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v$V"
RPM="sbarhive-0.4.0~beta.1-1.x86_64.rpm"
DEB="sbarhive_0.4.0~beta.1_amd64.deb"
MSI="SbarHive-0.4.0-beta.1-x64.msi"

DIST="$TMP/dist"
mkdir -p "$DIST"
printf 'rpm bytes\n' > "$DIST/$RPM"
printf 'deb bytes, a little longer\n' > "$DIST/$DEB"
printf 'msi' > "$DIST/$MSI"
printf 'not a package\n' > "$DIST/SHA256SUMS"

mkassets() { # mkassets FILE RPM_SIZE
  jq -n --arg b "$BASE" --arg rpm "$RPM" --arg deb "$DEB" --arg msi "$MSI" \
    --argjson rs "$2" \
    --argjson ds "$(stat -c %s "$DIST/$DEB")" --argjson ms "$(stat -c %s "$DIST/$MSI")" '
    def a($n; $s): {apiUrl: ("https://api.github.com/x/" + $n), contentType: "application/octet-stream",
      createdAt: "2026-10-24T10:00:00Z", downloadCount: 0, id: "A_1", label: "", name: $n,
      size: $s, state: "uploaded", updatedAt: "2026-10-24T10:00:00Z", url: ($b + "/" + $n)};
    {assets: [a($rpm; $rs), a("SHA256SUMS"; 5), a($deb; $ds), a($msi; $ms), a($msi + ".minisig"; 9)]}' > "$1"
}
RPM_SIZE="$(stat -c %s "$DIST/$RPM")"
mkassets "$TMP/assets.json" "$RPM_SIZE"

OUT="$TMP/out/beta.json"
mkdir -p "$TMP/out"
"$S" --dist "$DIST" beta "$V" "$TMP/assets.json" "$OUT" >"$TMP/run.log" 2>&1
check "AC1 exit 0 on good input" test "$?" -eq 0
check "AC1 output file exists" test -f "$OUT"
check "AC1 script is executable" test -x "$S"
[ -x "$S" ] || { finish; exit 1; }

j() { jq -r "$1" "$OUT" 2>/dev/null; }
check "AC1 top level keys are exactly the schema 1 keys" \
  test "$(jq -c 'keys' "$OUT" 2>/dev/null)" = '["assets","channel","notes_url","published","schema","version"]'
check "AC1 schema is the number 1" test "$(j '.schema|type + ":" + tostring')" = "number:1"
check "AC2 channel" test "$(j .channel)" = beta
check "AC2 version" test "$(j .version)" = "$V"
check "AC2 notes_url" test "$(j .notes_url)" = "https://github.com/M7MMAD-OMAR/sbarhive/releases/tag/v$V"
check "AC2 published is UTC seconds" \
  test "$(j '.published|test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$")')" = true
check "AC1 exactly three entries (SHA256SUMS and .msi.minisig ignored)" test "$(j '.assets|length')" = 3
check "AC1 entries keep the input order" test "$(j '[.assets[].name]|join(",")')" = "$RPM,$DEB,$MSI"
check "AC1 every entry has exactly the eight keys" \
  test "$(jq -c '[.assets[]|keys]|unique' "$OUT" 2>/dev/null)" = '[["arch","kind","name","os","sha256","size","url"]]'

i=0
for n in "$RPM" "$DEB" "$MSI"; do
  want_sha="$(sha256sum "$DIST/$n" | cut -d' ' -f1)"
  check "AC1 sha256 of $n from the local file" test "$(j ".assets[$i].sha256")" = "$want_sha"
  check "AC1 size of $n" test "$(j ".assets[$i].size")" = "$(stat -c %s "$DIST/$n")"
  check "AC1 name of $n verbatim (a ~ survives)" test "$(j ".assets[$i].name")" = "$n"
  check "AC1 url of $n copied from the input" test "$(j ".assets[$i].url")" = "$BASE/$n"
  i=$((i + 1))
done
check "AC1 the ~ is still in the rpm url" test "$(j '.assets[0].url|contains("~")')" = true
check "AC2 rpm: kind rpm, os linux, arch x86_64" \
  test "$(j '.assets[0]|[.kind,.os,.arch]|join(",")')" = "rpm,linux,x86_64"
check "AC2 deb: kind deb, os linux, arch x86_64" \
  test "$(j '.assets[1]|[.kind,.os,.arch]|join(",")')" = "deb,linux,x86_64"
check "AC2 msi: kind msi, os windows, arch x86_64" \
  test "$(j '.assets[2]|[.kind,.os,.arch]|join(",")')" = "msi,windows,x86_64"

# size comes from the input, not from stat
mkassets "$TMP/assets-size.json" 4242
"$S" --dist "$DIST" beta "$V" "$TMP/assets-size.json" "$TMP/out/size.json" >/dev/null 2>&1
check "AC1 size is copied from the input, not measured" \
  test "$(jq -r '.assets[0].size' "$TMP/out/size.json" 2>/dev/null)" = 4242

# a plain version on the beta channel is allowed; stable needs a plain version
jq --arg b "https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0" \
  '.assets|=map(.url|=sub("v0.4.0-beta.1";"v0.4.0"))' "$TMP/assets.json" > "$TMP/assets-plain.json"
"$S" --dist "$DIST" stable 0.4.0 "$TMP/assets-plain.json" "$TMP/out/stable.json" >/dev/null 2>&1
check "AC2 stable manifest for 0.4.0" \
  test "$(jq -r '[.channel,.version]|join(",")' "$TMP/out/stable.json" 2>/dev/null)" = "stable,0.4.0"
"$S" --dist "$DIST" beta 0.4.0 "$TMP/assets-plain.json" "$TMP/out/beta-plain.json" >/dev/null 2>&1
check "AC2 beta manifest for a plain version" \
  test "$(jq -r '[.channel,.version]|join(",")' "$TMP/out/beta-plain.json" 2>/dev/null)" = "beta,0.4.0"

# refusals: exit code and no output file, nothing left beside it
refuse() { # refuse "desc" WANT_CODE args...  (OUT is a fresh dir each time)
  local d="$1" want="$2" dir got
  shift 2
  dir="$(mktemp -d -p "$TMP")"
  "$S" "$@" "$dir/o.json" >"$dir.log" 2>&1
  got=$?
  if [ "$got" = "$want" ] && [ -z "$(ls -A "$dir")" ]; then pass "$d"; else
    fail "$d (exit $got want $want, left: $(ls -A "$dir" | tr '\n' ' '))"; fi
}
mkdir -p "$TMP/dist-missing"
cp "$DIST/$RPM" "$DIST/$DEB" "$TMP/dist-missing/"
refuse "AC1 missing local .msi: exit 3, no output file" 3 --dist "$TMP/dist-missing" beta "$V" "$TMP/assets.json"
mkdir -p "$TMP/dist-nodeb"
cp "$DIST/$RPM" "$DIST/$MSI" "$TMP/dist-nodeb/"
refuse "AC1 missing local .deb: exit 3, no output file" 3 --dist "$TMP/dist-nodeb" beta "$V" "$TMP/assets.json"
refuse "AC1 --dist directory does not exist: exit 2" 2 --dist "$TMP/nope" beta "$V" "$TMP/assets.json"
refuse "AC1 --dist missing: exit 2" 2 beta "$V" "$TMP/assets.json"
refuse "AC1 unknown channel: exit 2" 2 --dist "$DIST" nightly "$V" "$TMP/assets.json"
refuse "AC1 version with a leading v: exit 2" 2 --dist "$DIST" beta "v$V" "$TMP/assets.json"
refuse "AC1 version that is not semver: exit 2" 2 --dist "$DIST" beta 0.4 "$TMP/assets.json"
refuse "AC1 stable channel with a prerelease version: exit 2" 2 --dist "$DIST" stable "$V" "$TMP/assets.json"
refuse "AC1 assets.json missing: exit 2" 2 --dist "$DIST" beta "$V" "$TMP/none.json"
printf 'not json' > "$TMP/bad.json"
refuse "AC1 assets.json not JSON: exit 2" 2 --dist "$DIST" beta "$V" "$TMP/bad.json"
printf '{"files":[]}' > "$TMP/noassets.json"
refuse "AC1 no assets key: exit 2" 2 --dist "$DIST" beta "$V" "$TMP/noassets.json"
jq '.assets[0].size=0' "$TMP/assets.json" > "$TMP/zero.json"
refuse "AC1 size 0 in the input: exit 2" 2 --dist "$DIST" beta "$V" "$TMP/zero.json"
jq '.assets[0].size="12"' "$TMP/assets.json" > "$TMP/strsize.json"
refuse "AC1 size that is a string: exit 2" 2 --dist "$DIST" beta "$V" "$TMP/strsize.json"
jq '.assets|=map(select(.name|test("\\.(msi|rpm|deb)$")|not))' "$TMP/assets.json" > "$TMP/nopkg.json"
refuse "AC1 no package asset at all: exit 4" 4 --dist "$DIST" beta "$V" "$TMP/nopkg.json"
refuse "AC1 too few arguments: exit 2" 2 --dist "$DIST" beta "$V"

finish
