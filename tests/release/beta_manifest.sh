#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-055 carried recommendation 5: make-update-manifest.sh refuses a package url
# outside https://github.com/M7MMAD-OMAR/sbarhive/releases/download/ with exit 2
# and no output file. (The existing S-053c manifest test only uses good urls, so
# it stays green.)
# shellcheck disable=SC2016,SC2012
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
S="$REPO_ROOT/scripts/make-update-manifest.sh"
GOOD="https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0-beta.1"
DIST="$TMP/dist"; mkdir -p "$DIST"
printf 'rpm' > "$DIST/a.rpm"
mk() { jq -n --arg u "$1" '{assets:[{name:"a.rpm",url:$u,size:3}]}' > "$TMP/assets.json"; }
try() { # try "desc" WANT URL
  local d="$1" want="$2" out got
  mk "$3"
  out="$(mktemp -d -p "$TMP")"
  "$S" --dist "$DIST" beta 0.4.0-beta.1 "$TMP/assets.json" "$out/o.json" >/dev/null 2>&1
  got=$?
  if [ "$got" = "$want" ] && { [ "$want" != 2 ] || [ -z "$(ls -A "$out")" ]; }; then pass "$d"; else fail "$d (exit $got, want $want)"; fi
}
try "carried 5: a url under the release download prefix is accepted" 0 "$GOOD/a.rpm"
try "carried 5: another host is refused" 2 "https://example.com/M7MMAD-OMAR/sbarhive/releases/download/v1/a.rpm"
try "carried 5: plain http is refused" 2 "http://github.com/M7MMAD-OMAR/sbarhive/releases/download/v1/a.rpm"
try "carried 5: a look-alike host is refused" 2 "https://github.com.evil.example/M7MMAD-OMAR/sbarhive/releases/download/v1/a.rpm"
try "carried 5: another repository is refused" 2 "https://github.com/other/sbarhive/releases/download/v1/a.rpm"
try "carried 5: the api url is refused" 2 "https://api.github.com/repos/M7MMAD-OMAR/sbarhive/releases/assets/1"
try "carried 5: the download prefix without the trailing slash is refused" 2 "https://github.com/M7MMAD-OMAR/sbarhive/releases/download"
try "carried 5: the releases page is refused" 2 "https://github.com/M7MMAD-OMAR/sbarhive/releases/tag/v1"
try "carried 5: a url with a userinfo trick is refused" 2 "https://github.com@evil.example/M7MMAD-OMAR/sbarhive/releases/download/v1/a.rpm"
try "carried 5: an empty url is refused" 2 ""
finish
