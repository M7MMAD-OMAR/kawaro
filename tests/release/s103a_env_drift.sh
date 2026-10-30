#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103a drift check: every variable a channel script requires (the ones it
# turns into "SKIP NAME: VAR not configured") must be in the workflow step
# env, mapped to the secret or variable of the same name; every script in
# scripts/channels/ must be a matrix entry; dry runs never SKIP for missing
# configuration.
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
W="$REPO_ROOT/.github/workflows/release-distribute.yml"
CH="$REPO_ROOT/scripts/channels"
cd "$REPO_ROOT" || exit 1

# Required variable names of one script: uppercase tokens on the lines that
# build the "not configured" decision.
required() {
  grep -E 'missing=|for v in |^SECRET=' "$1" \
    | grep -oE '\b[A-Z][A-Z0-9]*(_[A-Z0-9]+)+\b' | grep -vE '^(SKIP|NOTE)$' | sort -u
}

for s in "$CH"/*.sh; do
  n="$(basename "$s" .sh)"
  case "$n" in run-all | lib) continue ;; esac
  check "d1 $n is a matrix entry" python3 -I - "$W" "$n" <<'PY'
import sys, yaml
d = yaml.safe_load(open(sys.argv[1]))
m = [j for j in d["jobs"].values() if "strategy" in j][0]["strategy"]["matrix"]
names = set(m.get("channel", [])) | {i["channel"] for i in m.get("include", [])}
assert sys.argv[2] in names, sorted(names)
PY
  for v in $(required "$s"); do
    check "d2 $n: $v is in the env of a step that runs for $n, from the same name" python3 -I - "$W" "$v" "$n" <<'PY'
import sys, re, yaml
d = yaml.safe_load(open(sys.argv[1]))
job = [j for j in d["jobs"].values() if "strategy" in j][0]
v, ch = sys.argv[2], sys.argv[3]
def applies(step, ch):
    c = step.get("if")
    if not c:
        return True
    m = re.fullmatch(r"matrix\.channel (==|!=) '(\w+)'", c.strip())
    return (m.group(1) == "==") == (m.group(2) == ch) if m else False
env = {}
for st in job["steps"]:
    if applies(st, ch):
        env.update(st.get("env") or {})
assert v in env, v + " missing"
assert env[v] in ("${{ secrets.%s }}" % v, "${{ vars.%s }}" % v), env[v]
PY
  done
done

# Dry runs: DRYRUN line, a NOTE when unconfigured, never a SKIP for config.
# A fake git archive is not needed: dry runs only read dist/SHA256SUMS.
SUMS="$TMP/SHA256SUMS"
sed 's/_1\.2\.3-1_amd64/_1.2.3_amd64/' "$REPO_ROOT/tests/release/golden/winget-input/SHA256SUMS" > "$SUMS"
for n in scoop aur copr winget apt; do
  [ -f "$CH/$n.sh" ] || continue
  out="$(env -i PATH="$PATH" HOME="$TMP" KAWARO_DIST="$REPO_ROOT/tests/release/golden/winget-input" \
    SHA256SUMS_FILE="$SUMS" \
    bash "$CH/$n.sh" v1.2.3 --dry-run 2>&1)"
  check "d3 $n dry run unconfigured: DRYRUN would publish line" grep -q "^DRYRUN $n: would publish 1.2.3" <<<"$out"
  check "d3 $n dry run unconfigured: NOTE line" grep -q "^NOTE $n: .* not configured" <<<"$out"
  check "d3 $n dry run unconfigured: no SKIP line" bash -c '! grep -q "^SKIP" <<<"$1"' _ "$out"
done

# The tag is validated before any step uses inputs.tag as a ref.
check "d4 tag validation step precedes checkout" python3 -I - "$W" <<'PY'
import sys, yaml
d = yaml.safe_load(open(sys.argv[1]))
job = [j for j in d["jobs"].values() if "strategy" in j][0]
steps = job["steps"]
ci = next(i for i, s in enumerate(steps) if "checkout" in s.get("uses", ""))
vi = next(i for i, s in enumerate(steps) if "run" in s and "[0-9]+" in s["run"])
assert vi < ci
PY
check "d10 no object storage names remain in the workflow" bash -c '! grep -q "R2" "$1"' _ "$W"
check "d8 APT_SIGNING_KEY is only in a step that runs for apt alone" python3 -I - "$W" <<'PY'
import sys, re, yaml
d = yaml.safe_load(open(sys.argv[1]))
job = [j for j in d["jobs"].values() if "strategy" in j][0]
for st in job["steps"]:
    if "APT_SIGNING_KEY" in (st.get("env") or {}):
        assert st.get("if") == "matrix.channel == 'apt'", st.get("name")
text = open(sys.argv[1]).read()
assert text.count("secrets.APT_SIGNING_KEY") == 1
PY
# Documented names, optional ones included: every name in the secrets and
# variables columns of docs/distribution.md (both languages) must be in the
# env of a step for that channel, of the documented kind, and the workflow
# must not pass a name the table does not document.
for doc in distribution.md distribution.ar.md; do
  check "d9 $doc: documented kinds and names equal the workflow env" python3 -I - "$W" "$REPO_ROOT/docs/$doc" <<'PY'
import sys, re, yaml
d = yaml.safe_load(open(sys.argv[1]))
job = [j for j in d["jobs"].values() if "strategy" in j][0]
def applies(step, ch):
    c = step.get("if")
    if not c:
        return True
    m = re.fullmatch(r"matrix\.channel (==|!=) '(\w+)'", c.strip())
    return (m.group(1) == "==") == (m.group(2) == ch) if m else False
rows = {}
for l in open(sys.argv[2]):
    m = re.match(r"\|\s*(winget|scoop|copr|aur|apt)\s*\|([^|]*)\|([^|]*)\|", l)
    if m:
        rows[m.group(1)] = (re.findall(r"`([A-Z0-9_]+)`", m.group(2)), re.findall(r"`([A-Z0-9_]+)`", m.group(3)))
assert len(rows) == 5, rows.keys()
documented = set()
for ch, (secrets, variables) in rows.items():
    env = {}
    for st in job["steps"]:
        if applies(st, ch):
            env.update(st.get("env") or {})
    for n in secrets:
        assert env.get(n) == "${{ secrets.%s }}" % n, (ch, n, env.get(n))
    for n in variables:
        assert env.get(n) == "${{ vars.%s }}" % n, (ch, n, env.get(n))
    documented |= set(secrets) | set(variables)
for st in job["steps"]:
    for n, v in (st.get("env") or {}).items():
        if "secrets." in str(v) or "vars." in str(v):
            assert n in documented, "undocumented " + n
PY
done
# Tools: pinned installs only for the channel that needs them.
check "d5 komac install is verified by sha256 and only for winget" python3 -I - "$W" <<'PY'
import sys, yaml
d = yaml.safe_load(open(sys.argv[1]))
job = [j for j in d["jobs"].values() if "strategy" in j][0]
s = [s for s in job["steps"] if "komac" in s.get("name", "").lower()][0]
assert "matrix.channel == 'winget'" in s["if"]
assert "sha256sum -c" in s["run"]
assert len(s["env"]["KOMAC_SHA256"]) == 64
PY
check "d6 copr tools are hash pinned and only for copr" python3 -I - "$W" <<'PY'
import sys, yaml
d = yaml.safe_load(open(sys.argv[1]))
job = [j for j in d["jobs"].values() if "strategy" in j][0]
s = [s for s in job["steps"] if "copr" in s.get("name", "").lower()][0]
assert "matrix.channel == 'copr'" in s["if"]
assert "--require-hashes" in s["run"]
PY
check "d7 environments must be restricted to v* tags (documented)" grep -q 'v\*' "$REPO_ROOT/docs/distribution-channels/README.md"
finish
