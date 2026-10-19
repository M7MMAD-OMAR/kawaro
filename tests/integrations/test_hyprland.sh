#!/usr/bin/env bash
# S-006 AC7: the Hyprland helper copies the selection, then opens it.
# hyprctl and sbarhive are fakes; the real hyprctl would send Ctrl+C to the
# focused window of the running session.
set -u
# shellcheck source=tests/integrations/lib.sh
. "$(dirname "$0")/lib.sh"
make_sandbox
trap drop_sandbox EXIT

H="$REPO_ROOT/integrations/linux/hyprland/sbarhive-open-selection.sh"
check "helper exits 0" env SBARHIVE_COPY_DELAY=0 "$H"
want="hyprctl dispatch sendshortcut CTRL,C,activewindow
sbarhive open --from-clipboard"
assert_eq "call order" "$(cat "$FAKE_LOG/calls.log" 2>/dev/null)" "$want"

rm -f "$FAKE_LOG/calls.log"
check "SBARHIVE_BIN override" env SBARHIVE_COPY_DELAY=0 SBARHIVE_BIN="$SB/fakebin/sbarhive" "$H"
assert_eq "override call" "$(tail -n1 "$FAKE_LOG/calls.log" 2>/dev/null)" "sbarhive open --from-clipboard"

finish test_hyprland
