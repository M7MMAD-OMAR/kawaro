#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-114 fast loop tests (no display, no Xvfb, no network): the structure and
# skip behaviour of tests/x11/run.sh, and the pure helpers of tests/x11/lib.sh.
# The live checks (position, focus, stacking, pixel, 150 ms, 40 MB) are the
# manual release step and are not part of this file. The Rust half (tool
# manifest, licences, docs) is crates/core/tests/s114_sources.rs.
# Usage: tests/x11/s114.sh
set -u
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
RUN="$HERE/run.sh"
LIB="$HERE/lib.sh"
FAILS=0
CHECKS=0
WORK="$(mktemp -d "$HOME/.cache/kawaro-s114-test.XXXXXX")" || exit 1
trap 'rm -rf "$WORK"' EXIT
pass() { CHECKS=$((CHECKS + 1)); echo "PASS $*"; }
fail() { CHECKS=$((CHECKS + 1)); FAILS=$((FAILS + 1)); echo "FAIL $*" >&2; }
check() { local d="$1"; shift; if "$@"; then pass "$d"; else fail "$d"; fi; }
has() { grep -Eq -- "$2" "$1"; }

# ---- AC1: the skip rule, run with a PATH that hides the X tools ----
mkshim() { # mkshim DIR: coreutils and friends only, no Xvfb, no xfwm4
  mkdir -p "$1"
  local c p
  for c in bash env dirname basename mktemp rm cat sleep date mkdir grep sed awk \
    head tail tr cut sort seq kill readlink realpath id uname wc ls printf true \
    false cp mv ln touch tee find timeout cmp diff; do
    p="$(command -v "$c" 2>/dev/null)" && ln -sf "$p" "$1/$c"
  done
}
fake() { # fake DIR NAME MARKER: a stand-in that records that it was started
  printf '#!/bin/bash\necho started >> "%s"\nexit 0\n' "$3" >"$1/$2"
  chmod +x "$1/$2"
}
skip_case() { # skip_case NAME HAVE_XVFB WANT_LINE
  local name="$1" have="$2" want="$3" d="$WORK/$1" out rc
  mkshim "$d/bin"
  mkdir -p "$d/home"
  [ "$have" = yes ] && fake "$d/bin" Xvfb "$d/mark"
  [ ! -x "$RUN" ] && { fail "AC1 $name: tests/x11/run.sh is executable"; return; }
  out="$(cd "$d" && env -i HOME="$d/home" PATH="$d/bin" DISPLAY=:0 timeout 20 "$d/bin/bash" "$RUN" 2>"$d/err")"
  rc=$?
  [ "$rc" = 0 ] && pass "AC1 $name: exit 0" || fail "AC1 $name: exit $rc, want 0"
  grep -qxF "$want" <<<"$out" && pass "AC1 $name: stdout has '$want'" || fail "AC1 $name: stdout was '$out', want line '$want'"
  [ ! -e "$d/mark" ] && pass "AC1 $name: started no process" || fail "AC1 $name: a fake X tool was started"
  [ -z "$(find "$d/home" -mindepth 1 2>/dev/null)" ] && pass "AC1 $name: created nothing under HOME" || fail "AC1 $name: left files under HOME"
}
skip_case no-xvfb no 'SKIP: Xvfb not found'
skip_case no-xfwm4 yes 'SKIP: xfwm4 not found'

# ---- AC1, AC2: static structure of run.sh ----
check "AC1 run.sh exists and is executable" test -x "$RUN"
if [ -f "$RUN" ]; then
  check "AC1 run.sh starts with the bash shebang" test "$(head -1 "$RUN")" = '#!/usr/bin/env bash'
  check "AC1 run.sh has the SPDX line" has "$RUN" 'SPDX-License-Identifier: GPL-3.0-or-later'
  check "AC1 bash -n is clean" bash -n "$RUN"
  if command -v shellcheck >/dev/null; then
    check "AC1 shellcheck at warning level is clean" shellcheck -S warning "$RUN"
  else
    fail "AC1 shellcheck is not installed (needed for the gate)"
  fi
  skip_ln="$(grep -n 'SKIP: Xvfb not found' "$RUN" | head -1 | cut -d: -f1)"
  start_ln="$(grep -n -- '-displayfd' "$RUN" | head -1 | cut -d: -f1)"
  check "AC1 the SKIP line comes before the first Xvfb start" \
    test -n "$skip_ln" -a -n "$start_ln" -a "${skip_ln:-999}" -lt "${start_ln:-0}"
  check "AC1 run.sh names the xfwm4 skip line" has "$RUN" 'SKIP: xfwm4 not found'
  check "AC2 Xvfb with -displayfd and 1280x800x24" has "$RUN" 'Xvfb .*-displayfd'
  check "AC2 screen size 1280x800x24" has "$RUN" '1280x800x24'
  check "AC2 xfwm4 with the compositor off" has "$RUN" 'xfwm4 .*--compositor=off'
  check "AC2 inside dbus-run-session" has "$RUN" 'dbus-run-session'
  check "AC2 unsets WAYLAND_DISPLAY" has "$RUN" 'unset .*WAYLAND_DISPLAY'
  check "AC2 unsets HYPRLAND_INSTANCE_SIGNATURE" has "$RUN" 'unset .*HYPRLAND_INSTANCE_SIGNATURE'
  check "AC2 forces GDK_BACKEND=x11 (xfwm4 dies on a Wayland GDK_BACKEND, S-114 risk finding)" has "$RUN" 'GDK_BACKEND=x11'
  check "AC2 temporary HOME and XDG_RUNTIME_DIR" has "$RUN" 'XDG_RUNTIME_DIR='
  check "AC2 temporary directory is made under \$HOME/.cache" has "$RUN" 'mktemp -d .*\.cache'
  check "AC2 an EXIT trap removes it" has "$RUN" "trap .* EXIT"
  check "AC2 polls the runtime dir for wayland-* files" has "$RUN" 'wayland-\*'
  check "AC2 never uses pkill or killall" bash -c "! grep -Eq '(^|[^a-z])(pkill|killall)([^a-z]|$)' '$RUN'"
  check "AC2 never writes under /tmp" bash -c "! grep -Eq '(^|[^A-Za-z.])/tmp([/ \"]|$)' '$RUN'"
  check "AC3 the tool target dir is under \$HOME/.cache" has "$RUN" 'CARGO_TARGET_DIR=.*\.cache'
  check "AC4 compares the log line 'hive surface: window'" has "$RUN" 'hive surface: window'
  check "AC5 counts the choice line" has "$RUN" 'hive surface choice: window, not a Wayland session'
  check "AC6 reads 'hive focus: true'" has "$RUN" 'hive focus: true'
  check "AC6 runs with DESKTOP_STARTUP_ID" has "$RUN" 'DESKTOP_STARTUP_ID'
  check "AC7 sends Esc through the tool" has "$RUN" 'x11-tool.* key|key Escape'
  check "AC8 sets the root colour with xsetroot -solid" has "$RUN" 'xsetroot .*-solid'
  check "AC9 runs measure.sh with X11=1" has "$RUN" 'X11=1'
  check "AC9 reads RSS from /proc" has "$RUN" '/proc/'
fi
check "AC9 crates/ui/tests/measure.sh understands X11=1" has "$ROOT/crates/ui/tests/measure.sh" 'X11'
check "AC3 .gitignore ignores the tool target dir" has "$ROOT/.gitignore" 'tests/x11/x11-tool/target'

# ---- the pure helpers of tests/x11/lib.sh ----
if [ ! -f "$LIB" ]; then
  fail "pure helpers: tests/x11/lib.sh does not exist"
else
  # shellcheck source=/dev/null
  source "$LIB"
  L='2026-10-31T09:00:00Z hive surface: window 400 500 618 574'
  check "AC4 parse: window line gives X Y W H" test "$(x11_parse_surface "$L")" = '400 500 618 574'
  check "AC4 parse: negative origin" test "$(x11_parse_surface 'hive surface: window -3 0 100 90')" = '-3 0 100 90'
  expect_fail() { local d="$1"; shift; if "$@" >/dev/null 2>&1; then fail "$d"; else pass "$d"; fi; }
  expect_fail "AC4 parse: the choice line is not a surface line" x11_parse_surface 'hive surface choice: window, not a Wayland session'
  expect_fail "AC4 parse: a layer line is not a window line" x11_parse_surface 'hive surface: layer 1 2 3 4'
  expect_fail "AC4 parse: a short line fails" x11_parse_surface 'hive surface: window 1 2 3'
  expect_fail "AC4 parse: non numbers fail" x11_parse_surface 'hive surface: window a b c d'
  check "AC4 within: equal" x11_within 400 400 1
  check "AC4 within: one apart" x11_within 400 401 1
  check "AC4 within: one apart, other way" x11_within 401 400 1
  expect_fail "AC4 within: two apart" x11_within 400 402 1
  check "AC4 within: negative values" x11_within -1 0 1
  check "AC4 match: identical" x11_geometry_match '400 500 618 574' '400 500 618 574'
  check "AC4 match: 1 px off in each field" x11_geometry_match '400 500 618 574' '401 499 619 573'
  expect_fail "AC4 match: 2 px off in X" x11_geometry_match '400 500 618 574' '402 500 618 574'
  expect_fail "AC4 match: 2 px off in H" x11_geometry_match '400 500 618 574' '400 500 618 576'
  check "AC4 inside: fits" x11_box_inside '331 226 618 574' 1280 800
  check "AC4 inside: touching the edges" x11_box_inside '0 0 1280 800' 1280 800
  expect_fail "AC4 inside: past the right edge" x11_box_inside '700 0 618 574' 1280 800
  expect_fail "AC4 inside: past the bottom edge" x11_box_inside '0 300 618 574' 1280 800
  expect_fail "AC4 inside: negative origin" x11_box_inside '-1 0 618 574' 1280 800
  printf 'a\nhive surface choice: window, not a Wayland session\nb\nhive surface choice: window, screen reader running\nhive surface choice: window, not a Wayland session x\n' >"$WORK/log"
  check "AC5 count: exact line counts once" test "$(x11_count_choice "$WORK/log")" = 1
  : >"$WORK/empty"
  check "AC5 count: empty log is 0" test "$(x11_count_choice "$WORK/empty")" = 0
  check "AC8 pixel: case and prefix are ignored" x11_pixel_eq '#1A2B3C' '0x1a2b3c'
  expect_fail "AC8 pixel: different colours" x11_pixel_eq '1a2b3c' '1a2b3d'
  check "AC9 median: odd count" test "$(x11_median 90 10 50)" = 50
  check "AC9 median: even count takes the lower middle" test "$(x11_median 40 10 30 20)" = 20
  check "AC9 median: one value" test "$(x11_median 7)" = 7
  check "AC9 rss: 40 MB passes" x11_rss_ok 40960
  expect_fail "AC9 rss: one KB over fails" x11_rss_ok 40961
fi

echo "$CHECKS checks, $FAILS failed"
[ "$FAILS" -eq 0 ]
