#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-083 AC1 to AC5 and AC7: the Windows signing pipeline in
# .github/workflows/packages.yml, the extended scripts/windows/sign-local.sh,
# the SmartScreen guide, the download page note and the CHANGELOG line. Light:
# no compile, no container; the sign, timestamp and verify round trip is the
# heavy tests/windows/test_s083_sign.sh (throwaway certificate, local RFC 3161
# timestamp authority).
#
# Contract (DECISION marks a gap in the story; D-items are in the story file):
#   sign-local.sh CERT_SOURCE RELEASE_ZIP [OUT_DIR]
#   sign-local.sh CERT_SOURCE --sign FILE...
#   sign-local.sh CERT_SOURCE --verify FILE...
#   - CERT_SOURCE is `test` (self-signed throwaway leaf in KAWARO_WIN_SIGN_DIR,
#     created on demand) or `pkcs12:PATH` (PKCS#12 bundle, passphrase from the
#     environment variable KAWARO_WIN_SIGN_PASS).
#   - Refusals before any tool check: a wrong argument count (2), an unknown
#     CERT_SOURCE (2), a missing or empty pkcs12 file (2), KAWARO_WIN_SIGN_PASS
#     unset for pkcs12 (2), an empty KAWARO_WIN_SIGN_TS (2, every signature
#     carries an RFC 3161 timestamp, so "no timestamp server" is never
#     allowed). Nothing is written on a refusal and the message names the
#     missing input.
#   - AC1 packages.yml windows job: environment release-signing; three S-083
#     steps (sign the executables, sign the MSI, verify the signatures) whose
#     run texts (a) skip with `::notice::` and exit 0 when GITHUB_REF_TYPE is
#     not tag (workflow_dispatch), (b) fail loudly with `::error::` and exit 1
#     on a tag build with no certificate in the secrets, (c) materialise the
#     PKCS#12 from WINDOWS_SIGNING_PFX (base64) under umask 077 with a trap
#     that removes it, and (d) run sign-local.sh pkcs12:... inside
#     registry.fedoraproject.org/fedora:44 after installing osslsigncode (the
#     same toolchain the dry run proves). Step order in the job:
#     build.sh < sign the executables < make-zip.sh < make-msi.sh <
#     sign the MSI < verify the signatures < upload-artifact, so the exes are
#     signed before make-zip.sh packs them and before make-msi.sh embeds them.
#   - AC2: signing precedes every checksum and manifest step: the exe sign
#     step runs before make-zip.sh (whose inner SHA256SUMS then hashes signed
#     bytes), the release job (needs windows) writes the published SHA256SUMS,
#     and update-manifest (needs release) never calls sign-local.sh or
#     osslsigncode (minisign stays the update signature).
#   - Secrets: WINDOWS_SIGNING_PFX and WINDOWS_SIGNING_PASS appear only in the
#     windows job and only as whole values under env:; UPDATE_SIGNING_KEY and
#     UPDATE_SIGNING_PASSWORD stay only in update-manifest.
#   - SignPath (added for free SignPath Foundation signing, keeps the S-083
#     pipeline as the fallback): on a tag build with SIGNPATH_API_TOKEN (whole
#     value under env:) the exes and the MSI go up as throwaway `unsigned-*`
#     artifacts and come back signed by signpath/github-action-submit-
#     signing-request (organization, project and signing policy from the
#     SIGNPATH_* repository variables, the token from env), installed over the
#     build output before make-zip.sh, make-msi.sh and the final upload, and
#     verified against the system trust store. Without the token the three
#     sign-local.sh steps sign as before; each SignPath step is gated on the
#     token and a tag build, and the step order check counts the `windows`
#     upload, not the throwaway ones.
#   - AC3: docs/windows-smartscreen.md and docs/windows-smartscreen.ar.md each
#     start with the language switch line, show the dialog before and after
#     signing (unknown publisher versus the publisher name), the clicks
#     "More info" and "Run anyway", the SHA256SUMS verification, and say that
#     EV does not remove the warning (never that it does).
#   - AC4: the guide has a per-release maintainer section: submit the MSI to
#     https://www.microsoft.com/wdsi/filesubmission, keep the submission
#     confirmation, expect the warning to fade as downloads accumulate on the
#     same certificate.
#   - AC5: the built download pages (site/download/index.html and
#     site/en/download/index.html) carry the two-line note in the Windows slab
#     (one <p class="download-note"> with one <br>) naming SmartScreen,
#     SHA256SUMS, "More info" and "Run anyway", and link the guide
#     (windows-smartscreen.md in English, windows-smartscreen.ar.md in Arabic).
#     The note never says "signed", never says the warning is gone and never
#     says to disable SmartScreen. The pages are the committed build output:
#     a build.py change that was not rebuilt is a failure here on purpose.
#   - AC7: exactly one CHANGELOG.md line for S-083, the [S-083] link form,
#     an English part and an Arabic part in guillemets, no long dash.
#   - Superseded S-055/S-053c assertions (D-items): the windows job now has
#     environment: release-signing and the WINDOWS_SIGNING_* pair, so
#     beta_workflow.sh and workflow.sh expect exactly that instead of "no
#     environment and no secrets in windows".
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
WF="$REPO_ROOT/.github/workflows/packages.yml"
SIGN="$REPO_ROOT/scripts/windows/sign-local.sh"
EN_GUIDE="$REPO_ROOT/docs/windows-smartscreen.md"
AR_GUIDE="$REPO_ROOT/docs/windows-smartscreen.ar.md"
CHANGELOG="$REPO_ROOT/CHANGELOG.md"

nodash() { ! LC_ALL=C grep -q -e "$(printf '\xe2\x80\x94')" -e "$(printf '\xe2\x80\x93')" "$1"; }
nodash_text() { ! LC_ALL=C grep -q -e "$(printf '\xe2\x80\x94')" -e "$(printf '\xe2\x80\x93')" <<<"$1"; }

# ---------------------------------------------------------------- AC1 (parse)
python3 -I - "$WF" > "$TMP/py.out" 2>&1 <<'PY'
import re, sys, yaml
wf = sys.argv[1]
doc = yaml.safe_load(open(wf).read())
res = []
def ok(name, cond):
    res.append(("PASS " if cond else "FAIL ") + name)
jobs = doc.get("jobs", {})
w = jobs.get("windows") or {}
def steps(j): return [s for s in (jobs.get(j) or {}).get("steps", []) if isinstance(s, dict)]
def runs(j): return [s.get("run", "") for s in steps(j) if s.get("run")]

ok("AC1 windows environment is release-signing", w.get("environment") == "release-signing")

ws = steps("windows")
def step_i(pred):
    for i, s in enumerate(ws):
        if pred(s):
            return i
    return -1
def has(s, *words):
    r = s.get("run", "")
    return all(x in r for x in words)

build_i = step_i(lambda s: has(s, "scripts/windows/build.sh"))
sign_exe_i = step_i(lambda s: has(s, "sign-local.sh", "--sign", "kawaro.exe", "kawarod.exe"))
zip_i = step_i(lambda s: has(s, "scripts/windows/make-zip.sh"))
msi_i = step_i(lambda s: has(s, "scripts/windows/make-msi.sh"))
sign_msi_i = step_i(lambda s: has(s, "sign-local.sh", "--sign", ".msi"))
ver_i = step_i(lambda s: has(s, "sign-local.sh", "--verify"))
# the release upload only: the SignPath throwaway `unsigned-*` uploads sit
# earlier in the job and must not end the step order
up_i = step_i(lambda s: str(s.get("uses", "")).startswith("actions/upload-artifact@")
              and (s.get("with") or {}).get("name") == "windows")
ok("AC1 the exe sign step exists", sign_exe_i >= 0)
ok("AC1 the MSI sign step exists", sign_msi_i >= 0)
ok("AC1 the verify step exists", ver_i >= 0)
ok("AC1 step order: build < sign exes < make-zip < make-msi < sign MSI < verify < upload",
   -1 not in (build_i, sign_exe_i, zip_i, msi_i, sign_msi_i, ver_i, up_i)
   and build_i < sign_exe_i < zip_i < msi_i < sign_msi_i < ver_i < up_i)

gate = [s for s in ws if "sign-local.sh" in s.get("run", "")]
ok("AC1 exactly three signing steps", len(gate) == 3)
for i, s in enumerate(gate):
    r = s.get("run", "")
    n = f"AC1 signing step {i + 1}"
    ok(f"{n} skips with a notice when not a tag build",
       "GITHUB_REF_TYPE" in r and "::notice::" in r and re.search(r"exit 0", r) is not None)
    ok(f"{n} fails loudly with no certificate on a tag build",
       "::error::" in r and re.search(r"exit 1", r) is not None
       and re.search(r"-z[^\n]*WINDOWS_SIGNING_PFX", r) is not None)
    ok(f"{n} runs sign-local.sh in the Fedora container",
       "registry.fedoraproject.org/fedora:44" in r and "osslsigncode" in r
       and "pkcs12:" in r and "sign-local.sh" in r)
    ok(f"{n} materialises the certificate under umask 077 with a trap",
       "umask 077" in r and re.search(r"trap[^\n]*rm[^\n]*cert\.pfx", r) is not None
       and "base64 -d" in r)

# SignPath path: two requests gated on the token and a tag build, each fed by
# a throwaway unsigned upload; the S-083 sign-local.sh steps stay the fallback
sp = [s for s in ws if str(s.get("uses", "")).startswith("signpath/github-action-submit-signing-request@")]
upsu = [(i, s) for i, s in enumerate(ws) if str(s.get("uses", "")).startswith("actions/upload-artifact@")
        and str((s.get("with") or {}).get("name", "")).startswith("unsigned-")]
ok("AC1 two SignPath steps sign the executables and the MSI", len(sp) == 2)
ok("AC1 two throwaway unsigned uploads feed the SignPath requests",
   sorted((u.get("with") or {}).get("name") for _, u in upsu)
   == ["unsigned-executables", "unsigned-msi"])
for i, s in enumerate(sp):
    cond = str(s.get("if", ""))
    wi = s.get("with") or {}
    n = f"AC1 SignPath step {i + 1}"
    ok(f"{n} runs only with the token on a tag build",
       "SIGNPATH_API_TOKEN" in cond and "tag" in cond)
    ok(f"{n} takes its token from env, never from secrets.",
       "env.SIGNPATH_API_TOKEN" in str(wi.get("api-token", "")))
    ok(f"{n} names the organization, project and signing policy variables",
       "vars.SIGNPATH_ORGANIZATION_ID" in str(wi.get("organization-id", ""))
       and "vars.SIGNPATH_PROJECT_SLUG" in str(wi.get("project-slug", ""))
       and "vars.SIGNPATH_SIGNING_POLICY" in str(wi.get("signing-policy-slug", "")))
    feed = [j for j, u in upsu
            if f"steps.{u.get('id')}.outputs.artifact-id" in str(wi.get("github-artifact-id", ""))]
    ok(f"{n} signs the artifact of a preceding unsigned upload",
       len(feed) == 1 and feed[0] < ws.index(s))
exe_up = [j for j, u in upsu if (u.get("with") or {}).get("name") == "unsigned-executables"]
msi_up = [j for j, u in upsu if (u.get("with") or {}).get("name") == "unsigned-msi"]
ok("AC1 the executables reach SignPath before make-zip.sh",
   len(exe_up) == 1 and exe_up[0] < zip_i)
ok("AC1 the MSI reaches SignPath after make-msi.sh and before the upload",
   len(msi_up) == 1 and msi_i < msi_up[0] < up_i)

ok("AC1 the exe sign step names the release exes",
   sign_exe_i >= 0 and "x86_64-pc-windows-gnu/release/kawaro.exe" in ws[sign_exe_i].get("run", "")
   and "kawarod.exe" in ws[sign_exe_i].get("run", ""))
ok("AC1 the MSI sign step names the built msi",
   sign_msi_i >= 0 and re.search(r"dist/kawaro-[^ ]*\.msi", ws[sign_msi_i].get("run", "")) is not None)
ok("AC1 the verify step verifies the exes and the msi",
   ver_i >= 0 and "kawaro.exe" in ws[ver_i].get("run", "")
   and "kawarod.exe" in ws[ver_i].get("run", "") and ".msi" in ws[ver_i].get("run", ""))

# secrets placement
others_no_win = yaml.dump({n: j for n, j in jobs.items() if n != "windows"})
win_dump = yaml.dump(w)
ok("AC1 WINDOWS_SIGNING_PFX and WINDOWS_SIGNING_PASS only in windows",
   "secrets.WINDOWS_SIGNING_PFX" in win_dump and "secrets.WINDOWS_SIGNING_PASS" in win_dump
   and "WINDOWS_SIGNING" not in others_no_win)
ok("AC1 UPDATE_SIGNING_KEY and UPDATE_SIGNING_PASSWORD stay only in update-manifest",
   "UPDATE_SIGNING" not in win_dump
   and "UPDATE_SIGNING_KEY" not in yaml.dump({n: j for n, j in jobs.items() if n != "update-manifest"}))
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
ok("AC1 every secrets. reference is a whole value under env: " + ",".join(bad), not bad)
ok("AC1 no ${{ inside any windows run text", not any("${{" in r for r in runs("windows")))

# ---------------------------------------------------------------- AC2
r = jobs.get("release") or {}
need = r.get("needs"); need = [need] if isinstance(need, str) else (need or [])
ok("AC2 release needs windows (its SHA256SUMS hashes the signed artifacts)", "windows" in need)
ok("AC2 update-manifest needs release", (jobs.get("update-manifest") or {}).get("needs") == "release")
ok("AC2 the release job writes SHA256SUMS after the download",
   any("sha256sum" in x and "SHA256SUMS" in x for x in runs("release")))
ok("AC2 the exe sign step precedes make-zip.sh (its inner SHA256SUMS hashes signed bytes)",
   0 <= sign_exe_i < zip_i)
ok("AC2 update-manifest never re-signs",
   not any("sign-local.sh" in x or "osslsigncode" in x for x in runs("update-manifest")))
ok("AC2 release never re-signs",
   not any("sign-local.sh" in x or "osslsigncode" in x for x in runs("release")))

print("\n".join(res))
PY
while IFS= read -r l; do
  case "$l" in FAIL*) fail "${l#FAIL }" ;; PASS*) pass "${l#PASS }" ;; *) echo "py: $l" >&2 ;; esac
done < "$TMP/py.out"

# ---------------------------------------------------------------- AC1 (shell)
check "AC1 sign-local.sh exists and is executable" test -x "$SIGN"
expect_status "AC1 no arguments exits 2" 2 "$SIGN"
out="$("$SIGN" 2>&1 || true)"
check "AC1 usage names CERT_SOURCE" grep -q 'CERT_SOURCE' <<<"$out"
check "AC1 usage names --sign" grep -q -- '--sign' <<<"$out"
check "AC1 usage names --verify" grep -q -- '--verify' <<<"$out"
echo x > "$TMP/f"
echo x > "$TMP/ok.pfx"
: > "$TMP/empty.pfx"
touch "$TMP/marker"
expect_status "AC1 unknown CERT_SOURCE exits 2" 2 env KAWARO_WIN_SIGN_TS=http://ts.example \
  KAWARO_WIN_SIGN_PASS=x "$SIGN" nope --sign "$TMP/f"
expect_status "AC1 missing pkcs12 certificate exits 2" 2 env KAWARO_WIN_SIGN_TS=http://ts.example \
  KAWARO_WIN_SIGN_PASS=x "$SIGN" "pkcs12:$TMP/missing.pfx" --sign "$TMP/f"
out="$(env KAWARO_WIN_SIGN_TS=http://ts.example KAWARO_WIN_SIGN_PASS=x \
  "$SIGN" "pkcs12:$TMP/missing.pfx" --sign "$TMP/f" 2>&1 || true)"
check "AC1 the missing certificate is named in the message" grep -qi certificate <<<"$out"
expect_status "AC1 empty pkcs12 certificate exits 2" 2 env KAWARO_WIN_SIGN_TS=http://ts.example \
  KAWARO_WIN_SIGN_PASS=x "$SIGN" "pkcs12:$TMP/empty.pfx" --sign "$TMP/f"
expect_status "AC1 pkcs12 without KAWARO_WIN_SIGN_PASS exits 2" 2 env -u KAWARO_WIN_SIGN_PASS \
  KAWARO_WIN_SIGN_TS=http://ts.example "$SIGN" "pkcs12:$TMP/ok.pfx" --sign "$TMP/f"
expect_status "AC1 empty KAWARO_WIN_SIGN_TS exits 2 (every signature is timestamped)" 2 \
  env KAWARO_WIN_SIGN_TS= KAWARO_WIN_SIGN_PASS=x "$SIGN" "pkcs12:$TMP/ok.pfx" --sign "$TMP/f"
check "AC1 a refusal writes nothing" test -z "$(find "$TMP" -newer "$TMP/marker" -type f)"

# ---------------------------------------------------------------- AC3, AC4
for g in "$EN_GUIDE" "$AR_GUIDE"; do
  n="$(basename "$g")"
  check "AC3 $n exists" test -s "$g"
  [ -s "$g" ] || continue
  check "AC3 $n starts with the language switch line" \
    bash -c 'head -1 "$1" | grep -qxF "[English](windows-smartscreen.md) | [العربية](windows-smartscreen.ar.md)"' _ "$g"
  for phrase in "Windows protected your PC" "Unknown publisher" "More info" "Run anyway" "SHA256SUMS"; do
    check "AC3 $n shows $phrase" grep -qF "$phrase" "$g"
  done
  check "AC3 $n shows the publisher name after signing" grep -qi 'publisher name' "$g"
  check "AC3 $n says EV does not remove the warning" \
    grep -Eiq 'EV (certificate )?(does not|doesn.t|no longer) (remove|bypass|clear)' "$g"
  check "AC3 $n never claims EV removes the warning" \
    bash -c '! grep -Eiq "EV (certificate )?(removes|bypasses|clears|eliminates)" "$1"' _ "$g"
  check "AC4 $n has the per-release maintainer section" grep -Eiq '^#+ .*maintainer' "$g"
  check "AC4 $n names the file submission portal" \
    grep -qF 'https://www.microsoft.com/wdsi/filesubmission' "$g"
  check "AC4 $n says to keep the submission confirmation" grep -Eiq 'confirmation' "$g"
  check "AC4 $n expects the warning to fade as downloads accumulate" grep -Eiq 'accumulat' "$g"
  check "AC3 $n has no long dash" nodash "$g"
done

# ---------------------------------------------------------------- AC5
for page in "$REPO_ROOT/site/download/index.html" "$REPO_ROOT/site/en/download/index.html"; do
  n="${page#"$REPO_ROOT"/}"
  check "AC5 $n exists (committed build output)" test -s "$page"
  [ -s "$page" ] || continue
  note="$(perl -ne 'print "$1\n" if /(<p class="download-note">.*?<\/p>)/' "$page" | head -1)"
  check "AC5 $n has the download note" test -n "$note"
  for phrase in "SmartScreen" "SHA256SUMS" "More info" "Run anyway"; do
    check "AC5 $n note names $phrase" grep -qF "$phrase" <<<"$note"
  done
  check "AC5 $n note is two lines (one <br>)" test "$(grep -o '<br>' <<<"$note" | wc -l)" -eq 1
  check "AC5 $n note links the guide" grep -q 'windows-smartscreen' <<<"$note"
  case "$page" in
    */en/*) check "AC5 $n note links the English guide" grep -q 'windows-smartscreen\.md' <<<"$note" ;;
    *)      check "AC5 $n note links the Arabic guide" grep -q 'windows-smartscreen\.ar\.md' <<<"$note" ;;
  esac
  check "AC5 $n note never claims the build is signed" bash -c '! grep -qi "signed" <<<"$1"' _ "$note"
  check "AC5 $n note never says the warning is gone" \
    bash -c '! grep -qi "no warning\|warning is gone\|no longer warns" <<<"$1"' _ "$note"
  check "AC5 $n note never says to disable SmartScreen" bash -c '! grep -qi disable <<<"$1"' _ "$note"
done
expect_status "AC5 design-lint is clean" 0 "$REPO_ROOT/scripts/design-lint.sh"

# ---------------------------------------------------------------- AC7
check "AC7 exactly one CHANGELOG line for S-083" test "$(grep -c 'S-083' "$CHANGELOG")" -eq 1
LINE="$(grep 'S-083' "$CHANGELOG" | head -1)"
check "AC7 the CHANGELOG line is a list item in the [S-083] link form" \
  grep -Eq '^- [A-Za-z].*\(\[S-083\]\(https://github.com/M7MMAD-OMAR/kawaro\)\)' <<<"$LINE"
check "AC7 the CHANGELOG line has an Arabic part in guillemets" \
  bash -c 'grep -q "$(printf "\xc2\xab\xd8")\|$(printf "\xc2\xab\xd9")" <<<"$1"' _ "$LINE"
check "AC7 the CHANGELOG line has no long dash" nodash_text "$LINE"

finish
