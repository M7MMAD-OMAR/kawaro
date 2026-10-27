#!/usr/bin/env bash
# S-045 heavy tests (backlog/S-045-windows-cfg-gates.md): the real Windows
# build. Every cargo call goes through the scope scripts (6 GB, two CPUs, -j 2).
#   bash tests/windows/test_s045.sh
set -u
cd "$(dirname "$0")/../.." || exit 2
REPO="$PWD"
TARGET="${CARGO_TARGET_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target}"
REL="$TARGET/x86_64-pc-windows-gnu/release"
fails=0
fail () { echo "FAIL: $*"; fails=$((fails + 1)); }
ok () { echo "ok: $*"; }

# AC1: the whole workspace checks for Windows.
echo "== AC1: cargo check --workspace for Windows"
CARGO_TARGET_DIR="$TARGET" systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200% \
  cargo check -j 2 --workspace --target x86_64-pc-windows-gnu
[ $? -eq 0 ] && ok "AC1 check" || fail "AC1: cargo check --workspace --target x86_64-pc-windows-gnu"

# AC8: check.sh (with the daemon and ui clippy) and build.sh exit 0.
echo "== AC8: check.sh and build.sh"
"$REPO/scripts/windows/check.sh" && ok "AC8 check.sh" || fail "AC8: check.sh"
"$REPO/scripts/windows/build.sh" && ok "AC8 build.sh" || fail "AC8: build.sh"

# AC1, AC4: the daemon exe exists and is a PE32+ executable.
echo "== AC1/AC4: kawarod.exe"
if [ -f "$REL/kawarod.exe" ]; then
  file "$REL/kawarod.exe" | grep -q 'PE32+ executable' && ok "PE32+" || fail "kawarod.exe is not PE32+"
else
  fail "AC1/AC4: $REL/kawarod.exe does not exist"
fi

# AC5: subsystem GUI (2) for the daemon, console (3) for the CLI.
echo "== AC5: subsystem"
sub () { x86_64-w64-mingw32-objdump -p "$1" 2>/dev/null | awk '$1 == "Subsystem" {print $2}'; }
[ "$(sub "$REL/kawarod.exe")" = 00000002 ] && ok "daemon GUI" || fail "AC5: kawarod.exe Subsystem is '$(sub "$REL/kawarod.exe")', want 00000002"
[ "$(sub "$REL/kawaro.exe")" = 00000003 ] && ok "cli console" || fail "AC5: kawaro.exe Subsystem is '$(sub "$REL/kawaro.exe")', want 00000003"

# AC8: no lib*.dll import.
echo "== AC8: check-dlls"
out="$("$REPO/scripts/windows/check-dlls.sh" "$REL/kawarod.exe" 2>&1)"
echo "$out" | grep -Eq '^dll check: ok kawarod\.exe [0-9]+ imports$' && ok "dll check" || fail "AC8: check-dlls said: $out"

# AC9: Cargo.lock package count unchanged (baseline 672, 673 since S-053a adds minisign-verify), ui diff is lib.rs only.
# S-085 adds winresource 0.1.34 (MIT, build dependency); toml and version_check were already locked, so 674.
echo "== AC9: lock and ui"
n="$(grep -c '^\[\[package\]\]' Cargo.lock)"
[ "$n" = 674 ] && ok "lock 674" || fail "AC9: Cargo.lock has $n packages, baseline 674"
git diff --name-only HEAD -- crates/ui | grep -vx 'crates/ui/src/lib.rs' | grep -q . \
  && fail "AC9: crates/ui changed outside lib.rs" || ok "ui diff"

echo "failures: $fails"
[ "$fails" -eq 0 ]
