#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-112 AC1 to AC4 and AC8 (part): AppStream metadata. Static: reads files,
# never touches the network. appstreamcli is optional (SKIP when missing).
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

R="$REPO_ROOT"
MI="$R/packaging/io.github.m7mmad_omar.Kawaro.metainfo.xml"
CHK="$R/tests/release/s112_appstream_check.py"
STAGE="$R/packaging/linux/stage.sh"
SPECS=("$R/packaging/linux/kawaro.spec" "$R/packaging/copr/kawaro.spec")
SHOTS=(hive-en-light.png hive-ar-dark.png)
ck() { python3 -I "$CHK" "$1" "$R" "$2" >/dev/null 2>&1; }
# mut_fails WHAT COPY: the real file passes WHAT and the mutated copy fails it.
mut_fails() { ck "$MI" "$1" && ! ck "$2" "$1"; }

check "S-112 the checker helper exists" test -f "$CHK"
check "S-112 the test is part of run_all" grep -qw s112_appstream "$R/tests/release/run_all.sh"

# AC1 icon.
check "AC1 metainfo has exactly one stock icon kawaro" ck "$MI" icon
check "AC1 design/brand/kawaro.svg exists" test -s "$R/design/brand/kawaro.svg"
check "AC1 design/brand/png/kawaro-128.png exists" test -s "$R/design/brand/png/kawaro-128.png"
check "AC1 stage.sh installs apps/kawaro.svg" grep -qE 'install .*kawaro\.svg.*apps/kawaro\.svg' "$STAGE"
check "AC1 stage.sh installs apps/kawaro.png" grep -qE 'apps/kawaro\.png' "$STAGE"
sed '/<icon /d' "$MI" > "$TMP/noicon.xml"
check "AC1 a copy without the icon fails the check" mut_fails icon "$TMP/noicon.xml"

# AC2 screenshots.
check "AC2 metainfo screenshots are well formed" ck "$MI" screenshots
for f in "${SHOTS[@]}"; do
  check "AC2 $f is a byte identical copy of the UI snapshot in packaging/screenshots" \
    cmp -s "$R/crates/ui/tests/snapshots/$f" "$R/packaging/screenshots/$f"
  check "AC2 $f is a byte identical copy in site/screenshots" \
    cmp -s "$R/crates/ui/tests/snapshots/$f" "$R/site/screenshots/$f"
  check "AC2 stage.sh installs $f under share/kawaro/screenshots" \
    bash -c 'grep -q "share/kawaro/screenshots" "$1" && { grep -q "$2" "$1" || grep -qE "packaging/screenshots/\*|for .* in .*screenshots" "$1"; }' _ "$STAGE" "$f"
done
for s in "${SPECS[@]}"; do
  check "AC2 $(basename "$(dirname "$s")") spec lists /usr/share/kawaro/screenshots in %files" \
    bash -c 'sed -n "/^%files/,\$p" "$1" | grep -q "^/usr/share/kawaro/screenshots"' _ "$s"
done
w="$(python3 -I -c 'import re,sys;print(re.search(r"width=\"(\d+)\"",open(sys.argv[1]).read()).group(1))' "$MI" 2>/dev/null || echo 0)"
sed "0,/width=\"$w\"/s//width=\"$((w + 1))\"/" "$MI" > "$TMP/badwidth.xml"
check "AC2 a copy with a wrong width fails the check" mut_fails screenshots "$TMP/badwidth.xml"
check "AC2 the metainfo really has a width to corrupt" test "$w" -gt 0

# AC3 releases.
check "AC3 every release has English and Arabic notes" ck "$MI" releases
check "AC3 there are at least 17 releases" test "$(grep -c '<release ' "$MI")" -ge 17
python3 -I - "$MI" "$TMP/oneless.xml" <<'PY'
import re, sys
s = open(sys.argv[1], encoding="utf-8").read()
m = re.search(r'\n\s*<p xml:lang="ar">.*?</p>', s[s.index("<releases>"):], re.S)
i = s.index("<releases>")
open(sys.argv[2], "w", encoding="utf-8").write(s[:i] + s[i:][:m.start()] + s[i:][m.end():] if m else s)
PY
check "AC3 a copy without one Arabic release paragraph fails" mut_fails releases "$TMP/oneless.xml"

# AC4 validation.
if command -v appstreamcli >/dev/null 2>&1; then
  check "AC4 appstreamcli validate --pedantic --no-net passes" appstreamcli validate --pedantic --no-net "$MI"
  sed 's#</summary>#</summry>#' "$MI" > "$TMP/broken.xml"
  check "AC4 a copy with a broken tag is rejected" bash -c 'appstreamcli validate --pedantic --no-net "$1" >/dev/null 2>&1 && ! appstreamcli validate --pedantic --no-net "$2" >/dev/null 2>&1' _ "$MI" "$TMP/broken.xml"
else
  echo "SKIP S-112 appstreamcli is not installed"
fi

# AC8 part: no long dash in the files this script owns or the story changes.
for f in "$MI" "$CHK" "$0" "${SPECS[@]}" "$STAGE"; do
  check "AC8 no long dash in $(basename "$f")" bash -c '! grep -qP "\x{2013}|\x{2014}" "$1"' _ "$f"
done
check "AC8 this test passes bash -n" bash -n "$0"
if command -v shellcheck >/dev/null 2>&1; then
  check "AC8 this test passes shellcheck" shellcheck -S warning -x "$0"
else
  echo "SKIP S-112 shellcheck is not installed"
fi
finish
