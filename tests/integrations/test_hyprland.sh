#!/usr/bin/env bash
# S-006 AC7: the Hyprland helper copies the selection, then opens it.
# hyprctl and kawaro are fakes; the real hyprctl would send Ctrl+C to the
# focused window of the running session.
set -u
# shellcheck source=tests/integrations/lib.sh
. "$(dirname "$0")/lib.sh"
make_sandbox
trap drop_sandbox EXIT

# S-110 AC7: the helper reads the pointer once with `hyprctl cursorpos` and
# passes it as `--at X,Y`. The stub prints $FAKE_CURSOR (or fails with
# $FAKE_CURSOR_RC) for `cursorpos`; every call is logged like the fake of
# lib.sh.
cat > "$SB/fakebin/hyprctl" <<'EOF2'
#!/bin/sh
echo "hyprctl $*" >> "$FAKE_LOG/calls.log"
if [ "${1:-}" = cursorpos ]; then
  [ -n "${FAKE_CURSOR+x}" ] && printf '%s\n' "$FAKE_CURSOR"
  exit "${FAKE_CURSOR_RC:-0}"
fi
exit 0
EOF2
chmod 0755 "$SB/fakebin/hyprctl"
export FAKE_CURSOR="640, 360"

H="$REPO_ROOT/integrations/linux/hyprland/kawaro-open-selection.sh"
check "helper exits 0" env KAWARO_COPY_DELAY=0 "$H"
want="hyprctl dispatch sendshortcut CTRL,C,activewindow
kawaro open --from-clipboard --at 640,360"
# S-110: the cursorpos read is checked on its own below.
assert_eq "call order" "$(grep -v '^hyprctl cursorpos$' "$FAKE_LOG/calls.log" 2>/dev/null)" "$want"

rm -f "$FAKE_LOG/calls.log"
check "KAWARO_BIN override" env KAWARO_COPY_DELAY=0 KAWARO_BIN="$SB/fakebin/kawaro" "$H"
assert_eq "override call" "$(tail -n1 "$FAKE_LOG/calls.log" 2>/dev/null)" "kawaro open --from-clipboard --at 640,360"

# S-110 AC7: syntax, one cursorpos read, the exact final call.
check "bash -n" bash -n "$H"
check "sh -n" sh -n "$H"
for pair in "640, 360|640,360" "-1280, 5|-1280,5" "10.5, 20.25|10.5,20.25"; do
  out="${pair%%|*}"; arg="${pair##*|}"
  rm -f "$FAKE_LOG/calls.log"
  FAKE_CURSOR="$out" check "cursorpos [$out] exits 0" env KAWARO_COPY_DELAY=0 "$H"
  assert_eq "cursorpos [$out] read once" "$(grep -c '^hyprctl cursorpos$' "$FAKE_LOG/calls.log")" "1"
  assert_eq "ctrl+c once [$out]" "$(grep -c '^hyprctl dispatch sendshortcut CTRL,C,activewindow$' "$FAKE_LOG/calls.log")" "1"
  assert_eq "final call [$out]" "$(tail -n1 "$FAKE_LOG/calls.log")" "kawaro open --from-clipboard --at $arg"
done

# S-110 AC7: no pointer, no open (no 0,0 and no centred substitute).
for mode in fail nothing empty; do
  rm -f "$FAKE_LOG/calls.log"
  case "$mode" in
    fail) export FAKE_CURSOR_RC=1; export FAKE_CURSOR="" ;;
    nothing) export FAKE_CURSOR_RC=0; export FAKE_CURSOR="nothing" ;;
    empty) export FAKE_CURSOR_RC=0; export FAKE_CURSOR="" ;;
  esac
  rc=0
  KAWARO_COPY_DELAY=0 "$H" 2> "$SB/err.txt" || rc=$?
  check "[$mode] exits non-zero" test "$rc" -ne 0
  assert_eq "[$mode] one stderr line" "$(wc -l < "$SB/err.txt" | tr -d ' ')" "1"
  assert_eq "[$mode] no kawaro open call" "$(grep -c '^kawaro open' "$FAKE_LOG/calls.log" 2>/dev/null)" "0"
done
unset FAKE_CURSOR_RC
export FAKE_CURSOR="640, 360"

finish test_hyprland
