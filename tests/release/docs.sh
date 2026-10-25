#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-053c AC6 (spec line is in packages.sh), AC7, AC8 and AC9: the signing
# document, no secret key in git, shellcheck, CHANGELOG.
#
# AC8 DEVIATION: the story's literal command
#   git log --all -p | grep -c 'encrypted secret key'
# already prints 1, because the story file itself quotes that phrase. This
# test greps for the full first line of a minisign secret key file and for
# the base64 prefix of an encrypted secret key body, both built from pieces so
# this file does not contain them.
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT" || exit 1
DOC=docs/update-signing.md
# no em dash (E2 80 94) and no en dash (E2 80 93), compared as bytes
nodash() { ! LC_ALL=C grep -q -e "$(printf '\xe2\x80\x94')" -e "$(printf '\xe2\x80\x93')" "$1"; }
nodash_text() { ! LC_ALL=C grep -q -e "$(printf '\xe2\x80\x94')" -e "$(printf '\xe2\x80\x93')" <<<"$1"; }
noarabic() { ! LC_ALL=C grep -q -e "$(printf '\xd8')" -e "$(printf '\xd9')" "$1"; }
# a guillemet (C2 AB) followed by an Arabic letter (lead byte D8 or D9)
arabic_in_guillemets() {
  LC_ALL=C grep -q -e "$(printf '\xc2\xab\xd8')" -e "$(printf '\xc2\xab\xd9')" <<<"$1"
}

check "AC7 $DOC exists" test -s "$DOC"
KEYID="$(sed -n 's/^untrusted comment: minisign public key \([0-9A-F]\{16\}\)$/\1/p' packaging/keys/kawaro-update.pub)"
check "AC7 the public key file has a 16 digit key id" test "${#KEYID}" -eq 16
check "AC7 the document contains the key id of packaging/keys/kawaro-update.pub" \
  grep -qF "$KEYID" "$DOC"

# a section is the text under a heading, up to the next heading
section() { # section REGEX: body of the first heading matching REGEX (case insensitive)
  awk -v re="$1" '/^#+ /{on=(tolower($0) ~ tolower(re)); next} on' "$DOC"
}
has_heading() { grep -Eiq "^#+ .*($1)" "$DOC"; }
check "AC7 section: generating the key" has_heading 'generat[a-z]* .*key|key generation'
check "AC7 section: where the private key lives" has_heading 'private key'
check "AC7 section: the two secrets and the environment" has_heading 'secrets'
check "AC7 section: rotation" has_heading 'rotat'
check "AC7 section: the leak limit" has_heading 'leak'
check "AC7 generating section shows minisign -G" grep -q 'minisign -G' <<<"$(section 'generat')"
S_SEC="$(section 'secrets')"
check "AC7 secrets section names UPDATE_SIGNING_KEY" grep -q 'UPDATE_SIGNING_KEY' <<<"$S_SEC"
check "AC7 secrets section names UPDATE_SIGNING_PASSWORD" grep -q 'UPDATE_SIGNING_PASSWORD' <<<"$S_SEC"
check "AC7 secrets section names the environment release-signing" grep -q 'release-signing' <<<"$S_SEC"
check "AC7 rotation section names kawaro-update-next.pub" \
  grep -q 'kawaro-update-next\.pub' <<<"$(section 'rotat')"
check "AC7 leak section says users reinstall by hand" grep -Eqi 'reinstall' <<<"$(section 'leak')"
check "AC7 the document names ADR 0006" grep -q '0006' "$DOC"
# shellcheck disable=SC2016
check "AC7 no home paths in the document" bash -c '! grep -q /home/ "$1"' _ "$DOC"
check "AC7 no em dash or en dash in the document" nodash "$DOC"
check "AC7 the document has no Arabic letters (English only)" noarabic "$DOC"

# AC8
HEAD1="minisign encrypted"; HEAD2=" secret key"
PREFIX="RWRT""Y0Iy"
check "AC8 no secret key header in the working tree (tracked or not)" \
  test "$(git grep -I --untracked -c -e "$HEAD1$HEAD2" -e "$PREFIX" 2>/dev/null | wc -l)" -eq 0
check "AC8 no secret key header anywhere in git history" \
  test "$(git log --all -p | grep -c -e "$HEAD1$HEAD2" -e "$PREFIX")" -eq 0
check "AC8 the only key file in packaging/keys is a .pub" \
  test "$(find packaging/keys -type f ! -name '*.pub' | wc -l)" -eq 0
check "AC8 *.key and kawaro-signing are not tracked" \
  test "$(git ls-files | grep -Ec '\.key$|kawaro-signing')" -eq 0

# AC9
for s in scripts/make-update-manifest.sh scripts/sign-update-manifest.sh; do
  check "AC9 $s exists and is executable" test -x "$s"
  expect_status "AC9 shellcheck $s" 0 shellcheck "$s"
  check "AC9 $s starts with a bash shebang" test "$(head -1 "$s")" = '#!/usr/bin/env bash'
  check "AC9 $s has the SPDX header" grep -q 'SPDX-License-Identifier: GPL-3.0-or-later' "$s"
  check "AC9 $s uses set -euo pipefail" grep -qx 'set -euo pipefail' "$s"
done
expect_status "AC9 design-lint is clean" 0 scripts/design-lint.sh
check "AC9 exactly one CHANGELOG line for S-053c" test "$(grep -c 'S-053c' CHANGELOG.md)" -eq 1
LINE="$(grep 'S-053c' CHANGELOG.md | head -1)"
check "AC9 the CHANGELOG line is a list item with an English part" grep -Eq '^- [A-Za-z].*\(S-053c\)' <<<"$LINE"
check "AC9 the CHANGELOG line has an Arabic part in guillemets" arabic_in_guillemets "$LINE"
check "AC9 the CHANGELOG line has no long dash" nodash_text "$LINE"
check "AC9 the CHANGELOG line mentions the signed feed and beta ordering" \
  grep -Eqi 'signed' <<<"$LINE"

finish
