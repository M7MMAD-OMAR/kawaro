#!/usr/bin/env bash
# S-006 AC7: the Hyprland helper copies the selection, then opens it.
# hyprctl and kawaro are fakes; the real hyprctl would send Ctrl+C to the
# focused window of the running session.
set -u
# shellcheck source=tests/integrations/lib.sh
. "$(dirname "$0")/lib.sh"
make_sandbox
trap drop_sandbox EXIT

H="$REPO_ROOT/integrations/linux/hyprland/kawaro-open-selection.sh"
check "helper exits 0" env KAWARO_COPY_DELAY=0 "$H"
want="hyprctl dispatch sendshortcut CTRL,C,activewindow
kawaro open --from-clipboard"
assert_eq "call order" "$(cat "$FAKE_LOG/calls.log" 2>/dev/null)" "$want"

rm -f "$FAKE_LOG/calls.log"
check "KAWARO_BIN override" env KAWARO_COPY_DELAY=0 KAWARO_BIN="$SB/fakebin/kawaro" "$H"
assert_eq "override call" "$(tail -n1 "$FAKE_LOG/calls.log" 2>/dev/null)" "kawaro open --from-clipboard"

finish test_hyprland
