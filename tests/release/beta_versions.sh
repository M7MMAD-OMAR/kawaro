#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-055 AC1 (the repository state of the bump). These checks read the real
# repository, so they are tied to the release cut: when the workspace moves
# on past 0.4.5, update EXPECT below in the story that bumps it. Every other
# test (S-051, S-053c) rewrites or reads the version dynamically and does not
# care which version Cargo.toml holds.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT" || exit 1
EXPECT="0.4.5"
WS="$(awk '/^\[workspace.package\]/{p=1;next} /^\[/{p=0} p && /^version *=/{gsub(/"/,"",$3); print $3; exit}' Cargo.toml)"
check "AC1 Cargo.toml [workspace.package] version is $EXPECT (found $WS)" test "$WS" = "$EXPECT"
lock_ok() { # every kawaro-* package in Cargo.lock has the expected version; at least 5
  awk -v want="$EXPECT" '
    /^name = "kawaro-/ { n++; getline; gsub(/version = |"/, ""); if ($0 != want) bad++ }
    END { exit !(n >= 5 && bad == 0) }' Cargo.lock
}
check "AC1 Cargo.lock: every kawaro workspace crate is $EXPECT" lock_ok
check "AC1 metainfo has <release version=\"$EXPECT\">" \
  grep -q "<release version=\"$EXPECT\"" packaging/io.github.m7mmad_omar.Kawaro.metainfo.xml
check "AC1 the $EXPECT release is the first entry of the metainfo" \
  test "$(grep -m1 -o '<release version="[^"]*"' packaging/io.github.m7mmad_omar.Kawaro.metainfo.xml)" = "<release version=\"$EXPECT\""
check "AC1 the metainfo entry has a date" \
  grep -Eq "<release version=\"$EXPECT\" date=\"[0-9]{4}-[0-9]{2}-[0-9]{2}\"" packaging/io.github.m7mmad_omar.Kawaro.metainfo.xml
expect_status "AC1 check-versions.sh rejects a version that is not semver" 2 scripts/release/check-versions.sh foo --out "$TMP"
finish
