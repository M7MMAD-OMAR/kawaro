#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-053c AC4: the update-manifest job of .github/workflows/packages.yml.
#
# Contract (DECISION marks a gap in the story):
#   - job `update-manifest`: needs: release; if (a ${{ }} wrapper is allowed)
#     startsWith(github.ref, 'refs/tags/v') && github.repository == 'M7MMAD-OMAR/sbarhive';
#     environment: release-signing. No other job has an environment.
#   - UPDATE_SIGNING_KEY and UPDATE_SIGNING_PASSWORD appear only in this job,
#     and every `secrets.` reference in the file is the value of a key under
#     an `env:` mapping. No `${{` inside any run text of the job (values come
#     from env vars), no `set -x`, no `bash -x`, no echo of a secret.
#   - "$UPDATE_SIGNING_PASSWORD" is never named in a run text (the signing
#     script reads it from its environment). "$UPDATE_SIGNING_KEY" appears
#     only as `printf '%s' "$UPDATE_SIGNING_KEY" > FILE` in a run text that
#     also holds `umask 077`.
#   - the job has an actions/checkout step, installs minisign, and calls
#     scripts/make-update-manifest.sh and scripts/sign-update-manifest.sh.
#   - the job downloads the build artifact: an actions/download-artifact@ step
#     with `name: packages` (the manifest needs the local files). Some run text
#     calls `gh release view` with `--json assets` (asset names and urls come
#     from the real release, ADR 0006 section 2).
#   - DECISION: exactly one step's run text holds `gh release upload`; it is a
#     separate publish step (a single combined step would also satisfy AC4,
#     but the test needs a run text it can execute with a fake gh). It is the
#     publish step. It reads GITHUB_REF_NAME, expects stable.json, stable.json.minisig,
#     beta.json and beta.json.minisig to exist (anywhere; only base names are
#     compared), runs `gh release create updates ... --prerelease --latest=false`
#     only when `gh release view updates ...` fails, and uploads the right
#     files to the tag `updates` with --clobber. The test runs that run text
#     with a fake gh.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
WF="$REPO_ROOT/.github/workflows/packages.yml"

python3 -I - "$WF" "$TMP/publish.sh" > "$TMP/py.out" 2>&1 <<'PY'
import re, sys, yaml
wf, out = sys.argv[1], sys.argv[2]
raw = open(wf).read()
doc = yaml.safe_load(raw)
res = []
def ok(name, cond):
    res.append(("PASS " if cond else "FAIL ") + name)
jobs = doc.get("jobs", {})
job = jobs.get("update-manifest")
ok("AC4 job update-manifest exists", job is not None)
if job is None:
    print("\n".join(res)); sys.exit(0)
ok("AC4 needs: release", job.get("needs") == "release")
cond = re.sub(r"^\$\{\{\s*|\s*\}\}$", "", str(job.get("if", "")).strip())
ok("AC4 if is the tag and repository condition",
   cond == "startsWith(github.ref, 'refs/tags/v') && github.repository == 'M7MMAD-OMAR/sbarhive'")
ok("AC4 environment: release-signing", job.get("environment") == "release-signing")
ok("AC4 no other job has an environment",
   all("environment" not in j for n, j in jobs.items() if n != "update-manifest"))
others = yaml.dump({n: j for n, j in jobs.items() if n != "update-manifest"})
ok("AC4 the secrets appear only in this job", "UPDATE_SIGNING" not in others and "release-signing" not in others)
ok("AC4 the job uses both secrets", "secrets.UPDATE_SIGNING_KEY" in yaml.dump(job) and "secrets.UPDATE_SIGNING_PASSWORD" in yaml.dump(job))

bad = []
def walk(node, under_env, path):
    if isinstance(node, dict):
        for k, v in node.items():
            walk(v, under_env or k == "env", path + [str(k)])
    elif isinstance(node, list):
        for i, v in enumerate(node):
            walk(v, under_env, path + [str(i)])
    elif isinstance(node, str) and "secrets." in node:
        if not under_env or path[-1] == "run" or not re.fullmatch(r"\$\{\{\s*secrets\.\w+\s*\}\}", node.strip()):
            bad.append("/".join(path))
walk(doc, False, [])
ok("AC4 every secrets. reference is a whole value under env: " + ",".join(bad), not bad)

steps = job.get("steps", [])
runs = [s.get("run", "") for s in steps if isinstance(s, dict)]
alltext = "\n".join(runs)
ok("AC4 no ${{ inside any run text of the job", not any("${{" in r for r in runs))
ok("AC4 no set -x, xtrace or bash -x anywhere in the workflow",
   not re.search(r"set\s+-[a-z]*x|xtrace|bash\s+-[a-z]*x", raw))
ok("AC4 no run text names $UPDATE_SIGNING_PASSWORD", not any("UPDATE_SIGNING_PASSWORD" in r for r in runs))
keylines = [l.strip() for r in runs for l in r.splitlines() if "UPDATE_SIGNING_KEY" in l]
ok("AC4 the key is only printf-ed to a file",
   bool(keylines) and all(re.fullmatch(r"printf '%s' \"\$UPDATE_SIGNING_KEY\" > \S+", l) for l in keylines))
ok("AC4 umask 077 before the key file",
   any("UPDATE_SIGNING_KEY" in r and "umask 077" in r for r in runs))
ok("AC4 no echo of a secret", not any(re.search(r"echo[^\n]*UPDATE_SIGNING", r) for r in runs))
ok("AC4 the job checks out the repository",
   any(str(s.get("uses", "")).startswith("actions/checkout@") for s in steps))
ok("AC4 the job downloads the packages artifact",
   any(str(s.get("uses", "")).startswith("actions/download-artifact@") and (s.get("with") or {}).get("name") == "packages" for s in steps))
ok("AC4 the job reads the real asset list with gh release view --json assets",
   re.search(r"gh release view[^\n]*--json assets", alltext) is not None)
ok("AC4 the job installs minisign", re.search(r"apt-get install[^\n]*\bminisign\b", alltext) is not None
   or ("minisign" in alltext and "sha256sum" in alltext))
ok("AC4 the job calls make-update-manifest.sh for both channels",
   "scripts/make-update-manifest.sh" in alltext and "stable" in alltext and "beta" in alltext)
ok("AC4 the job calls sign-update-manifest.sh", "scripts/sign-update-manifest.sh" in alltext)
pub = [r for r in runs if "gh release upload" in r]
ok("AC4 exactly one step uploads with gh release upload", len(pub) == 1)
if len(pub) == 1:
    open(out, "w").write(pub[0])
    ok("AC4 the publish step uses --clobber", "--clobber" in pub[0])
    ok("AC4 the publish step creates the updates release", "gh release create" in pub[0])
print("\n".join(res))
PY
cat "$TMP/py.out" | while IFS= read -r l; do
  case "$l" in PASS*) pass "${l#PASS }" ;; FAIL*) fail "${l#FAIL }" ;; *) echo "py: $l" >&2; fail "AC4 workflow reader: $l" ;; esac
done
# the while loop above runs in a subshell: recount from the file
P=$(grep -c '^PASS' "$TMP/py.out"); F=$(grep -vc '^PASS' "$TMP/py.out")
CHECKS=$((CHECKS + P + F)); FAILS=$((FAILS + F))

# CI wiring: the Rust half and run_all.sh need minisign, jq and rpm in CI
CI="$REPO_ROOT/.github/workflows/ci.yml"
APT="$(sed ':a;/\\$/{N;s/\\\n//;ba}' "$CI" | grep 'apt-get install')"
for pkg in minisign jq rpm; do
  check "AC3 ci.yml installs $pkg with apt-get" grep -Eqw "$pkg" <<<"$APT"
done
check "AC3 ci.yml runs tests/release/run_all.sh" grep -q 'tests/release/run_all.sh' "$CI"

# the rest runs the publish step, which must exist
if [ ! -s "$TMP/publish.sh" ]; then fail "AC4 no publish step to run"; finish; exit 1; fi

# run the publish step with a fake gh
FAKE="$TMP/fakebin"
mkdir -p "$FAKE"
cat > "$FAKE/gh" <<'FAKE_EOF'
#!/usr/bin/env bash
printf 'gh %s\n' "$*" >> "$FAKE_LOG"
if [ "$1 $2" = "release upload" ]; then
  for a in "$@"; do
    case "$a" in *.json|*.minisig) [ -f "$a" ] || { echo "gh: no such file $a" >&2; exit 1; } ;; esac
  done
fi
case "$1 $2" in
  "release view")
    case "$*" in
      *updates*) [ -n "${FAKE_UPDATES_EXISTS:-}" ] && { echo '{}'; exit 0; }; exit 1 ;;
    esac
    echo '{"assets":[]}' ;;
esac
exit 0
FAKE_EOF
chmod +x "$FAKE/gh"

run_publish() { # run_publish TAG [EXISTS]  -> log in $TMP/log-TAG-EXISTS
  local tag="$1" ex="${2:-}" w
  w="$(mktemp -d -p "$TMP")"
  for d in . feed out dist; do
    mkdir -p "$w/$d"
    for f in stable.json stable.json.minisig beta.json beta.json.minisig; do echo stub > "$w/$d/$f"; done
  done
  : > "$TMP/log-$tag-$ex"
  if [ -s "$TMP/publish.sh" ]; then
    (cd "$w" && PATH="$FAKE:$PATH" FAKE_LOG="$TMP/log-$tag-$ex" FAKE_UPDATES_EXISTS="$ex" \
      GITHUB_REF_NAME="$tag" GITHUB_REF="refs/tags/$tag" GITHUB_REPOSITORY=M7MMAD-OMAR/sbarhive \
      GH_TOKEN=dummy bash -e "$TMP/publish.sh" >"$TMP/log-$tag-$ex.out" 2>&1)
  fi
  echo $? > "$TMP/rc-$tag-$ex"
}
uploaded() { # uploaded TAG [EXISTS]: sorted base names on the upload line
  grep '^gh release upload' "$TMP/log-$1-${2:-}" | tr ' ' '\n' | grep -E '\.(json|minisig)$' \
    | xargs -r -n1 basename | sort | tr '\n' ' '
}
run_publish v0.4.0
run_publish v0.4.0-beta.2
run_publish v0.4.0 1
check "AC4 publish step exits 0 for v0.4.0" test "$(cat "$TMP/rc-v0.4.0-")" = 0
check "AC4 publish step exits 0 for v0.4.0-beta.2" test "$(cat "$TMP/rc-v0.4.0-beta.2-")" = 0
check "AC4 v0.4.0 uploads stable, beta and both signatures" \
  test "$(uploaded v0.4.0)" = "beta.json beta.json.minisig stable.json stable.json.minisig "
check "AC4 v0.4.0-beta.2 uploads beta.json and its signature only" \
  test "$(uploaded v0.4.0-beta.2)" = "beta.json beta.json.minisig "
check "AC4 upload goes to the updates tag with --clobber" \
  grep -Eq '^gh release upload updates .*--clobber|^gh release upload --clobber updates ' "$TMP/log-v0.4.0-"
check "AC4 absent updates release is created with --prerelease --latest=false" \
  grep -Eq '^gh release create updates .*--prerelease.*--latest=false|^gh release create updates .*--latest=false.*--prerelease' "$TMP/log-v0.4.0-"
check "AC4 the release is created before the upload" \
  test "$(grep -n '^gh release create' "$TMP/log-v0.4.0-" | head -1 | cut -d: -f1)" -lt \
       "$(grep -n '^gh release upload' "$TMP/log-v0.4.0-" | head -1 | cut -d: -f1)"
check "AC4 an existing updates release is not created again" \
  test "$(grep -c '^gh release create' "$TMP/log-v0.4.0-1")" -eq 0
check "AC4 an existing updates release still gets the upload" \
  test "$(uploaded v0.4.0 1)" = "beta.json beta.json.minisig stable.json stable.json.minisig "
check "AC4 a beta tag never uploads stable.json" \
  bash -c '! grep -q stable "$1"' _ "$TMP/log-v0.4.0-beta.2-"

finish
