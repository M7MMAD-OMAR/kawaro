#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-073: release asset names match what GitHub stores (`~` becomes `.`), and the
# update-manifest job runs. Same harness as beta_workflow.sh: PyYAML reads
# .github/workflows/packages.yml, the release job's run texts run in order
# against a fake gh in a throwaway workspace.
#
# Contract (DECISION marks a gap in the story):
#   - AC1: the release job's run texts, run for v0.4.0-beta.1 and v0.4.0 in a
#     workspace whose `scripts` is the repo's, exit 0; afterwards the download
#     folder holds no name with `~`, SHA256SUMS lists exactly the four packages
#     under their `.` names (unchanged for v0.4.0), `sha256sum -c` passes there,
#     and `gh release upload` names those four files plus SHA256SUMS. The file
#     contents survive the rename.
#   - AC2: update-manifest has runs-on ubuntu-24.04; in the concatenated run
#     texts of a job (step order), `scripts/release/github-names.sh dist`
#     appears before the first `make-update-manifest.sh` (update-manifest) and
#     before the first redirect into SHA256SUMS (release).
#   - AC3: scripts/release/github-names.sh is executable. Rename in place, each
#     `~` replaced by `.`, contents kept. "Prints each new name": DECISION, one
#     line per renamed file, either the base name or DIR/base name; a run with
#     nothing to rename prints nothing on stdout. A collision exits 2 and renames
#     nothing at all (also not the other, non colliding files in DIR). DECISION:
#     no `mv` remains in the run texts of release and update-manifest (the
#     rename is the script, not a hand written loop).
#   - AC4: one paragraph of docs/adr/0006-auto-update.md mentions GitHub, asset
#     names, `~`, a quoted dot (`.` or '.' or "."), a rename, the checksums (checksum or SHA256SUMS) and the
#     manifest.
#   - AC5: shellcheck clean on the script; one CHANGELOG.md line names S-073.
#     run_all.sh, design-lint and the other gates are the reviewer's.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
WF="$REPO_ROOT/.github/workflows/packages.yml"
SCRIPT="$REPO_ROOT/scripts/release/github-names.sh"
ADR="$REPO_ROOT/docs/adr/0006-auto-update.md"

# ---------------------------------------------------------------- AC2 (parse), AC4
python3 -I - "$WF" "$TMP" "$ADR" > "$TMP/py.out" 2>&1 <<'PY'
import os, re, sys, yaml
wf, tmp, adr = sys.argv[1], sys.argv[2], sys.argv[3]
doc = yaml.safe_load(open(wf).read())
res = []
def ok(name, cond):
    res.append(("PASS " if cond else "FAIL ") + name)
jobs = doc.get("jobs", {})
def steps(j): return [s for s in (jobs.get(j) or {}).get("steps", []) if isinstance(s, dict)]
def runs(j): return "\n".join(s.get("run", "") for s in steps(j) if s.get("run"))
CALL = re.compile(r"scripts/release/github-names\.sh\s+['\"]?dist['\"]?(\s|$|;|\))")
def first(rx, text):
    m = re.search(rx, text)
    return m.start() if m else -1

um = jobs.get("update-manifest") or {}
ok("AC2 update-manifest runs-on ubuntu-24.04", um.get("runs-on") == "ubuntu-24.04")
mt = runs("update-manifest")
c, m = first(CALL, mt), first(r"make-update-manifest\.sh", mt)
ok("AC2 update-manifest calls scripts/release/github-names.sh dist", c >= 0)
ok("AC2 update-manifest renames before make-update-manifest.sh", c >= 0 and m >= 0 and c < m)

rt = runs("release")
c, s = first(CALL, rt), first(r">\s*['\"]?(dist/)?SHA256SUMS", rt)
ok("AC2 release calls scripts/release/github-names.sh dist", c >= 0)
ok("AC2 release renames before the line that writes SHA256SUMS", c >= 0 and s >= 0 and c < s)

ok("AC3 no hand written mv loop in the release run texts", re.search(r"(^|[\s;&|(])mv\s", rt) is None)
ok("AC3 no hand written mv loop in the update-manifest run texts", re.search(r"(^|[\s;&|(])mv\s", mt) is None)

open(os.path.join(tmp, "release.sh"), "w").write(rt)

try:
    text = open(adr).read()
except OSError:
    text = ""
paras = [" ".join(p.split()) for p in re.split(r"\n\s*\n", text)]
def hit(p):
    l = p.lower()
    dot = any(q in p for q in ("`.`", "'.'", '"."'))
    return "github" in l and "asset" in l and "~" in p and dot and "renam" in l \
        and ("checksum" in l or "sha256sums" in l) and "manifest" in l
ok("AC4 ADR 0006 says GitHub stores ~ as . in asset names and the pipeline renames before checksums and the manifest",
   any(hit(p) for p in paras))
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
  "release view") exit 1 ;;
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

# ---------------------------------------------------------------- AC1: release job
sorted_words() { tr ' ' '\n' | grep -v '^$' | sort | tr '\n' ' '; }
if [ -s "$TMP/release.sh" ]; then
  for tag in v0.4.0-beta.1 v0.4.0; do
    ver="${tag#v}"; tilde="${ver//-/\~}"; dot="${ver//-/.}"
    w="$(mktemp -d -p "$TMP")"
    mkdir -p "$w/dist" "$w/docs"
    ln -s "$REPO_ROOT/scripts" "$w/scripts"
    echo notes > "$w/docs/release-notes-$ver.md"
    echo rpm > "$w/dist/kawaro-$tilde-1.x86_64.rpm"
    echo deb > "$w/dist/kawaro_${tilde}_amd64.deb"
    echo zip > "$w/dist/kawaro-$ver-windows-x64.zip"
    echo msi > "$w/dist/kawaro-$ver-windows-x64.msi"
    L="$TMP/rlog-$tag"; : > "$L"
    (cd "$w" && PATH="$FAKE:$PATH" FAKE_LOG="$L" GITHUB_REF_NAME="$tag" GITHUB_REF="refs/tags/$tag" \
      GITHUB_REPOSITORY=M7MMAD-OMAR/kawaro GH_TOKEN=dummy bash -e "$TMP/release.sh" >"$L.out" 2>&1)
    rc=$?
    rpm="kawaro-$dot-1.x86_64.rpm"; deb="kawaro_${dot}_amd64.deb"
    zip="kawaro-$ver-windows-x64.zip"; msi="kawaro-$ver-windows-x64.msi"
    four="$(printf '%s ' "$rpm" "$deb" "$zip" "$msi" | sorted_words)"
    five="$(printf '%s ' SHA256SUMS "$rpm" "$deb" "$zip" "$msi" | sorted_words)"
    check "AC1 $tag: release run texts exit 0" test "$rc" = 0
    check "AC1 $tag: no file name in dist contains ~" test -z "$(find "$w/dist" -name '*~*' 2>/dev/null)"
    check "AC1 $tag: dist holds $rpm with the rpm content" test "$(cat "$w/dist/$rpm" 2>/dev/null)" = rpm
    check "AC1 $tag: dist holds $deb with the deb content" test "$(cat "$w/dist/$deb" 2>/dev/null)" = deb
    got="$(awk '{print $2}' "$w/dist/SHA256SUMS" 2>/dev/null | sed 's/^\*//' | xargs -r -n1 basename | sorted_words)"
    check "AC1 $tag: SHA256SUMS names $rpm, $deb, the zip and the msi" test "$got" = "$four"
    expect_status "AC1 $tag: sha256sum -c SHA256SUMS passes in dist" 0 bash -c "cd '$w/dist' && sha256sum -c SHA256SUMS"
    up="$(grep '^gh release upload' "$L" | tr ' ' '\n' | grep -E '(\.rpm|\.deb|\.zip|\.msi|SHA256SUMS)$' | xargs -r -n1 basename | sorted_words)"
    check "AC1 $tag: gh release upload names $rpm, $deb, the zip, the msi and SHA256SUMS" test "$up" = "$five"
  done
else
  fail "AC1 no release run texts to execute"
fi

# ---------------------------------------------------------------- AC3: the script
run_names() { # run_names DIR -> $TMP/n.out, $TMP/n.err, echoes the exit code
  (cd "$TMP" && "$SCRIPT" "$1" >"$TMP/n.out" 2>"$TMP/n.err"); echo $?
}
printed() { # printed NAME: one stdout line is NAME or ends with /NAME
  grep -Eqx "(.*/)?$(printf '%s' "$1" | sed 's/[.[\*^$]/\\&/g')" "$TMP/n.out"
}
listing() { (cd "$1" && find . -type f -exec sha256sum {} + | sort); }

check "AC3 scripts/release/github-names.sh exists and is executable" test -x "$SCRIPT"

d="$TMP/d1"; mkdir -p "$d"; echo one > "$d/a~b.rpm"; echo two > "$d/x~y~z.deb"; echo three > "$d/c.zip"
rc="$(run_names "$d")"
check "AC3 a~b.rpm: exit 0" test "$rc" = 0
check "AC3 a~b.rpm becomes a.b.rpm with its content" test "$(cat "$d/a.b.rpm" 2>/dev/null)" = one
check "AC3 a~b.rpm is gone" test ! -e "$d/a~b.rpm"
check "AC3 every ~ is replaced: x~y~z.deb becomes x.y.z.deb" test "$(cat "$d/x.y.z.deb" 2>/dev/null)" = two
check "AC3 a name without ~ is left alone" test "$(cat "$d/c.zip" 2>/dev/null)" = three
check "AC3 the new name a.b.rpm is printed" printed a.b.rpm
check "AC3 the new name x.y.z.deb is printed" printed x.y.z.deb
check "AC3 only the two renamed files are printed" test "$(grep -c . "$TMP/n.out")" = 2
check "AC3 dist ends with three files" test "$(find "$d" -type f | wc -l)" = 3

d="$TMP/d2"; mkdir -p "$d"; echo p > "$d/kawaro-0.4.0-1.x86_64.rpm"; echo q > "$d/SHA256SUMS"
before="$(listing "$d")"
rc="$(run_names "$d")"
check "AC3 a dir with no ~: exit 0" test "$rc" = 0
check "AC3 a dir with no ~ is untouched" test "$(listing "$d")" = "$before"
check "AC3 a dir with no ~ prints nothing" test ! -s "$TMP/n.out"

d="$TMP/d3"; mkdir -p "$d"; echo new > "$d/a~b"; echo old > "$d/a.b"; echo other > "$d/c~d"
before="$(listing "$d")"
rc="$(run_names "$d")"
check "AC3 a~b next to an existing a.b exits 2" test "$rc" = 2
check "AC3 the existing a.b is not overwritten" test "$(cat "$d/a.b" 2>/dev/null)" = old
check "AC3 a~b is still there" test "$(cat "$d/a~b" 2>/dev/null)" = new
check "AC3 nothing renamed on a collision (c~d stays)" test "$(listing "$d")" = "$before"

# ---------------------------------------------------------------- AC5
if [ -f "$SCRIPT" ]; then
  expect_status "AC5 shellcheck clean on scripts/release/github-names.sh" 0 shellcheck "$SCRIPT"
else
  fail "AC5 shellcheck clean on scripts/release/github-names.sh (no script)"
fi
check "AC5 one CHANGELOG.md line names S-073" test "$(grep -c 'S-073' "$REPO_ROOT/CHANGELOG.md")" = 1

finish
