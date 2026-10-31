#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103d: the COPR channel. Offline: copr-cli, cargo and every network tool
# are stubs that record their calls. Needs rpmspec and rpmbuild (light, no
# compile); rpmlint runs when installed.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

COPR="$REPO_ROOT/scripts/channels/copr.sh"
SPEC="$REPO_ROOT/packaging/copr/kawaro.spec"
REL="$REPO_ROOT/packaging/linux/kawaro.spec"
DOC="$REPO_ROOT/docs/distribution-channels/copr.md"
SECRET='sentinel-copr-config-9f3a'

check "S-103d copr.sh exists and is executable" test -x "$COPR"
check "S-103d copr.sh passes shellcheck" shellcheck -S warning -x "$COPR"
check "S-103d the COPR spec exists" test -f "$SPEC"

# The spec parses and carries the injected version.
parsed="$(rpmspec -P --define 'kawaro_version 9.8.7' "$SPEC" 2>/dev/null)"
check "S-103d rpmspec parses the spec with the version" \
  bash -c 'grep -qx "Version:        9.8.7" <<<"$1"' _ "$parsed"
check "S-103d Release is 1 with the dist tag" grep -qE '^Release:[[:space:]]+1%\{\?dist\}$' "$SPEC"
check "S-103d the build is locked and offline" \
  bash -c 'grep -q -- "--locked" "$1" && grep -q -- "--offline" "$1"' _ "$SPEC"
check "S-103d the files come from stage.sh, like the release RPM" grep -q 'packaging/linux/stage.sh' "$SPEC"

# Same payload as the release RPM: file list, weak deps and scriptlets.
check "S-103d %files equals the release RPM's" \
  bash -c 'diff <(sed -n "/^%files/,\$p" "$1" | grep -v "^$" | sort) <(sed -n "/^%files/,\$p" "$2" | grep -v "^$" | sort)' _ "$SPEC" "$REL"
check "S-103d Recommends and Suggests equal the release RPM's" \
  bash -c 'diff <(grep -E "^(Recommends|Suggests):" "$1") <(grep -E "^(Recommends|Suggests):" "$2")' _ "$SPEC" "$REL"
check "S-103d %post equals the release RPM's" \
  grep -qxF 'systemctl --global enable kawaro.service >/dev/null 2>&1 || :' "$SPEC"
check "S-103d %preun disables only on removal" \
  bash -c 'sed -n "/^%preun/,/^fi/p" "$1" | grep -q "\"\$1\" -eq 0"' _ "$SPEC"

if command -v rpmlint >/dev/null 2>&1; then
  check "S-103d rpmlint reports 0 errors on the spec" \
    bash -c '! rpmlint "$1" 2>&1 | grep -E "(^|[ :])E:" ' _ "$SPEC"
else
  echo "SKIP S-103d rpmlint is not installed (CI installs it)"
fi

# Stubs.
BIN="$TMP/bin"; mkdir -p "$BIN"
cat > "$BIN/copr-cli" <<'STUB'
#!/usr/bin/env bash
echo "copr-cli $*" >> "$STUB_LOG"
cfg=""; [ "$1" = "--config" ] && { cfg="$2"; shift 2; }
echo "config-present=$([ -f "$cfg" ] && echo yes || echo no) path=$cfg" >> "$STUB_LOG"
[ -f "$cfg" ] && echo "config-content=$(cat "$cfg")" >> "$STUB_LOG"
case "$1" in
  build) echo "Created builds: 4242" ;;
  watch-build) [ -n "${STUB_FAIL:-}" ] && { echo "fedora-44-x86_64 failed"; exit 1; }; echo ok ;;
  status) [ -n "${STUB_FAIL:-}" ] && echo failed || echo succeeded ;;
esac
STUB
cat > "$BIN/cargo" <<'STUB'
#!/usr/bin/env bash
echo "cargo $*" >> "$STUB_LOG"
if [ "$1" = vendor ]; then d="${*: -1}"; mkdir -p "$d/stubcrate"; echo x > "$d/stubcrate/lib.rs"; fi
STUB
for t in curl wget git-lfs; do printf '#!/bin/sh\necho "%s $*" >> "$STUB_LOG"\n' "$t" > "$BIN/$t"; done
chmod +x "$BIN"/*

run() { # run LOGNAME ENV... -- args  -> sets OUT, RC
  local name="$1"; shift
  local envs=()
  while [ "$1" != "--" ]; do envs+=("$1"); shift; done
  shift
  : > "$TMP/$name.log"
  OUT="$(cd "$REPO_ROOT" && env "${envs[@]}" PATH="$BIN:$PATH" TMPDIR="$TMP" STUB_LOG="$TMP/$name.log" \
    COPR_SOURCE_REF=HEAD "$COPR" "$@" 2>&1)"
  RC=$?
}

# Dry run: renders, prints the plan, touches nothing external.
run dry COPR_CONFIG="$SECRET" COPR_PROJECT=OWNER/kawaro -- v9.8.7 --dry-run
check "S-103d dry run exits 0" test "$RC" -eq 0
check "S-103d dry run prints the plan line" grep -qxF 'DRYRUN copr: would publish 9.8.7' <<<"$OUT"
for c in fedora-43-x86_64 fedora-44-x86_64 fedora-45-x86_64; do
  check "S-103d dry run lists $c" grep -q -- "$c" <<<"$OUT"
done
check "S-103d dry run made no external call" test ! -s "$TMP/dry.log"
check "S-103d dry run prints no secret" bash -c '! grep -q "$1" <<<"$2"' _ "$SECRET" "$OUT"

# Clean SKIP.
run skip COPR_PROJECT=OWNER/kawaro -- v9.8.7
check "S-103d no config: exit 10" test "$RC" -eq 10
check "S-103d no config: one SKIP line naming the variable" \
  test "$(grep -c '^SKIP copr: COPR_CONFIG not configured$' <<<"$OUT")" -eq 1
run skip2 COPR_CONFIG="$SECRET" -- v9.8.7
check "S-103d no project: exit 10 naming COPR_PROJECT" \
  bash -c '[ "$1" -eq 10 ] && grep -qx "SKIP copr: COPR_PROJECT not configured" <<<"$2"' _ "$RC" "$OUT"
run pre COPR_CONFIG="$SECRET" COPR_PROJECT=OWNER/kawaro -- v9.8.7-beta.1
check "S-103d prerelease: exit 10" test "$RC" -eq 10
check "S-103d prerelease: SKIP line" grep -qxF 'SKIP copr: prerelease tag' <<<"$OUT"
run bad COPR_CONFIG="$SECRET" COPR_PROJECT=OWNER/kawaro -- 9.8
check "S-103d malformed tag is a failure, not a skip" bash -c '[ "$1" -ne 0 ] && [ "$1" -ne 10 ]' _ "$RC"

# Real path with stubs.
run ok COPR_CONFIG="$SECRET" COPR_PROJECT=OWNER/kawaro -- v9.8.7
check "S-103d stubbed build exits 0" test "$RC" -eq 0
check "S-103d the build targets exactly the three chroots of S-016" \
  bash -c 'l="$(grep "copr-cli .*build" "$1")"; for c in fedora-43-x86_64 fedora-44-x86_64 fedora-45-x86_64; do grep -q -- "-r $c" <<<"$l" || exit 1; done; [ "$(grep -o -- "-r " <<<"$l" | wc -l)" -eq 3 ] && grep -q "OWNER/kawaro" <<<"$l"' _ "$TMP/ok.log"
check "S-103d the config file existed during the call" grep -q 'config-present=yes' "$TMP/ok.log"
check "S-103d the config file is gone afterwards" \
  bash -c 'p="$(sed -n "s/.*path=//p" "$1" | head -1)"; [ -n "$p" ] && [ ! -e "$p" ]' _ "$TMP/ok.log"
check "S-103d no work dir is left behind" bash -c '! ls -d "$1"/kawaro-copr.* >/dev/null 2>&1' _ "$TMP"
check "S-103d vendoring uses a dir that cannot collide with the in tree vendor/" \
  grep -q 'cargo vendor --locked vendor-crates' "$COPR"
check "S-103d output carries no secret" bash -c '! grep -q "$1" <<<"$2"' _ "$SECRET" "$OUT"

run fail COPR_CONFIG="$SECRET" COPR_PROJECT=OWNER/kawaro STUB_FAIL=1 -- v9.8.7
check "S-103d a failed chroot fails the job (not 0, not 10)" bash -c '[ "$1" -ne 0 ] && [ "$1" -ne 10 ]' _ "$RC"
check "S-103d the config file is gone also after a failure" \
  bash -c 'p="$(sed -n "s/.*path=//p" "$1" | head -1)"; [ -n "$p" ] && [ ! -e "$p" ]' _ "$TMP/fail.log"
check "S-103d no work dir is left behind after a failure" bash -c '! ls -d "$1"/kawaro-copr.* >/dev/null 2>&1' _ "$TMP"
check "S-103d failure output carries no secret" bash -c '! grep -q "$1" <<<"$2"' _ "$SECRET" "$OUT"

# Docs.
check "S-103d the doc shows the user command" grep -qF 'sudo dnf copr enable ACCOUNT/kawaro' "$DOC"
check "S-103d the doc names the secrets, never values" \
  bash -c 'grep -q COPR_CONFIG "$1" && grep -q COPR_PROJECT "$1"' _ "$DOC"

# Privacy and dash lint over every file this part owns.
LONG_DASH="$(printf '\342\200\224\|\342\200\223')"
for f in "$COPR" "$SPEC" "$DOC"; do
  check "S-103d no long dash in $(basename "$f")" bash -c '! grep -q "$2" "$1"' _ "$f" "$LONG_DASH"
  check "S-103d no email, home path or key block in $(basename "$f")" \
    bash -c '! grep -qE "[A-Za-z0-9._-]+@[A-Za-z0-9.-]+\.[a-z]{2,}|/home/|BEGIN [A-Z ]*PRIVATE KEY" "$1"' _ "$f"
done
finish
