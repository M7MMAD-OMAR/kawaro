#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-055 AC2, AC3, AC8 and the S-053c carried recommendations: structure and
# behaviour of .github/workflows/packages.yml. PyYAML reads the structure; the
# run texts of the release job and the signing steps are executed with a fake
# gh and throwaway minisign keys.
#
# Contract (DECISION marks a gap in the story):
#   - `windows` job: runs-on ubuntu-22.04, timeout-minutes 45, Swatinem/rust-cache
#     (manager ruling 4), the mingw target installed, apt installs mingw-w64 and
#     msitools, runs scripts/windows/{build,make-zip,make-msi}.sh with CI=true (job env
#     or step env or an inline prefix), uploads artifact `windows` whose path lists
#     exactly one .zip and one .msi (two lines, no directory wildcard) (DECISION).
#   - `release` job: needs includes test and windows; if is exactly
#     startsWith(github.ref, 'refs/tags/v'); checks out the repo (the notes file lives
#     there); downloads `packages` and `windows` into ONE folder (explicit names with the
#     same path, or pattern plus merge-multiple); all its run texts, run in order with a
#     fake gh, write SHA256SUMS (4 lines, base names, sha256sum -c clean) in that folder,
#     create the release only when `gh release view TAG` fails, with --prerelease iff the
#     tag has a `-`, `--notes-file docs/release-notes-VERSION.md` (VERSION without v),
#     and upload the four files plus SHA256SUMS with --clobber.
#   - `update-manifest` needs exactly `release` (S-053c test). DECISION: artifact
#     immutability means the MSI cannot be added to `packages` later, so this job also
#     downloads `windows` into the SAME folder it passes to --dist (carried item 6).
#   - carried: top-level permissions has no write; release and update-manifest have
#     contents: write; build and test have none. minisign -V -p
#     packaging/keys/sbarhive-update.pub runs in a step before the publish step (not inside
#     it: the S-053c publish test runs that step with stub files); a trap ... EXIT removes
#     the key file also on failure; each json is uploaded before its .minisig.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
WF="$REPO_ROOT/.github/workflows/packages.yml"

python3 -I - "$WF" "$TMP" > "$TMP/py.out" 2>&1 <<'PY'
import fnmatch, os, re, sys, yaml
wf, tmp = sys.argv[1], sys.argv[2]
doc = yaml.safe_load(open(wf).read())
res = []
def ok(name, cond):
    res.append(("PASS " if cond else "FAIL ") + name)
jobs = doc.get("jobs", {})
def steps(j): return [s for s in (jobs.get(j) or {}).get("steps", []) if isinstance(s, dict)]
def runs(j): return [s.get("run", "") for s in steps(j) if s.get("run")]
def cond(x): return re.sub(r"^\$\{\{\s*|\s*\}\}$", "", str(x).strip())
def truthy(v): return v is True or str(v).lower() == "true"

# ---- AC2
w = jobs.get("windows")
ok("AC2 job windows exists", w is not None)
w = w or {}
ok("AC2 windows runs-on ubuntu-22.04", w.get("runs-on") == "ubuntu-22.04")
wt = "\n".join(runs("windows"))
ok("AC2 windows installs mingw-w64 with apt", re.search(r"apt-get install[^\n]*\bmingw-w64\b", wt) is not None)
ok("AC2 windows installs msitools with apt", re.search(r"apt-get install[^\n]*\bmsitools\b", wt) is not None)
ok("AC2 windows adds the x86_64-pc-windows-gnu target",
   "x86_64-pc-windows-gnu" in yaml.dump(w.get("steps", [])))
for sc in ("build", "make-zip", "make-msi"):
    ok(f"AC2 windows runs scripts/windows/{sc}.sh", f"scripts/windows/{sc}.sh" in wt)
    hits = [s for s in steps("windows") if f"scripts/windows/{sc}.sh" in s.get("run", "")]
    cienv = truthy((w.get("env") or {}).get("CI")) or any(truthy((s.get("env") or {}).get("CI")) for s in hits) \
        or any(re.search(rf"\bCI=true\b[^\n]*scripts/windows/{sc}\.sh", s.get("run", "")) for s in hits)
    ok(f"AC2 {sc}.sh runs with CI=true", bool(hits) and cienv)
ok("AC2 windows has the 45 minute timeout", w.get("timeout-minutes") == 45)
ok("AC2 windows uses Swatinem/rust-cache", any(str(s.get("uses", "")).startswith("Swatinem/rust-cache@") for s in steps("windows")))
ok("AC2 windows checks out the repository", any(str(s.get("uses", "")).startswith("actions/checkout@") for s in steps("windows")))
ups = [s for s in steps("windows") if str(s.get("uses", "")).startswith("actions/upload-artifact@")]
ok("AC2 windows has one upload-artifact step", len(ups) == 1)
if len(ups) == 1:
    wi = ups[0].get("with") or {}
    ok("AC2 artifact is named windows", wi.get("name") == "windows")
    lines = [l.strip() for l in str(wi.get("path", "")).splitlines() if l.strip()]
    ok("AC2 artifact path is exactly one .zip and one .msi", len(lines) == 2
       and sum(l.endswith(".zip") for l in lines) == 1 and sum(l.endswith(".msi") for l in lines) == 1)
ok("AC2 windows has no environment", "environment" not in w)
ok("AC2 windows has no secrets. reference", "secrets." not in yaml.dump(w))

# ---- AC3
r = jobs.get("release") or {}
need = r.get("needs"); need = [need] if isinstance(need, str) else (need or [])
ok("AC3 release needs test and windows", "test" in need and "windows" in need)
ok("AC3 release if stays startsWith(github.ref, 'refs/tags/v')", cond(r.get("if", "")) == "startsWith(github.ref, 'refs/tags/v')")
ok("AC3 update-manifest needs exactly release", (jobs.get("update-manifest") or {}).get("needs") == "release")
ok("AC3 release checks out the repository (notes file)", any(str(s.get("uses", "")).startswith("actions/checkout@") for s in steps("release")))
def downloads(j):
    out = []
    for s in steps(j):
        if str(s.get("uses", "")).startswith("actions/download-artifact@"):
            out.append(s.get("with") or {})
    return out
def covered(j):
    names, paths = set(), set()
    for wi in downloads(j):
        paths.add(str(wi.get("path", ".")))
        if wi.get("name"): names.add(wi["name"])
        elif wi.get("pattern"):
            for n in ("packages", "windows"):
                if fnmatch.fnmatch(n, str(wi["pattern"])): names.add(n)
    return names, paths
rn, rp = covered("release")
ok("AC3 release downloads packages and windows", {"packages", "windows"} <= rn)
ok("AC3 release downloads both into ONE folder", len(rp) == 1)
rfolder = sorted(rp)[0] if len(rp) == 1 else "dist"
mn, mp = covered("update-manifest")
ok("AC3 update-manifest downloads packages and windows (the MSI must be in dist)", {"packages", "windows"} <= mn)
ok("AC3 update-manifest downloads into ONE folder", len(mp) == 1)
mt = "\n".join(runs("update-manifest"))
mf = sorted(mp)[0] if len(mp) == 1 else "dist"
ok("AC3 update-manifest passes that folder as --dist", re.search(r"make-update-manifest\.sh\s+--dist\s+['\"]?" + re.escape(mf) + r"['\"]?\s", mt) is not None)
ok("AC3 update-manifest checks out the repository (public key)", any(str(s.get("uses", "")).startswith("actions/checkout@") for s in steps("update-manifest")))
open(os.path.join(tmp, "release-folder"), "w").write(rfolder)

# ---- AC8: secrets only in update-manifest
others = yaml.dump({n: j for n, j in jobs.items() if n != "update-manifest"})
ok("AC8 UPDATE_SIGNING_KEY only in update-manifest", "UPDATE_SIGNING_KEY" not in others)
ok("AC8 UPDATE_SIGNING_PASSWORD only in update-manifest", "UPDATE_SIGNING_PASSWORD" not in others)
ok("AC8 release-signing only in update-manifest", "release-signing" not in others)
ok("AC8 job names are build, test, windows, release, update-manifest",
   sorted(jobs) == sorted(["build", "test", "windows", "release", "update-manifest"]))
ok("AC5 test matrix still has 5 images", len((jobs.get("test", {}).get("strategy", {}).get("matrix", {}).get("include", []))) == 5)

# ---- carried 4: permissions
def writes(p):
    return isinstance(p, dict) and any(str(v) == "write" for v in p.values())
top = doc.get("permissions")
ok("carried 4: top-level permissions has no write", not writes(top) and top != "write-all")
ok("carried 4: release has contents: write", (r.get("permissions") or {}).get("contents") == "write")
ok("carried 4: update-manifest has contents: write", (jobs.get("update-manifest", {}).get("permissions") or {}).get("contents") == "write")
for j in ("build", "test"):
    jp = jobs.get(j, {}).get("permissions")
    ok(f"carried 4: {j} has no write permission", not writes(jp) and jp != "write-all")

# ---- carried 1, 3: signing steps
us = steps("update-manifest")
sign_i = [i for i, s in enumerate(us) if "sign-update-manifest.sh" in s.get("run", "")]
pub_i = [i for i, s in enumerate(us) if "gh release upload" in s.get("run", "")]
ok("carried 1: one signing step and one publish step", len(sign_i) >= 1 and len(pub_i) == 1)
ver_i = [i for i, s in enumerate(us) if re.search(r"minisign\s+-V\s+-p\s+packaging/keys/sbarhive-update\.pub\s+-m\b", s.get("run", ""))]
ok("carried 1: minisign -V -p packaging/keys/sbarhive-update.pub -m runs in a step", len(ver_i) >= 1)
ok("carried 1: verification runs before the publish step", bool(ver_i) and bool(pub_i) and min(ver_i) < pub_i[0])
ok("carried 1: verification covers every manifest (a loop or both files)",
   any(re.search(r"for\s+\w+\s+in\s+feed/\*\.json", us[i].get("run", "")) or ("beta.json" in us[i].get("run", "") and "stable.json" in us[i].get("run", "")) for i in ver_i))
keystep = [s.get("run", "") for s in us if "UPDATE_SIGNING_KEY" in s.get("run", "")]
ok("carried 3: the key file is removed by trap ... EXIT",
   len(keystep) == 1 and re.search(r"trap\s+.*rm\s+-f?[^\n]*update\.key[^\n]*\s+EXIT", keystep[0]) is not None)
ok("carried 3: the trap is set before the key is written",
   len(keystep) == 1 and keystep[0].find("trap") != -1 and keystep[0].find("trap") < keystep[0].find("UPDATE_SIGNING_KEY"))
if sign_i and pub_i:
    first = min([i for i, s in enumerate(us) if "UPDATE_SIGNING_KEY" in s.get("run", "")] + [sign_i[0]])
    open(os.path.join(tmp, "sign.sh"), "w").write("\n".join(us[i].get("run", "") for i in range(first, pub_i[0])))
    open(os.path.join(tmp, "publish.sh"), "w").write(us[pub_i[0]].get("run", ""))
# release run texts in order
open(os.path.join(tmp, "release.sh"), "w").write("\n".join(runs("release")))
print("\n".join(res))
PY
P=$(grep -c '^PASS' "$TMP/py.out"); F=$(grep -vc '^PASS' "$TMP/py.out")
while IFS= read -r l; do case "$l" in FAIL*) echo "$l" >&2 ;; PASS*) echo "$l" ;; *) echo "py: $l" >&2 ;; esac; done < "$TMP/py.out"
CHECKS=$((CHECKS + P + F)); FAILS=$((FAILS + F))

# ---------------------------------------------------------------- fake gh
FAKE="$TMP/fakebin"; mkdir -p "$FAKE"
cat > "$FAKE/gh" <<'FAKE_EOF'
#!/usr/bin/env bash
printf 'gh %s\n' "$*" >> "$FAKE_LOG"
case "$1 $2" in
  "release view")
    case "$*" in
      *updates*) [ -n "${FAKE_UPDATES_EXISTS:-}" ] && { echo '{}'; exit 0; }; exit 1 ;;
      *) [ -n "${FAKE_RELEASE_EXISTS:-}" ] && { echo '{"assets":[]}'; exit 0; }; exit 1 ;;
    esac ;;
  "release create")
    prev=""
    for a in "$@"; do
      [ "$prev" = "--notes-file" ] && { [ -f "$a" ] || { echo "gh: no notes file $a" >&2; exit 1; }; }
      prev="$a"
    done ;;
  "release upload")
    shift 3
    skip=""
    for a in "$@"; do
      if [ -n "$skip" ]; then skip=""; continue; fi
      case "$a" in -R | --repo) skip=1 ;; -*) ;; *) [ -e "$a" ] || { echo "gh: no such file $a" >&2; exit 1; } ;; esac
    done ;;
esac
exit 0
FAKE_EOF
chmod +x "$FAKE/gh"

# ---------------------------------------------------------------- release job
if [ -s "$TMP/release.sh" ]; then
  FOLDER="$(cat "$TMP/release-folder")"
  run_release() { # run_release TAG [EXISTS]
    local tag="$1" ex="${2:-}" ver w
    ver="${tag#v}"
    w="$(mktemp -d -p "$TMP")"
    mkdir -p "$w/$FOLDER" "$w/docs"
    echo notes > "$w/docs/release-notes-$ver.md"
    local mp="${ver//-/\~}"
    echo rpm > "$w/$FOLDER/sbarhive-$mp-1.x86_64.rpm"
    echo deb > "$w/$FOLDER/sbarhive_${mp}_amd64.deb"
    echo zip > "$w/$FOLDER/sbarhive-$ver-windows-x64.zip"
    echo msi > "$w/$FOLDER/sbarhive-$ver-windows-x64.msi"
    : > "$TMP/rlog-$tag-$ex"
    (cd "$w" && PATH="$FAKE:$PATH" FAKE_LOG="$TMP/rlog-$tag-$ex" FAKE_RELEASE_EXISTS="$ex" GITHUB_REF_NAME="$tag" \
      GITHUB_REF="refs/tags/$tag" GITHUB_REPOSITORY=M7MMAD-OMAR/sbarhive GH_TOKEN=dummy \
      bash -e "$TMP/release.sh" >"$TMP/rlog-$tag-$ex.out" 2>&1)
    echo $? > "$TMP/rrc-$tag-$ex"
    echo "$w" > "$TMP/rw-$tag-$ex"
  }
  run_release v0.4.0-beta.1
  run_release v0.4.0
  run_release v0.4.0-beta.1 1
  for tag in v0.4.0-beta.1 v0.4.0; do
    ver="${tag#v}"; L="$TMP/rlog-$tag-"; W="$(cat "$TMP/rw-$tag-")"
    check "AC3 $tag: release run texts exit 0" test "$(cat "$TMP/rrc-$tag-")" = 0
    check "AC3 $tag: SHA256SUMS has exactly 4 lines" test "$(wc -l < "$W/$FOLDER/SHA256SUMS" 2>/dev/null)" = 4
    expect_status "AC3 $tag: sha256sum -c SHA256SUMS in the folder" 0 bash -c "cd '$W/$FOLDER' && sha256sum -c SHA256SUMS"
    check "AC3 $tag: SHA256SUMS holds base names of rpm, deb, zip, msi only" \
      test "$(awk '{print $2}' "$W/$FOLDER/SHA256SUMS" 2>/dev/null | sed 's/.*\.//' | sort | tr '\n' ' ')" = "deb msi rpm zip "
    check "AC3 $tag: release view runs before create" \
      test "$(grep -n '^gh release view' "$L" | head -1 | cut -d: -f1)" -lt "$(grep -n '^gh release create' "$L" | head -1 | cut -d: -f1)"
    check "AC3 $tag: absent release is created once" test "$(grep -c '^gh release create' "$L")" = 1
    check "AC3 $tag: create names the tag" grep -Eq "^gh release create (.* )?$tag( |\$)" "$L"
    check "AC3 $tag: create uses --notes-file docs/release-notes-$ver.md" grep -q -- "--notes-file docs/release-notes-$ver.md" "$L"
    check "AC3 $tag: create happens before the upload" \
      test "$(grep -n '^gh release create' "$L" | head -1 | cut -d: -f1)" -lt "$(grep -n '^gh release upload' "$L" | head -1 | cut -d: -f1)"
    check "AC3 $tag: upload has --clobber" grep -Eq '^gh release upload .*--clobber' "$L"
    names="$(grep '^gh release upload' "$L" | tr ' ' '\n' | grep -E '(\.rpm|\.deb|\.zip|\.msi|SHA256SUMS)$' | xargs -r -n1 basename | sort | tr '\n' ' ')"
    mp="${ver//-/\~}"
    want="$(printf '%s\n' SHA256SUMS "sbarhive-$mp-1.x86_64.rpm" "sbarhive_${mp}_amd64.deb" "sbarhive-$ver-windows-x64.zip" "sbarhive-$ver-windows-x64.msi" | sort | tr '\n' ' ')"
    check "AC3 $tag: upload names the four files and SHA256SUMS" test "$names" = "$want"
  done
  check "AC3 beta tag: create has --prerelease" grep -Eq '^gh release create .*--prerelease' "$TMP/rlog-v0.4.0-beta.1-"
  check "AC3 plain tag: create has no --prerelease" bash -c '! grep "^gh release create" "$1" | grep -q -- --prerelease' _ "$TMP/rlog-v0.4.0-"
  check "AC3 existing release is not created again" test "$(grep -c '^gh release create' "$TMP/rlog-v0.4.0-beta.1-1")" = 0
  check "AC3 existing release still gets the upload with --clobber" grep -Eq '^gh release upload .*--clobber' "$TMP/rlog-v0.4.0-beta.1-1"
else
  fail "AC3 no release run texts to execute"
fi

# ---------------------------------------------------------------- signing steps
if [ -s "$TMP/sign.sh" ]; then
  minisign -G -W -f -p "$TMP/good.pub" -s "$TMP/good.key" >/dev/null 2>&1
  minisign -G -W -f -p "$TMP/other.pub" -s "$TMP/other.key" >/dev/null 2>&1
  run_sign() { # run_sign NAME PUB KEYFILE_CONTENT_FILE
    local n="$1" pub="$2" key="$3" w="$TMP/ws-$1"
    mkdir -p "$w/packaging/keys" "$w/feed" "$w/rt"
    ln -s "$REPO_ROOT/scripts" "$w/scripts"
    cp "$pub" "$w/packaging/keys/sbarhive-update.pub"
    echo '{"schema":1,"channel":"beta","version":"0.4.0-beta.1"}' > "$w/feed/beta.json"
    echo '{"schema":1,"channel":"stable","version":"0.4.0"}' > "$w/feed/stable.json"
    (cd "$w" && RUNNER_TEMP="$w/rt" UPDATE_SIGNING_KEY="$(cat "$key")" UPDATE_SIGNING_PASSWORD="" \
      GITHUB_REF_NAME=v0.4.0 PATH="$FAKE:$PATH" FAKE_LOG="$TMP/slog-$n" \
      bash -e "$TMP/sign.sh" >"$TMP/sout-$n" 2>&1)
    echo $? > "$TMP/src-$n"
  }
  run_sign good "$TMP/good.pub" "$TMP/good.key"
  check "carried 1: signing steps succeed with a matching key" test "$(cat "$TMP/src-good")" = 0
  check "carried 1: both manifests got a .minisig" test -s "$TMP/ws-good/feed/beta.json.minisig" -a -s "$TMP/ws-good/feed/stable.json.minisig"
  check "carried 3: no key file is left after success" test -z "$(find "$TMP/ws-good/rt" -type f 2>/dev/null)"
  run_sign wrongpub "$TMP/other.pub" "$TMP/good.key"
  check "carried 1: a signature that does not verify against the committed key fails the steps" test "$(cat "$TMP/src-wrongpub")" != 0
  check "carried 3: no key file is left after a verification failure" test -z "$(find "$TMP/ws-wrongpub/rt" -type f 2>/dev/null)"
  echo 'not a key' > "$TMP/garbage.key"
  run_sign garbage "$TMP/good.pub" "$TMP/garbage.key"
  check "carried 3: a signing failure fails the steps" test "$(cat "$TMP/src-garbage")" != 0
  check "carried 3: the trap removes the key file also on failure" test -z "$(find "$TMP/ws-garbage/rt" -type f 2>/dev/null)"
  check "carried 1: a failed signing leaves no .minisig" test -z "$(find "$TMP/ws-garbage/feed" -name '*.minisig')"
else
  fail "carried 1/3: no signing steps to execute"
fi

# ---------------------------------------------------------------- publish order
if [ -s "$TMP/publish.sh" ]; then
  for tag in v0.4.0 v0.4.0-beta.2; do
    w="$(mktemp -d -p "$TMP")"; mkdir -p "$w/feed"
    for f in stable.json stable.json.minisig beta.json beta.json.minisig; do echo stub > "$w/feed/$f"; done
    : > "$TMP/plog-$tag"
    (cd "$w" && PATH="$FAKE:$PATH" FAKE_LOG="$TMP/plog-$tag" GITHUB_REF_NAME="$tag" GITHUB_REPOSITORY=M7MMAD-OMAR/sbarhive \
      GH_TOKEN=dummy bash -e "$TMP/publish.sh" >/dev/null 2>&1)
    mapfile -t seq < <(grep '^gh release upload' "$TMP/plog-$tag" | tr ' ' '\n' | grep -E '\.(json|minisig)$' | xargs -r -n1 basename)
    pos() { local i; for i in "${!seq[@]}"; do [ "${seq[$i]}" = "$1" ] && { echo "$i"; return; }; done; echo -1; }
    for ch in beta stable; do
      [ "$ch" = stable ] && [ "$tag" != v0.4.0 ] && continue
      pj="$(pos "$ch.json")"; ps="$(pos "$ch.json.minisig")"
      check "carried 2: $tag uploads $ch.json before $ch.json.minisig" test "$pj" -ge 0 -a "$ps" -gt "$pj"
    done
  done
else
  fail "carried 2: no publish step to execute"
fi

finish
