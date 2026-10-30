#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103a: release-distribute.yml is a reusable workflow called by the last job
# of packages.yml, pinned by full commit SHA, least privilege, no secret
# outside a whole env value.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
W="$REPO_ROOT/.github/workflows/release-distribute.yml"
P="$REPO_ROOT/.github/workflows/packages.yml"
cd "$REPO_ROOT" || exit 1

py() { python3 -I - "$@"; }
check "w1 release-distribute.yml exists" test -f "$W"
check "w2 it has workflow_call and workflow_dispatch with tag and dry_run" py "$W" <<'PY'
import sys, yaml
d = yaml.safe_load(open(sys.argv[1]))
on = d.get(True, d.get("on"))
for t in ("workflow_call", "workflow_dispatch"):
    assert t in on, t
    assert "tag" in on[t]["inputs"] and "dry_run" in on[t]["inputs"], t
PY
check "w3 packages.yml final job calls it after release and update-manifest" py "$P" <<'PY'
import sys, yaml
d = yaml.safe_load(open(sys.argv[1]))
j = [v for v in d["jobs"].values() if str(v.get("uses", "")).endswith("release-distribute.yml")]
assert len(j) == 1
n = j[0]["needs"]
assert "release" in n and "update-manifest" in n
assert j[0]["permissions"] == {"contents": "read"}
PY
check "w4 a matrix with the five channels, fail-fast off" py "$W" <<'PY'
import sys, yaml
d = yaml.safe_load(open(sys.argv[1]))
m = [j for j in d["jobs"].values() if "strategy" in j][0]["strategy"]
assert sorted(m["matrix"]["channel"]) == ["apt", "aur", "copr", "scoop", "winget"]
assert m["fail-fast"] is False
PY
check "w5 every uses: is pinned to 40 hex characters" bash -c "! grep -E '^\s*-?\s*uses:' '$W' | grep -vqE '@[0-9a-f]{40}( |\$)'"
check "w6 no secrets. outside a whole env value" bash -c "! grep -n 'secrets\.' '$W' | grep -vE '^[0-9]+:\s+[A-Z_]+: \\$\{\{ secrets\.[A-Z_]+ \}\}\s*\$'"
check "w7 top level permissions are empty" py "$W" <<'PY'
import sys, yaml
d = yaml.safe_load(open(sys.argv[1]))
assert d["permissions"] == {}
PY
check "w8 channel job uses a protected environment per channel" grep -q 'environment: \${{ matrix.channel }}-publishing' "$W"
check "w9 the CANONICAL_REPO guard is a variable, not a name" grep -q 'vars.CANONICAL_REPO == github.repository' "$W"
check "w10 the tag is validated before use" grep -q 'v\[0-9\]' "$W"
check "w11 no secret echoing or set -x in the workflow" bash -c "! grep -nE 'set -x|echo .*secrets' '$W'"
check "w12 the workflow never pushes to main" bash -c "! grep -nE 'git push|contents: write' '$W'"
finish
