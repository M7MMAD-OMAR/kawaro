#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103g: the Flathub assessment and the manifest draft. Offline. The manifest
# is parsed by flatpak-builder --show-manifest when installed, and by
# flatpak-builder-lint when installed (CI runs it, see the assessment).
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

APPID="io.github.m7mmad_omar.Kawaro"
MAN="$REPO_ROOT/packaging/flatpak/$APPID.yml"
DOC="$REPO_ROOT/docs/distribution-channels/flathub.md"
META="$REPO_ROOT/packaging/$APPID.metainfo.xml"

check "S-103g the manifest draft exists" test -f "$MAN"
check "S-103g the assessment exists" test -f "$DOC"

py() { python3 -I -c "$1" "$MAN"; }
check "S-103g the manifest is valid YAML with app-id, runtime, command and modules" py '
import sys, yaml
m = yaml.safe_load(open(sys.argv[1]))
assert m["app-id"] and m["runtime"] and m["sdk"] and m["command"] and m["modules"]'
check "S-103g app-id equals the metainfo id" py "
import sys, yaml
m = yaml.safe_load(open(sys.argv[1]))
assert m['app-id'] == '$APPID'
assert '<id>$APPID</id>' in open('$META').read()"
check "S-103g the runtime version is quoted text, not a float" py '
import sys, yaml
m = yaml.safe_load(open(sys.argv[1]))
assert isinstance(m["runtime-version"], str)'
check "S-103g the build is offline and locked" \
  bash -c 'grep -q -- "--offline" "$1" && grep -q -- "--locked" "$1"' _ "$MAN"
check "S-103g no host escape: no talk to org.freedesktop.Flatpak, no --device=all, no --filesystem=host-root" \
  bash -c '! grep -E -- "org\.freedesktop\.Flatpak|--device=all|--filesystem=host-root|--socket=session-bus|--socket=system-bus" "$1" | grep -v "^[[:space:]]*#"' _ "$MAN"
check "S-103g Wayland is granted" grep -q -- '--socket=wayland' "$MAN"
check "S-103g the unproven filesystem grant is justified in a comment" \
  bash -c 'grep -B3 -- "--filesystem=" "$1" | grep -q "^[[:space:]]*#"' _ "$MAN"

if command -v flatpak-builder >/dev/null 2>&1; then
  check "S-103g flatpak-builder --show-manifest accepts the manifest" \
    bash -c 'flatpak-builder --show-manifest "$1" >/dev/null 2>&1' _ "$MAN"
else
  echo "SKIP S-103g flatpak-builder is not installed"
fi
if command -v flatpak-builder-lint >/dev/null 2>&1; then
  check "S-103g flatpak-builder-lint manifest reports no errors" flatpak-builder-lint manifest "$MAN"
else
  echo "SKIP S-103g flatpak-builder-lint is not installed (documented CI command)"
fi
check "S-103g a deliberately broken copy fails the YAML check" bash -c '
printf "app-id: [unclosed\n" > "$1/broken.yml"
! python3 -I -c "import sys,yaml; yaml.safe_load(open(sys.argv[1]))" "$1/broken.yml" 2>/dev/null' _ "$TMP"

# The assessment: 8 limits, each rated, one Decision line.
for limit in "systemd user daemon" "file manager integration" "writing next to the original" \
             "host engines" "layer-shell" "xdg-activation" "tray icon" "GlobalShortcuts portal"; do
  check "S-103g the assessment rates: $limit" \
    bash -c 'grep -i -F "$2" "$1" | grep -qE "(works|degraded|blocked)"' _ "$DOC" "$limit"
done
check "S-103g exactly one Decision line, GO or NO-GO with a reason" \
  bash -c '[ "$(grep -cE "^Decision: (GO|NO-GO)\b" "$1")" -eq 1 ] && grep -E "^Decision: (GO|NO-GO)" "$1" | grep -qE ":.{25,}"' _ "$DOC"
check "S-103g the Decision line is the last non empty line" \
  bash -c 'tail -n 1 <(grep -v "^[[:space:]]*$" "$1") | grep -qE "^Decision: "' _ "$DOC"
check "S-103g Flathub policy points are marked to confirm" grep -qi 'to confirm' "$DOC"
check "S-103g the lint command for CI is documented" grep -q 'flatpak-builder-lint manifest' "$DOC"
check "S-103g the doc names the cheapest path" grep -qi 'cheapest path' "$DOC"

LONG_DASH="$(printf '\342\200\224\|\342\200\223')"
for f in "$MAN" "$DOC"; do
  check "S-103g no long dash in $(basename "$f")" bash -c '! grep -q "$2" "$1"' _ "$f" "$LONG_DASH"
  check "S-103g no email, home path or key block in $(basename "$f")" \
    bash -c '! grep -qE "[A-Za-z0-9._-]+@[A-Za-z0-9.-]+\.[a-z]{2,}|/home/|BEGIN [A-Z ]*PRIVATE KEY" "$1"' _ "$f"
done
finish
