#!/usr/bin/env bash
# S-026a, light tests: no real build. AC1 and AC3 run the real scripts with a
# fake cargo first on PATH (and, for CI=true, a fake systemd-run); AC2 runs
# check-dlls.sh on the planted objdump fixtures; AC4 and AC7 read files.
# Run from outside any memory scope (a plain terminal or the Claude app scope),
# so the re-exec into systemd-run is observable:
#   bash tests/windows/test_scripts.sh
# The real builds are in tests/windows/test_heavy.sh.
# The sh -c bodies below take their values as arguments, on purpose.
# shellcheck disable=SC2016
set -u
# shellcheck source=tests/windows/lib.sh
. "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT" || exit 2
trap drop_fakes EXIT

ac1_build_sh_reexecs_in_a_6g_scope_with_j2_and_prints_its_cgroup () {
  echo "== AC1: build.sh outside a scope"
  make_fakes
  if [ ! -x "$WIN/build.sh" ]; then fail "AC1: missing or not executable $WIN/build.sh"; drop_fakes; return; fi
  local out rc first cg call me
  me="$(own_cgroup)"
  out="$(CI='' CARGO_TARGET_DIR="$FSB/target" PATH="$FSB/bin:$PATH" timeout 120 "$WIN/build.sh" 2>"$FSB/log/stderr")"
  rc=$?
  assert_eq "AC1: exit code" "$rc" "0"
  first="$(printf '%s\n' "$out" | head -n 1)"
  check "AC1: first line is 'cgroup: <path>' (got [$first])" grep -qE '^cgroup: /.+' <<<"$first"
  cg="${first#cgroup: }"
  call="$(cargo_call build)"
  if [ -z "$call" ]; then
    fail "AC1: cargo build was never called"
  else
    assert_eq "AC1: cargo build ran in the printed cgroup" "$(field 2 "$call")" "$cg"
    assert_eq "AC1: memory.max of the printed cgroup" "$(field 3 "$call")" "$SIX_G"
    check "AC1: the scope is a new cgroup, not the caller's ($me)" test "$cg" != "$me"
    check "AC1: cargo build ran with -j 2 ($(field 7 "$call"))" has_jobs_2 "$call"
    check "AC1: cargo build --release for x86_64-pc-windows-gnu ($(field 7 "$call"))" \
      sh -c 'case " $1 " in *" --release "*) exit 0 ;; esac; exit 1' _ "$(field 7 "$call")"
    check "AC1: --target x86_64-pc-windows-gnu ($(field 7 "$call"))" has_target_windows "$call"
    assert_eq "AC1: CARGO_TARGET_DIR from the caller is kept" "$(field 6 "$call")" "$FSB/target"
  fi
  check "AC1: sbarhive.exe produced in the target dir" test -f "$FSB/target/x86_64-pc-windows-gnu/release/sbarhive.exe"
  drop_fakes

  echo "== AC1: build.sh default target dir"
  make_fakes
  (unset CARGO_TARGET_DIR; CI='' PATH="$FSB/bin:$PATH" timeout 120 "$WIN/build.sh" >/dev/null 2>&1)
  call="$(cargo_call build)"
  assert_eq "AC1: default target dir" "$(field 6 "$call")" "$WIN_TARGET_DEFAULT"
  drop_fakes

  echo "== AC1: build.sh with CI=true runs directly"
  make_fakes
  cat > "$FSB/bin/systemd-run" <<'EOF'
#!/bin/sh
echo "systemd-run $*" >> "$FSB/log/systemd-run.log"
exit 99
EOF
  chmod 0755 "$FSB/bin/systemd-run"
  out="$(CI=true CARGO_TARGET_DIR="$FSB/target" PATH="$FSB/bin:$PATH" timeout 120 "$WIN/build.sh" 2>"$FSB/log/stderr")"
  rc=$?
  assert_eq "AC1 CI: exit code" "$rc" "0"
  assert_absent "$FSB/log/systemd-run.log"
  first="$(printf '%s\n' "$out" | head -n 1)"
  assert_eq "AC1 CI: first line is the caller's cgroup" "$first" "cgroup: $me"
  call="$(cargo_call build)"
  check "AC1 CI: cargo build was called" test -n "$call"
  check "AC1 CI: cargo build ran with -j 2" has_jobs_2 "$call"
  drop_fakes
}

ac2_check_dlls_accepts_system_dlls_and_rejects_lib_dlls () {
  echo "== AC2: check-dlls.sh"
  if [ ! -x "$WIN/check-dlls.sh" ]; then fail "AC2: missing or not executable $WIN/check-dlls.sh"; return; fi
  local out rc n
  n="$(grep -c 'DLL Name:' "$FIX/objdump-ok.txt")"
  out="$(timeout 30 "$WIN/check-dlls.sh" --text "$FIX/objdump-ok.txt" 2>&1)"; rc=$?
  assert_eq "AC2 ok: exit code" "$rc" "0"
  check "AC2 ok: line 'dll check: ok <name> $n imports' (got [$out])" \
    grep -qE "^dll check: ok (sbarhive\.exe|objdump-ok\.txt|$FIX/objdump-ok\.txt) $n imports\$" <<<"$out"

  out="$(timeout 30 "$WIN/check-dlls.sh" --text "$FIX/objdump-mingw.txt" 2>&1)"; rc=$?
  assert_eq "AC2 mingw: exit code" "$rc" "1"
  check "AC2 mingw: names libwinpthread-1.dll (got [$out])" grep -qF libwinpthread-1.dll <<<"$out"
  check "AC2 mingw: names libgcc_s_seh-1.dll (got [$out])" grep -qF libgcc_s_seh-1.dll <<<"$out"
  check "AC2 mingw: no ok line" sh -c '! grep -q "dll check: ok" <<EOF
$1
EOF' _ "$out"
  check "AC2 mingw: system DLLs are not listed as rejected" sh -c '! grep -qi "kernel32\|msvcrt" <<EOF
$1
EOF' _ "$out"

  make_fakes
  sed 's/DLL Name: msvcrt.dll/DLL Name: LIBSTDC++-6.DLL/' "$FIX/objdump-ok.txt" > "$FSB/upper.txt"
  out="$(timeout 30 "$WIN/check-dlls.sh" --text "$FSB/upper.txt" 2>&1)"; rc=$?
  assert_eq "AC2 upper case: exit code" "$rc" "1"
  check "AC2 upper case: names LIBSTDC++-6.DLL (got [$out])" grep -qF 'LIBSTDC++-6.DLL' <<<"$out"

  # Without --text it reads `x86_64-w64-mingw32-objdump -p <exe>`.
  cat > "$FSB/bin/x86_64-w64-mingw32-objdump" <<EOF
#!/bin/sh
echo "objdump \$*" >> "$FSB/log/objdump.log"
cat "$FIX/objdump-mingw.txt"
EOF
  chmod 0755 "$FSB/bin/x86_64-w64-mingw32-objdump"
  printf 'MZfake' > "$FSB/sbarhive.exe"
  out="$(PATH="$FSB/bin:$PATH" timeout 30 "$WIN/check-dlls.sh" "$FSB/sbarhive.exe" 2>&1)"; rc=$?
  assert_eq "AC2 exe: exit code from objdump output" "$rc" "1"
  check "AC2 exe: objdump -p ran on the exe" grep -qE -- "-p( .*)? $FSB/sbarhive.exe" "$FSB/log/objdump.log"
  check "AC2 exe: names libwinpthread-1.dll" grep -qF libwinpthread-1.dll <<<"$out"
  drop_fakes
}

ac3_check_sh_runs_windows_clippy_all_targets_in_a_6g_scope () {
  echo "== AC3: check.sh"
  make_fakes
  if [ ! -x "$WIN/check.sh" ]; then fail "AC3: missing or not executable $WIN/check.sh"; drop_fakes; return; fi
  local rc call
  CI='' PATH="$FSB/bin:$PATH" timeout 120 "$WIN/check.sh" >"$FSB/log/stdout" 2>&1; rc=$?
  assert_eq "AC3: exit code" "$rc" "0"
  call="$(cargo_call clippy)"
  if [ -z "$call" ]; then
    fail "AC3: cargo clippy was never called"
  else
    assert_eq "AC3: memory.max of clippy's cgroup" "$(field 3 "$call")" "$SIX_G"
    check "AC3: -j 2 ($(field 7 "$call"))" has_jobs_2 "$call"
    check "AC3: --target x86_64-pc-windows-gnu" has_target_windows "$call"
    check "AC3: -p sbarhive-core -p sbarhive-engines -p sbarhive-cli --all-targets ($(field 7 "$call"))" \
      argv_has "$call" "-p sbarhive-core" "-p sbarhive-engines" "-p sbarhive-cli" --all-targets
    check "AC3: -- -D warnings" argv_has "$call" "-- -D warnings"
  fi
  drop_fakes
}

# fn_has_cfg_unix FILE NAME: one of the attribute lines directly above
# `fn NAME(` is `#[cfg(unix)]`.
fn_has_cfg_unix () {
  awk -v name="$2" '
    { lines[NR] = $0 }
    $0 ~ "^[[:space:]]*fn " name "\\(" { at = NR }
    END {
      if (!at) exit 1
      for (i = at - 1; i > 0; i--) {
        l = lines[i]; gsub(/^[[:space:]]+|[[:space:]]+$/, "", l)
        if (l == "#[cfg(unix)]") exit 0
        if (l !~ /^#\[/ && l !~ /^\/\//) exit 1
      }
      exit 1
    }' "$1"
}

ac4_unix_only_tests_carry_cfg_unix_and_nothing_else_changes () {
  echo "== AC4: cfg(unix) guards"
  local f
  for f in crates/cli/tests/cli.rs crates/cli/tests/settings.rs; do
    check "AC4: $f starts with the inner attribute #![cfg(unix)]" \
      grep -qxE '#!\[cfg\(unix\)\]' "$f"
  done
  check "AC4: output.rs ac7_dangling_symlink_counts_as_taken has #[cfg(unix)]" \
    fn_has_cfg_unix crates/core/tests/output.rs ac7_dangling_symlink_counts_as_taken
  check "AC4: contracts.rs socket_path_lives_in_the_runtime_dir has #[cfg(unix)]" \
    fn_has_cfg_unix crates/core/tests/contracts.rs socket_path_lives_in_the_runtime_dir
  check "AC4: office.rs office_find_program_respects_name_order_and_exec_bit has #[cfg(unix)]" \
    fn_has_cfg_unix crates/engines/src/office.rs office_find_program_respects_name_order_and_exec_bit
  for f in office_timeout_kills_the_process_group office_cancel_kills_the_process_group \
           office_already_cancelled_starts_nothing office_missing_program_is_unavailable_error; do
    check "AC4: office_engine.rs $f (a /bin/sh fake program) has #[cfg(unix)]" \
      fn_has_cfg_unix crates/engines/tests/office_engine.rs "$f"
  done
  check "AC4: pdf_ghostscript.rs pdf_ghostscript_identity_and_capabilities (/bin/sh) has #[cfg(unix)]" \
    fn_has_cfg_unix crates/engines/tests/pdf_ghostscript.rs pdf_ghostscript_identity_and_capabilities

  # 5be5270 is the commit S-026a starts from.
  local files=(crates/cli/tests/cli.rs crates/cli/tests/settings.rs crates/core/tests/output.rs
               crates/core/tests/contracts.rs crates/engines/src/office.rs
               crates/engines/tests/office_engine.rs crates/engines/tests/pdf_ghostscript.rs)
  local removed added_other
  removed="$(git diff -U0 5be5270 -- "${files[@]}" | grep -E '^-' | grep -vE '^--- ')"
  added_other="$(git diff -U0 5be5270 -- "${files[@]}" | grep -E '^\+' | grep -vE '^\+\+\+ ' \
    | grep -vE '^\+[[:space:]]*#!?\[cfg(_attr)?\(.*\)\][[:space:]]*$')"
  assert_eq "AC4: no deleted line in the seven files" "$removed" ""
  assert_eq "AC4: every added line is a cfg line" "$added_other" ""
}

ac7_engines_md_and_readme_document_the_build_tools () {
  echo "== AC7: docs"
  check "AC7: engines.md names mingw64-gcc 16.2.1 with its licence" \
    grep -qE 'mingw64-gcc.*16\.2\.1.*GPL|GPL.*mingw64-gcc.*16\.2\.1' docs/engines.md
  check "AC7: engines.md names genisoimage with its licence" \
    grep -qE 'genisoimage.*GPL|GPL.*genisoimage' docs/engines.md
  local r=scripts/windows/README.md
  if [ ! -f "$r" ]; then fail "AC7: missing $r"; return; fi
  local s
  for s in build.sh check-dlls.sh check.sh; do assert_contains "AC7: README names $s" "$r" "$s"; done
  check "AC7: README has Arabic text" grep -qP '\p{Arabic}' "$r"
  check "AC7: README has English text" grep -qiE '\b(the|build|run)\b' "$r"
}

no_long_dash_and_shell_lint_for_the_new_scripts () {
  echo "== AC1 AC2 AC3: bash -n, shellcheck, no long dash"
  local s
  for s in build.sh check-dlls.sh check.sh; do
    if [ ! -f "$WIN/$s" ]; then fail "lint: missing $WIN/$s"; continue; fi
    check "lint: bash -n $s" bash -n "$WIN/$s"
    if command -v shellcheck >/dev/null; then check "lint: shellcheck $s" shellcheck "$WIN/$s"; fi
  done
  for s in "$WIN"/build.sh "$WIN"/check-dlls.sh "$WIN"/check.sh "$WIN"/README.md; do
    [ -f "$s" ] || continue
    check "lint: no em or en dash in $s" sh -c '! grep -q "$(printf "\342\200\224\|\342\200\223")" "$1"' _ "$s"
  done
}

ac1_build_sh_reexecs_in_a_6g_scope_with_j2_and_prints_its_cgroup
ac2_check_dlls_accepts_system_dlls_and_rejects_lib_dlls
ac3_check_sh_runs_windows_clippy_all_targets_in_a_6g_scope
ac4_unix_only_tests_carry_cfg_unix_and_nothing_else_changes
ac7_engines_md_and_readme_document_the_build_tools
no_long_dash_and_shell_lint_for_the_new_scripts
finish "S-026a windows scripts (light)"
