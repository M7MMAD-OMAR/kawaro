#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-112 AC7, AC8 (part): signed SHA256SUMS. Static scan of the workflow and the
# script; the only run is the no-secret path with stub gpg and gh, which must
# print SKIP, exit 0 and call neither. The signing path is never run here.
# The script reads GITHUB_REF_NAME and GITHUB_REPOSITORY from the environment
# and takes no argument.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

R="$REPO_ROOT"
WF="$R/.github/workflows/packages.yml"
SC="$R/scripts/release/sign-checksums.sh"
SKIPLINE='SKIP sign-checksums: APT_SIGNING_KEY not configured'

check "S-112 the test is part of run_all" grep -qw s112_signed_checksums "$R/tests/release/run_all.sh"
check "AC7 the script exists and is executable" test -x "$SC"
check "AC7 the script passes bash -n" bash -n "$SC"
if command -v shellcheck >/dev/null 2>&1; then
  check "AC7 the script passes shellcheck at warning level" shellcheck -S warning -x "$SC"
else
  echo "SKIP S-112 shellcheck is not installed"
fi

# Workflow scan (parse only, nothing is executed).
check "AC7 job sign-checksums: needs release, tag and repo guard, environment" python3 -I - "$WF" "$SC" <<'PY'
import os, sys, yaml
d = yaml.safe_load(open(sys.argv[1], encoding="utf-8"))
script = open(sys.argv[2], encoding="utf-8").read() if os.path.exists(sys.argv[2]) else ""
j = d["jobs"]["sign-checksums"]
n = j["needs"]
assert (n == "release") or ("release" in n), n
c = " ".join(str(j["if"]).split())
assert "startsWith(github.ref, 'refs/tags/v')" in c, c
assert "github.repository == 'M7MMAD-OMAR/kawaro'" in c, c
assert j["environment"] == "apt-publishing" or j["environment"].get("name") == "apt-publishing"
blob = yaml.safe_dump(j)
assert "scripts/release/sign-checksums.sh" in blob
assert "--clobber" in blob or "--clobber" in script
for name in ("SHA256SUMS.asc", "kawaro-archive.asc"):
    assert name in blob or name in script, name
assert "APT_SIGNING_KEY" in blob
assert "BEGIN PGP" not in blob and "trusted" not in blob.lower()
PY
check "AC7 no pull_request trigger, only push tags and dispatch" python3 -I - "$WF" <<'PY'
import sys, yaml
d = yaml.safe_load(open(sys.argv[1], encoding="utf-8"))
on = d.get(True, d.get("on"))
assert "pull_request" not in on and "pull_request_target" not in on, on
assert "sign-checksums" in d["jobs"]
PY
check "AC7 release job still checks exactly 4 assets and uploads no .asc" python3 -I - "$WF" <<'PY'
import sys, yaml
d = yaml.safe_load(open(sys.argv[1], encoding="utf-8"))
r = yaml.safe_dump(d["jobs"]["release"])
assert 'test "$(wc -l < SHA256SUMS)" -eq 4' in "\n".join(s.get("run", "") for s in d["jobs"]["release"]["steps"])
assert ".asc" not in r
assert "dist/*.rpm dist/*.deb dist/*.zip dist/*.msi dist/SHA256SUMS" in r
assert "sign-checksums" in d["jobs"]
PY
# Script scan.
if [ -f "$SC" ]; then
  for pat in 'mktemp -d' 'chmod 700' 'trap ' 'EXIT' 'SHA256SUMS.asc' 'kawaro-archive.asc' 'packaging/keys/kawaro-archive.asc' '--armor' "$SKIPLINE"; do
    check "AC7 script contains: $pat" grep -qF -- "$pat" "$SC"
  done
  check "AC7 script refuses a fingerprint mismatch" grep -qiE 'fingerprint|--with-colons' "$SC"
  check "AC7 script uses --clobber on upload (script or job)" bash -c 'grep -qF -- "--clobber" "$1" "$2"' _ "$SC" "$WF"
  check "AC7 script has no key block and no trusted word" bash -c '! grep -qiE "BEGIN PGP|trusted" "$1"' _ "$SC"
else
  fail "AC7 script contains the safety lines (script missing)"
fi

# The only run: the empty secret path, with stubs that record any call.
if [ -x "$SC" ]; then
  BIN="$TMP/bin"; mkdir -p "$BIN" "$TMP/run"
  for t in gpg gh curl; do printf '#!/bin/sh\necho "%s $*" >> "%s/calls"\nexit 99\n' "$t" "$TMP" > "$BIN/$t"; chmod +x "$BIN/$t"; done
  OUT="$(cd "$TMP/run" && env -i PATH="$BIN:/usr/bin:/bin" HOME="$TMP/run" APT_SIGNING_KEY="" \
    GITHUB_REF_NAME=v9.9.9 GITHUB_REPOSITORY=M7MMAD-OMAR/kawaro bash "$SC" 2>&1)"
  RC=$?
  check "AC7 empty secret: exit 0" test "$RC" -eq 0
  check "AC7 empty secret: prints the SKIP line" grep -qF "$SKIPLINE" <<<"$OUT"
  check "AC7 empty secret: gpg, gh and curl are never called" test ! -e "$TMP/calls"
else
  fail "AC7 empty secret path (script missing)"
fi

# AC8 part.
for f in "$SC" "$0" "$WF"; do
  [ -f "$f" ] && check "AC8 no long dash in $(basename "$f")" bash -c '! grep -qP "\x{2013}|\x{2014}" "$1"' _ "$f"
done
check "AC8 this test passes bash -n" bash -n "$0"
if command -v shellcheck >/dev/null 2>&1; then
  check "AC8 this test passes shellcheck" shellcheck -S warning -x "$0"
else
  echo "SKIP S-112 shellcheck is not installed"
fi
finish
