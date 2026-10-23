#!/usr/bin/env bash
# S-044b, light tests: no build, no VM, no network. AC8 and AC12 parse
# .github/workflows/ci.yml (python3 with PyYAML); AC9 runs make-iso.sh on a
# fake build output in a temp dir and reads the ISO back with isoinfo; AC10
# and AC11 read run-tests.cmd and the README; AC12 runs check-dlls.sh on bad
# input and reads docs/engines.md.
#   bash tests/windows/test_ci_iso.sh
# The CI jobs themselves only run on a push to main or a pull request, so
# their first green run is checked live in S-052, not here.
# The sh -c bodies below take their values as arguments, on purpose.
# shellcheck disable=SC2016
set -u
# shellcheck source=tests/windows/lib.sh
. "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT" || exit 2
trap drop_fakes EXIT

CI_YML=.github/workflows/ci.yml
BASE="$(git merge-base main HEAD)"
LONG_DASH="$(printf '\342\200\224\|\342\200\223')"

# ci_query CHECK: runs one named python check on ci.yml; prints a reason and
# exits 1 when it does not hold.
ci_query () {
  local base_yml
  base_yml="$(mktemp)"
  git show "$BASE:$CI_YML" > "$base_yml" 2>/dev/null
  python3 -I - "$1" "$CI_YML" "$base_yml" <<'PY'
import sys, yaml
check, path, base_path = sys.argv[1], sys.argv[2], sys.argv[3]
def load(p):
    with open(p, encoding="utf-8") as f:
        return yaml.safe_load(f)
try:
    doc = load(path)
except Exception as e:
    print(f"ci.yml does not parse: {e}"); sys.exit(1)
jobs = doc.get("jobs") or {}
def steps(job):
    return (jobs.get(job) or {}).get("steps") or []
def runs(job):
    return [str(s.get("run", "")) for s in steps(job)]
def first(job, pred):
    for i, s in enumerate(steps(job)):
        if pred(s):
            return i
    return -1
def say(ok, why):
    if not ok:
        print(why)
    sys.exit(0 if ok else 1)

if check == "check_unchanged":
    base = load(base_path)
    say((base.get("jobs") or {}).get("check") == jobs.get("check"),
        "job check differs from the merge base")
if check == "has_cross":
    say("windows-cross" in jobs, f"no job windows-cross (jobs: {sorted(jobs)})")
if check == "has_native":
    say("windows-native" in jobs, f"no job windows-native (jobs: {sorted(jobs)})")
if check == "cross_runs_on":
    say((jobs.get("windows-cross") or {}).get("runs-on") == "ubuntu-24.04",
        "windows-cross runs-on is not ubuntu-24.04")
if check == "native_runs_on":
    say((jobs.get("windows-native") or {}).get("runs-on") == "windows-latest",
        "windows-native runs-on is not windows-latest")

SCRIPTS = ["scripts/windows/check.sh", "scripts/windows/build.sh", "scripts/windows/check-dlls.sh"]
def is_mingw(s):
    r = str(s.get("run", ""))
    return "mingw-w64" in r and "install" in r
def is_target(s):
    r = str(s.get("run", ""))
    w = s.get("with") or {}
    return ("rustup target add" in r and "x86_64-pc-windows-gnu" in r) or \
        "x86_64-pc-windows-gnu" in str(w.get("targets", "")) or \
        "x86_64-pc-windows-gnu" in str(w.get("target", ""))
def runs_script(name):
    return lambda s: name in str(s.get("run", ""))

if check == "cross_installs_mingw":
    say(first("windows-cross", is_mingw) >= 0, "no step installs mingw-w64")
if check == "cross_adds_target":
    say(first("windows-cross", is_target) >= 0, "no step adds the x86_64-pc-windows-gnu target")
if check == "cross_runs_scripts":
    idx = {n: first("windows-cross", runs_script(n)) for n in SCRIPTS}
    missing = [n for n, i in idx.items() if i < 0]
    if missing:
        say(False, f"windows-cross does not run {missing}")
    setup = max(first("windows-cross", is_mingw), first("windows-cross", is_target))
    if min(idx.values()) < setup:
        say(False, "a script runs before mingw-w64 and the target are installed")
    say(idx["scripts/windows/check-dlls.sh"] > idx["scripts/windows/build.sh"],
        "check-dlls.sh does not run after build.sh")
if check == "cross_dll_arg":
    i = first("windows-cross", runs_script("scripts/windows/check-dlls.sh"))
    r = str(steps("windows-cross")[i].get("run", "")) if i >= 0 else ""
    say("x86_64-pc-windows-gnu/release/sbarhive.exe" in r,
        f"check-dlls.sh is not given x86_64-pc-windows-gnu/release/sbarhive.exe: [{r.strip()}]")

def target_dir_values():
    vals = []
    for scope in (doc, jobs.get("windows-cross") or {}):
        v = (scope.get("env") or {}).get("CARGO_TARGET_DIR")
        if v is not None:
            vals.append(("level", str(v)))
    for s in steps("windows-cross"):
        if any(n in str(s.get("run", "")) for n in SCRIPTS[1:]):
            v = (s.get("env") or {}).get("CARGO_TARGET_DIR")
            if v is not None:
                vals.append(("step", str(v)))
    return vals
if check == "cross_target_dir":
    vals = target_dir_values()
    if not vals:
        say(False, "windows-cross sets no CARGO_TARGET_DIR (workflow, job or the build and check-dlls steps)")
    bad = [v for _, v in vals if not ("github.workspace" in v or
           (not v.startswith("/") and not v.startswith("~") and not v.startswith("$HOME")))]
    say(not bad, f"CARGO_TARGET_DIR outside the workspace: {bad}")
if check == "cross_no_home":
    say("/home/sbarah" not in yaml.safe_dump(jobs.get("windows-cross") or {}),
        "windows-cross names /home/sbarah")

def is_choco(s):
    return "choco install ffmpeg -y" in str(s.get("run", ""))
TEST = "cargo test -p sbarhive-core -p sbarhive-engines -p sbarhive-cli"
def is_test(s):
    return TEST in " ".join(str(s.get("run", "")).split())
if check == "native_ffmpeg_then_test":
    c, t = first("windows-native", is_choco), first("windows-native", is_test)
    if c < 0:
        say(False, "windows-native has no 'choco install ffmpeg -y' step")
    if t < 0:
        say(False, f"windows-native has no '{TEST}' step")
    say(c < t, "choco install ffmpeg runs after cargo test")
if check == "native_no_office_gs":
    text = " ".join(runs("windows-native")).lower()
    bad = [w for w in ("libreoffice", "ghostscript", "gsview") if w in text]
    say(not bad, f"windows-native installs {bad}")
print(f"unknown check {check}"); sys.exit(1)
PY
  local rc=$?
  rm -f "$base_yml"
  return $rc
}

ci_check () {
  # ci_check "description" CHECK
  local why
  if why="$(ci_query "$2")"; then pass; else fail "$1: $why"; fi
}

ac8_ci_adds_windows_cross_and_windows_native_and_leaves_check_alone () {
  echo "== AC8: ci.yml"
  local diff removed added
  diff="$(git diff -U0 "$BASE" -- "$CI_YML")"
  removed="$(printf '%s\n' "$diff" | grep -E '^-' | grep -vE '^--- ')"
  added="$(printf '%s\n' "$diff" | grep -E '^\+' | grep -vcE '^\+\+\+ ')"
  assert_eq "AC8: no deleted line in ci.yml against the merge base" "$removed" ""
  check "AC8: ci.yml has added lines against the merge base (got $added)" test "$added" -gt 0
  ci_check "AC8: job check unchanged" check_unchanged
  ci_check "AC8: windows-cross exists" has_cross
  ci_check "AC8: windows-cross on ubuntu-24.04" cross_runs_on
  ci_check "AC8: windows-cross installs mingw-w64" cross_installs_mingw
  ci_check "AC8: windows-cross adds x86_64-pc-windows-gnu" cross_adds_target
  ci_check "AC8: windows-cross runs check.sh, build.sh, check-dlls.sh" cross_runs_scripts
  ci_check "AC8: check-dlls.sh reads the built sbarhive.exe" cross_dll_arg
  ci_check "AC8: windows-native exists" has_native
  ci_check "AC8: windows-native on windows-latest" native_runs_on
  ci_check "AC8: windows-native installs FFmpeg before cargo test" native_ffmpeg_then_test
  ci_check "AC8: windows-native installs neither LibreOffice nor Ghostscript" native_no_office_gs
}

# make_build_tree: a fake cross build output under $FSB/target, laid out as
# build.sh and `cargo test --no-run --target x86_64-pc-windows-gnu` leave it.
# Each file has distinct bytes. Sets TEST_EXES (base names).
make_build_tree () {
  local t="$FSB/target/x86_64-pc-windows-gnu" n
  mkdir -p "$t/release" "$t/debug/deps" "$FSB/target/debug/build"
  printf 'MZ release sbarhive\n' > "$t/release/sbarhive.exe"
  TEST_EXES=(sbarhive_core-0a1b2c3d4e5f6071.exe contracts-27b367aa4bb7975d.exe
             pdf_ghostscript-1122334455667788.exe single_instance-99aabbccddeeff00.exe
             sbarhive-4be04d6afe5036fa.exe)
  for n in "${TEST_EXES[@]}"; do printf 'MZ test %s\n' "$n" > "$t/debug/deps/$n"; done
  # Not test programs: must not land in tests/.
  printf 'dep info\n' > "$t/debug/deps/contracts-27b367aa4bb7975d.d"
  printf 'rlib\n' > "$t/debug/deps/libsbarhive_core-0a1b2c3d4e5f6071.rlib"
  printf 'host build script\n' > "$FSB/target/debug/build/build-script-build"
}

ac9_make_iso_builds_a_joliet_rock_ridge_iso_with_exes_sums_and_runner () {
  echo "== AC9: make-iso.sh on a fake build output"
  local real_iso="$WIN_TARGET_DEFAULT/sbarhive-win-tests.iso" real_before real_after
  real_before="$(stat -c '%Y %s' "$real_iso" 2>/dev/null || echo absent)"
  make_fakes
  if [ ! -x "$WIN/make-iso.sh" ]; then fail "AC9: missing or not executable $WIN/make-iso.sh"; drop_fakes; return; fi
  if [ ! -f "$WIN/run-tests.cmd" ]; then fail "AC9: missing $WIN/run-tests.cmd"; fi
  make_build_tree
  local iso="$FSB/target/sbarhive-win-tests.iso" rc list n sums want got exes
  CI='' CARGO_TARGET_DIR="$FSB/target" PATH="$FSB/bin:$PATH" timeout 120 "$WIN/make-iso.sh" \
    >"$FSB/log/stdout" 2>"$FSB/log/stderr"; rc=$?
  assert_eq "AC9: exit code" "$rc" "0"
  check "AC9: make-iso.sh builds nothing (no cargo call)" test ! -s "$FSB/log/cargo.log"
  if [ ! -f "$iso" ]; then
    fail "AC9: no ISO at \$CARGO_TARGET_DIR/sbarhive-win-tests.iso ($(tr '\n' ' ' < "$FSB/log/stderr"))"
    drop_fakes; return
  fi
  check "AC9: file says ISO 9660 ($(file -b "$iso"))" sh -c 'file -b "$1" | grep -q "ISO 9660"' _ "$iso"
  check "AC9: isoinfo -d reports Joliet (-J)" sh -c 'isoinfo -d -i "$1" | grep -q "^Joliet with UCS level [0-9]* found"' _ "$iso"
  check "AC9: isoinfo -d reports Rock Ridge (-R)" sh -c 'isoinfo -d -i "$1" | grep -q "^Rock Ridge signatures version [0-9]* found"' _ "$iso"
  list="$(isoinfo -J -f -i "$iso" 2>/dev/null)"
  check "AC9: lists /bin/sbarhive.exe" grep -qx /bin/sbarhive.exe <<<"$list"
  for n in "${TEST_EXES[@]}"; do
    check "AC9: lists /tests/$n" grep -qx "/tests/$n" <<<"$list"
  done
  check "AC9: lists /run-tests.cmd" grep -qx /run-tests.cmd <<<"$list"
  check "AC9: lists /SHA256SUMS" grep -qx /SHA256SUMS <<<"$list"
  check "AC9: no non-exe build file in /tests" sh -c '! grep -E "^/tests/" <<EOF | grep -qvE "\.exe$"
$1
EOF' _ "$list"
  exes="$(grep -ciE '\.exe$' <<<"$list")"
  assert_eq "AC9: .exe files on the ISO (sbarhive.exe and the test programs)" "$exes" "$(( ${#TEST_EXES[@]} + 1 ))"
  sums="$(isoinfo -J -x /SHA256SUMS -i "$iso" 2>/dev/null)"
  assert_eq "AC9: SHA256SUMS lines equal the executables" "$(grep -c . <<<"$sums")" "$exes"
  for n in /bin/sbarhive.exe "${TEST_EXES[@]/#//tests/}"; do
    want="$(isoinfo -J -x "$n" -i "$iso" 2>/dev/null | sha256sum | cut -d' ' -f1)"
    got="$(grep -iF "${n##*/}" <<<"$sums" | head -n 1 | awk '{print tolower($1)}')"
    assert_eq "AC9: SHA256SUMS hash of $n matches its bytes on the ISO" "$got" "$want"
  done
  want="$(sha256sum "$FSB/target/x86_64-pc-windows-gnu/release/sbarhive.exe" | cut -d' ' -f1)"
  got="$(isoinfo -J -x /bin/sbarhive.exe -i "$iso" 2>/dev/null | sha256sum | cut -d' ' -f1)"
  assert_eq "AC9: /bin/sbarhive.exe is the release build" "$got" "$want"
  want="$(sha256sum "$WIN/run-tests.cmd" 2>/dev/null | cut -d' ' -f1)"
  got="$(isoinfo -J -x /run-tests.cmd -i "$iso" 2>/dev/null | sha256sum | cut -d' ' -f1)"
  assert_eq "AC9: extracted run-tests.cmd sha256 equals scripts/windows/run-tests.cmd" "$got" "$want"
  drop_fakes
  real_after="$(stat -c '%Y %s' "$real_iso" 2>/dev/null || echo absent)"
  assert_eq "AC9: the real ISO in the cache was not touched" "$real_after" "$real_before"
}

ac10_run_tests_cmd_is_crlf_runs_every_test_exe_and_writes_results_in_userprofile () {
  echo "== AC10: run-tests.cmd"
  local c="$WIN/run-tests.cmd"
  if [ ! -f "$c" ]; then fail "AC10: missing $c"; return; fi
  check "AC10: file says CRLF ($(file -b "$c"))" sh -c 'file -b "$1" | grep -q "with CRLF line terminators"' _ "$c"
  check "AC10: every line ends in CRLF (no bare LF)" awk '!/\r$/ { bad = 1 } END { exit bad }' "$c"
  check "AC10: loops over tests\\*.exe" grep -qiE 'tests\\\*\.exe' "$c"
  check "AC10: --test-threads=1" grep -qF -- '--test-threads=1' "$c"
  check "AC10: reads errorlevel" grep -qi 'errorlevel' "$c"
  check "AC10: writes under %USERPROFILE%\\sbarhive-tests" grep -qiF '%USERPROFILE%\sbarhive-tests' "$c"
  check "AC10: results.txt" grep -qiF 'results.txt' "$c"
  check "AC10: log.txt" grep -qiF 'log.txt' "$c"
  check "AC10: last line format 'total N failed F'" grep -qiE 'total .*failed' "$c"
  check "AC10: no redirect onto the disc (%~dp0, %cd% or a drive letter)" \
    sh -c '! grep -qiE ">>?[[:space:]]*\"?(%~dp0|%cd%|[a-z]:)" "$1"' _ "$c"
  check "AC10: no mkdir on the disc" sh -c '! grep -qiE "^[[:space:]]*(mkdir|md)[[:space:]]+\"?%~dp0" "$1"' _ "$c"
}

ac11_readme_gives_the_virsh_attach_command_in_both_languages () {
  echo "== AC11: README attach command"
  local r="$WIN/README.md" en ar re
  re='virsh -c qemu:///session attach-disk win11 [^[:space:]`]+ sdb --type cdrom --mode readonly'
  en="$(awk '/^## English/{s=1;next} /^## /{s=0} s' "$r")"
  ar="$(awk '/^## العربية/{s=1;next} /^## /{s=0} s' "$r")"
  check "AC11: English section has the attach command" grep -qE "$re" <<<"$en"
  check "AC11: Arabic section has the attach command" grep -qE "$re" <<<"$ar"
}

ac12_check_dlls_exits_2_on_input_errors_and_docs_name_binutils_and_ubuntu_mingw () {
  echo "== AC12: check-dlls.sh input errors"
  make_fakes
  local out rc
  out="$(timeout 30 "$WIN/check-dlls.sh" --text "$FSB/no-such-file.txt" 2>&1)"; rc=$?
  assert_eq "AC12: --text on a missing file: exit code" "$rc" "2"
  check "AC12: missing file prints no ok line" sh -c '! grep -q "dll check: ok" <<EOF
$1
EOF' _ "$out"
  : > "$FSB/empty.txt"
  out="$(timeout 30 "$WIN/check-dlls.sh" --text "$FSB/empty.txt" 2>&1)"; rc=$?
  assert_eq "AC12: --text on an empty file: exit code" "$rc" "2"
  check "AC12: empty file prints no ok line ([$out])" sh -c '! grep -q "dll check: ok" <<EOF
$1
EOF' _ "$out"
  grep -v 'DLL Name:' "$FIX/objdump-ok.txt" > "$FSB/no-dll-name.txt"
  out="$(timeout 30 "$WIN/check-dlls.sh" --text "$FSB/no-dll-name.txt" 2>&1)"; rc=$?
  assert_eq "AC12: --text on objdump output with no DLL Name line: exit code" "$rc" "2"
  check "AC12: no DLL Name line prints no ok line ([$out])" sh -c '! grep -q "dll check: ok" <<EOF
$1
EOF' _ "$out"
  cat > "$FSB/bin/x86_64-w64-mingw32-objdump" <<EOF
#!/bin/sh
grep -v 'DLL Name:' "$FIX/objdump-ok.txt"
EOF
  chmod 0755 "$FSB/bin/x86_64-w64-mingw32-objdump"
  printf 'MZfake' > "$FSB/sbarhive.exe"
  out="$(PATH="$FSB/bin:$PATH" timeout 30 "$WIN/check-dlls.sh" "$FSB/sbarhive.exe" 2>&1)"; rc=$?
  assert_eq "AC12: objdump output with no DLL Name line: exit code" "$rc" "2"
  check "AC12: objdump with no DLL Name prints no ok line ([$out])" sh -c '! grep -q "dll check: ok" <<EOF
$1
EOF' _ "$out"
  out="$(timeout 30 "$WIN/check-dlls.sh" --text "$FIX/objdump-mingw.txt" 2>&1)"; rc=$?
  assert_eq "AC12: rejection still exits 1" "$rc" "1"
  out="$(timeout 30 "$WIN/check-dlls.sh" --text "$FIX/objdump-ok.txt" 2>&1)"; rc=$?
  assert_eq "AC12: clean input still exits 0" "$rc" "0"
  drop_fakes

  echo "== AC12: windows-cross CARGO_TARGET_DIR"
  ci_check "AC12: windows-cross sets CARGO_TARGET_DIR inside the workspace" cross_target_dir
  ci_check "AC12: windows-cross does not name /home/sbarah" cross_no_home

  echo "== AC12: engines.md"
  local e=docs/engines.md ver row
  ver="$(rpm -q --qf '%{VERSION}' mingw64-binutils 2>/dev/null)" || ver=""
  [ -n "$ver" ] || ver='[0-9]+\.[0-9]+'
  row="$(grep -E '^\|[[:space:]]*mingw64-binutils[[:space:]]*\|' "$e")"
  check "AC12: engines.md has a mingw64-binutils row" test -n "$row"
  check "AC12: mingw64-binutils row has its version ($ver)" grep -qE "$ver" <<<"$row"
  check "AC12: mingw64-binutils row has its licence" grep -qE 'GPL' <<<"$row"
  check "AC12: mingw64-binutils row names x86_64-w64-mingw32-objdump" grep -qF 'x86_64-w64-mingw32-objdump' <<<"$row"
  check "AC12: the mingw64-gcc row no longer claims objdump" \
    sh -c '! grep -E "^\|[[:space:]]*mingw64-gcc[[:space:]]*\|" "$1" | grep -qi objdump' _ "$e"
  check "AC12: engines.md records Ubuntu's mingw-w64 package" grep -qE 'mingw-w64.*Ubuntu|Ubuntu.*mingw-w64' "$e"
}

lint_no_long_dash_and_shellcheck_for_s026b_files () {
  echo "== lint: bash -n, shellcheck, no long dash"
  local s
  for s in make-iso.sh check-dlls.sh; do
    if [ ! -f "$WIN/$s" ]; then fail "lint: missing $WIN/$s"; continue; fi
    check "lint: bash -n $s" bash -n "$WIN/$s"
    if command -v shellcheck >/dev/null; then check "lint: shellcheck $s" shellcheck "$WIN/$s"; fi
  done
  for s in "$WIN/make-iso.sh" "$WIN/run-tests.cmd" "$WIN/README.md" "$CI_YML" docs/engines.md; do
    [ -f "$s" ] || continue
    check "lint: no em or en dash in $s" sh -c '! grep -q "$2" "$1"' _ "$s" "$LONG_DASH"
  done
}

ac8_ci_adds_windows_cross_and_windows_native_and_leaves_check_alone
ac9_make_iso_builds_a_joliet_rock_ridge_iso_with_exes_sums_and_runner
ac10_run_tests_cmd_is_crlf_runs_every_test_exe_and_writes_results_in_userprofile
ac11_readme_gives_the_virsh_attach_command_in_both_languages
ac12_check_dlls_exits_2_on_input_errors_and_docs_name_binutils_and_ubuntu_mingw
lint_no_long_dash_and_shellcheck_for_s026b_files
finish "S-044b windows CI, ISO and runner (light)"
