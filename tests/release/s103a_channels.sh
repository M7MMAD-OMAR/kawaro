#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103a: scripts/channels/run-all.sh runs every channel script in isolation,
# keeps going when one fails, and writes the channel status table between the
# marker comments (golden). Channel contract: exit 0 success, 10 clean SKIP,
# anything else failure.
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
R="$REPO_ROOT/scripts/channels/run-all.sh"
G="$REPO_ROOT/tests/release/golden"
cd "$REPO_ROOT" || exit 1

CH="$TMP/channels"
mkdir -p "$CH"
printf '#!/usr/bin/env bash\necho "OK alpha $* "\nexit 0\n' > "$CH/alpha.sh"
printf '#!/usr/bin/env bash\necho "boom"\nexit 3\n' > "$CH/bad.sh"
printf '#!/usr/bin/env bash\necho "SKIP mid: MID_SECRET not configured"\nexit 10\n' > "$CH/mid.sh"
printf '#!/usr/bin/env bash\necho "OK zeta $*"\nexit 0\n' > "$CH/zeta.sh"
# Not channels: the helper and the runner itself must never be executed.
printf '#!/usr/bin/env bash\necho "HELPER RAN"\nexit 99\n' > "$CH/lib.sh"
printf '#!/usr/bin/env bash\necho "RUNNER RAN"\nexit 99\n' > "$CH/run-all.sh"

DOC="$TMP/status.md"
cp "$G/channel-status.md" "$DOC.golden"
sed -e '/channels:start/,/channels:end/c\<!-- channels:start -->\nold\n<!-- channels:end -->' "$G/channel-status.md" > "$DOC"

OUT="$TMP/out.txt"
bash "$R" v1.2.3 --channels-dir "$CH" --status-file "$DOC" --date 2026-10-30 > "$OUT" 2>&1
RC=$?
check "a1 a failing channel makes the run exit 1" test "$RC" -eq 1
check "a2 channels after the failing one still ran" grep -q '^OK zeta ' "$OUT"
check "a3 the SKIP line is passed through exactly" grep -qx 'SKIP mid: MID_SECRET not configured' "$OUT"
check "a4 helper and runner are not executed" bash -c "! grep -q 'RAN' '$OUT'"
check "a5 status table equals the golden file" diff -u "$G/channel-status.md" "$DOC"

# Dry run passes --dry-run to the scripts and does not touch the default doc.
bash "$R" v1.2.3 --dry-run --channels-dir "$CH" --date 2026-10-30 > "$OUT" 2>&1
check "a6 dry run forwards --dry-run" grep -q 'OK alpha v1.2.3 --dry-run' "$OUT"
check "a7 dry run table goes to stdout, states say dry-run" grep -q '| alpha | dry-run | 1.2.3 | 2026-10-30 |' "$OUT"

# Prerelease: every channel SKIPs, no script runs, exit 0.
cp "$DOC" "$TMP/s2.md"
bash "$R" v1.0.0-beta.1 --channels-dir "$CH" --status-file "$TMP/s2.md" --date 2026-10-30 > "$OUT" 2>&1
RC=$?
check "a8 prerelease exits 0" test "$RC" -eq 0
check "a9 prerelease prints the exact SKIP line per channel" grep -qx 'SKIP alpha: prerelease tag' "$OUT"
check "a10 prerelease runs no channel script" bash -c "! grep -q '^OK ' '$OUT'"

# Bad tags and bad usage exit 2.
expect_status "a11 tag without v is refused" 2 bash "$R" 1.2.3 --channels-dir "$CH"
expect_status "a12 empty tag list is refused" 2 bash "$R"
expect_status "a13 missing markers are refused" 2 bash "$R" v1.2.3 --channels-dir "$CH" --status-file "$TMP/nomarkers.md"
printf 'no markers here\n' > "$TMP/nomarkers.md"
expect_status "a13b doc without markers is refused" 2 bash "$R" v1.2.3 --channels-dir "$CH" --status-file "$TMP/nomarkers.md"

# --only runs one channel; an unknown name is a clean SKIP, not a failure.
bash "$R" v1.2.3 --only nosuch --channels-dir "$CH" --date 2026-10-30 > "$OUT" 2>&1
check "a14 unknown channel with --only is a SKIP" grep -qx 'SKIP nosuch: no channel script' "$OUT"

# Results directory and render-only (used by the workflow status job).
RES="$TMP/res"
bash "$R" v1.2.3 --only alpha --channels-dir "$CH" --results-dir "$RES" --date 2026-10-30 > /dev/null 2>&1
bash "$R" v1.2.3 --only mid --channels-dir "$CH" --results-dir "$RES" --date 2026-10-30 > /dev/null 2>&1
bash "$R" v1.2.3 --render-only --results-dir "$RES" > "$OUT" 2>&1
check "a15 render-only builds the table from collected results" grep -q '| mid | skipped | 1.2.3 | 2026-10-30 |' "$OUT"

# The real tree: the markers exist once, and the runner discovers by glob.
D="$REPO_ROOT/docs/distribution-status.md"
check "a16 distribution-status has one start marker" test "$(grep -c '^<!-- channels:start -->$' "$D")" -eq 1
check "a17 distribution-status has one end marker" test "$(grep -c '^<!-- channels:end -->$' "$D")" -eq 1
check "a18 run-all.sh globs scripts/channels/*.sh" grep -q 'channels_dir"/\*.sh' "$R"
check "a19 shellcheck passes on run-all.sh" bash -c "command -v shellcheck >/dev/null && shellcheck -S warning '$R' || ! command -v shellcheck"
finish
