#!/usr/bin/env bash
# S-007: install.sh and uninstall.sh in a throwaway HOME with fake binaries.
# Never touches the real HOME: lib.sh points HOME and every XDG directory at a
# mktemp sandbox and shadows systemctl and hyprctl with logging fakes.
set -u
# shellcheck source=tests/integrations/lib.sh
. "$(dirname "$0")/lib.sh"
INSTALL="$REPO_ROOT/scripts/install.sh"
UNINSTALL="$REPO_ROOT/scripts/uninstall.sh"
FX="$REPO_ROOT/tests/integrations/fixtures"

guard_home () {
  case "$HOME" in "$SB"/home) pass ;; *) echo "HOME escaped the sandbox: $HOME" >&2; exit 2 ;; esac
}

seed_home () {
  mkdir -p "$HOME/.local/bin" "$XDG_CONFIG_HOME/Thunar" "$XDG_CONFIG_HOME/kawaro"
  printf '#!/bin/sh\n' > "$HOME/.local/bin/unrelated-tool"
  chmod 0755 "$HOME/.local/bin/unrelated-tool"
  cp "$FX/uca-user.xml" "$XDG_CONFIG_HOME/Thunar/uca.xml"
  echo 'theme = "dark"' > "$XDG_CONFIG_HOME/kawaro/config.toml"
}

brand_dir () {
  mkdir -p "$SB/brand"
  cp "$FX/icon.svg" "$SB/brand/kawaro.svg"
  cp "$FX/icon.svg" "$SB/brand/kawaro-symbolic.svg"
  echo "$SB/brand"
}

validate_desktop () {
  if command -v desktop-file-validate >/dev/null; then
    check "desktop-file-validate $1" desktop-file-validate "$1"
  fi
}

###########################################################################
echo "== scenario A: every file manager, Hyprland, systemd, icons"
make_sandbox
guard_home
seed_home
BIN="$HOME/.local/bin"
D="$XDG_DATA_HOME"
C="$XDG_CONFIG_HOME"
before="$(snapshot "$HOME")"
export KAWARO_FILE_MANAGERS=dolphin,nautilus,nemo,thunar KAWARO_HYPRLAND=1 KAWARO_SYSTEMD=1
KAWARO_BRAND_DIR="$(brand_dir)"; export KAWARO_BRAND_DIR
export KAWARO_THUNAR_SYSTEM_UCA="$FX/uca-system.xml"

if "$INSTALL" --no-build > "$SB/log/install1.out" 2>&1; then pass; else fail "install #1 exit code"; cat "$SB/log/install1.out" >&2; fi

assert_file "$BIN/kawaro" 755
assert_file "$BIN/kawarod" 755
assert_file "$BIN/kawaro-open-selection" 755
assert_file "$D/applications/io.github.m7mmad_omar.Kawaro.desktop" 644
assert_file "$D/metainfo/io.github.m7mmad_omar.Kawaro.metainfo.xml" 644
assert_file "$D/icons/hicolor/scalable/apps/kawaro.svg" 644
assert_file "$D/icons/hicolor/symbolic/apps/kawaro-symbolic.svg" 644
assert_file "$D/kio/servicemenus/kawaro.desktop" 755
assert_file "$D/nautilus-python/extensions/kawaro.py" 644
assert_file "$D/nemo/actions/kawaro.nemo_action" 644
assert_file "$C/systemd/user/kawaro.service" 644
assert_absent "$C/autostart/kawaro.desktop"
assert_eq "binary copied" "$(cat "$BIN/kawaro")" "$(cat "$SB/release/kawaro")"

# AC6: absolute paths everywhere a launcher runs us.
assert_eq "Dolphin Exec" "$(ini_get "$D/kio/servicemenus/kawaro.desktop" 'Desktop Action kawaro' Exec)" "$BIN/kawaro open -- %F"
assert_eq "Nemo Exec" "$(ini_get "$D/nemo/actions/kawaro.nemo_action" 'Nemo Action' Exec)" "$BIN/kawaro open -- %F"
assert_eq "app Exec" "$(ini_get "$D/applications/io.github.m7mmad_omar.Kawaro.desktop" 'Desktop Entry' Exec)" "$BIN/kawaro settings"
assert_eq "unit ExecStart" "$(ini_get "$C/systemd/user/kawaro.service" Service ExecStart)" "$BIN/kawarod"
assert_contains "Nautilus binary" "$D/nautilus-python/extensions/kawaro.py" "KAWARO_BIN = \"$BIN/kawaro\""
assert_contains "Hyprland binary" "$BIN/kawaro-open-selection" "$BIN/kawaro"
assert_contains "Thunar command" "$C/Thunar/uca.xml" "<command>$BIN/kawaro open -- %F</command>"
assert_contains "Thunar other action" "$C/Thunar/uca.xml" "<unique-id>1700000000000001-1</unique-id>"
validate_desktop "$D/applications/io.github.m7mmad_omar.Kawaro.desktop"

# AC5: systemd calls, through the fake only.
assert_contains "daemon-reload" "$SB/log/systemctl.log" "--user daemon-reload"
assert_contains "enable --now" "$SB/log/systemctl.log" "--user enable --now kawaro.service"
assert_absent "$SB/log/forbidden.log"

# AC4, AC9: bind line and bilingual summary.
assert_contains "bind line printed" "$SB/log/install1.out" "bind = SUPER, mouse:273, exec, $BIN/kawaro-open-selection"
assert_contains "Arabic summary" "$SB/log/install1.out" "اكتمل تثبيت كوارو"
assert_contains "English summary" "$SB/log/install1.out" "Kawaro installed"
check "hyprland.conf untouched" test ! -e "$C/hypr/hyprland.conf"

# AC7: second run gives the same tree.
once="$(snapshot "$HOME")"
if "$INSTALL" --no-build > "$SB/log/install2.out" 2>&1; then pass; else fail "install #2 exit code"; fi
twice="$(snapshot "$HOME")"
assert_eq "idempotent tree" "$twice" "$once"
assert_eq "one Thunar action" "$(grep -c '<unique-id>kawaro</unique-id>' "$C/Thunar/uca.xml")" "1"

# AC8: uninstall restores the tree, keeps config.
: > "$SB/log/systemctl.log"
if "$UNINSTALL" > "$SB/log/uninstall.out" 2>&1; then pass; else fail "uninstall exit code"; cat "$SB/log/uninstall.out" >&2; fi
after="$(snapshot "$HOME")"
assert_eq "tree restored" "$after" "$before"
if [ "$after" != "$before" ]; then diff <(echo "$before") <(echo "$after") >&2; fi
assert_eq "Thunar ours gone" "$(grep -c '<unique-id>kawaro</unique-id>' "$C/Thunar/uca.xml")" "0"
assert_contains "Thunar other action kept" "$C/Thunar/uca.xml" "<unique-id>1700000000000001-1</unique-id>"
assert_file "$C/kawaro/config.toml" 644
assert_contains "disable" "$SB/log/systemctl.log" "--user disable --now kawaro.service"

# --purge removes user config too.
"$INSTALL" --no-build > /dev/null 2>&1
if "$UNINSTALL" --purge > /dev/null 2>&1; then pass; else fail "uninstall --purge exit code"; fi
assert_absent "$C/kawaro"
assert_absent "$BIN/kawaro"
drop_sandbox

###########################################################################
echo "== scenario B: nothing present, no systemd, no brand icons"
make_sandbox
guard_home
before="$(snapshot "$HOME")"
export KAWARO_FILE_MANAGERS=none KAWARO_HYPRLAND=0 KAWARO_SYSTEMD=0
export KAWARO_BRAND_DIR="$SB/no-brand"
export KAWARO_THUNAR_SYSTEM_UCA="$FX/uca-system.xml"
BIN="$HOME/.local/bin"; D="$XDG_DATA_HOME"; C="$XDG_CONFIG_HOME"
if "$INSTALL" --no-build > "$SB/log/install.out" 2>&1; then pass; else fail "install exit code"; cat "$SB/log/install.out" >&2; fi
assert_file "$BIN/kawaro" 755
assert_file "$BIN/kawarod" 755
assert_absent "$BIN/kawaro-open-selection"
assert_absent "$D/kio/servicemenus/kawaro.desktop"
assert_absent "$D/nautilus-python/extensions/kawaro.py"
assert_absent "$D/nemo/actions/kawaro.nemo_action"
assert_absent "$C/Thunar/uca.xml"
assert_absent "$C/systemd/user/kawaro.service"
assert_absent "$D/icons/hicolor/scalable/apps/kawaro.svg"
assert_file "$C/autostart/kawaro.desktop" 644
assert_eq "autostart Exec" "$(ini_get "$C/autostart/kawaro.desktop" 'Desktop Entry' Exec)" "$BIN/kawarod"
validate_desktop "$C/autostart/kawaro.desktop"
assert_absent "$SB/log/systemctl.log"
assert_absent "$SB/log/forbidden.log"
assert_contains "icons skipped message" "$SB/log/install.out" "design/brand"
if "$UNINSTALL" > /dev/null 2>&1; then pass; else fail "uninstall exit code"; fi
assert_eq "tree restored" "$(snapshot "$HOME")" "$before"
drop_sandbox

###########################################################################
echo "== scenario C: custom PREFIX, Thunar seeded from system file, target inactive"
make_sandbox
guard_home
before="$(snapshot "$HOME")"
export PREFIX="$HOME/opt/sh"
export KAWARO_FILE_MANAGERS=thunar KAWARO_HYPRLAND=0 KAWARO_SYSTEMD=1 FAKE_GST_ACTIVE=1
export KAWARO_BRAND_DIR="$SB/no-brand"
export KAWARO_THUNAR_SYSTEM_UCA="$FX/uca-system.xml"
C="$XDG_CONFIG_HOME"
if "$INSTALL" --no-build > "$SB/log/install.out" 2>&1; then pass; else fail "install exit code"; cat "$SB/log/install.out" >&2; fi
assert_file "$PREFIX/bin/kawaro" 755
assert_absent "$HOME/.local/bin/kawaro"
assert_contains "Thunar seeded" "$C/Thunar/uca.xml" "<unique-id>1-1</unique-id>"
assert_contains "Thunar command uses PREFIX" "$C/Thunar/uca.xml" "<command>$PREFIX/bin/kawaro open -- %F</command>"
assert_eq "unit ExecStart uses PREFIX" "$(ini_get "$C/systemd/user/kawaro.service" Service ExecStart)" "$PREFIX/bin/kawarod"
assert_contains "enable without --now" "$SB/log/systemctl.log" "--user enable kawaro.service"
assert_not_contains "no start" "$SB/log/systemctl.log" "--now"
assert_contains "exec-once hint" "$SB/log/install.out" "exec-once"
if "$UNINSTALL" > /dev/null 2>&1; then pass; else fail "uninstall exit code"; fi
assert_absent "$C/Thunar/uca.xml"
assert_eq "tree restored" "$(snapshot "$HOME")" "$before"
unset PREFIX FAKE_GST_ACTIVE
drop_sandbox

###########################################################################
echo "== scenario E: default detection (no overrides)"
# lib.sh puts fake dolphin, nautilus, nemo, thunar, hyprctl and systemctl
# first on PATH, so detection finds them all; Nautilus additionally needs the
# real nautilus-python plugin library on this machine.
make_sandbox
guard_home
before="$(snapshot "$HOME")"
export KAWARO_BRAND_DIR="$SB/no-brand"
export KAWARO_THUNAR_SYSTEM_UCA="$FX/uca-system.xml"
D="$XDG_DATA_HOME"; C="$XDG_CONFIG_HOME"
if "$INSTALL" --no-build > "$SB/log/install.out" 2>&1; then pass; else fail "install exit code"; cat "$SB/log/install.out" >&2; fi
assert_file "$D/kio/servicemenus/kawaro.desktop" 755
assert_file "$D/nemo/actions/kawaro.nemo_action" 644
assert_contains "Thunar detected" "$C/Thunar/uca.xml" "<unique-id>kawaro</unique-id>"
assert_file "$HOME/.local/bin/kawaro-open-selection" 755
assert_file "$C/systemd/user/kawaro.service" 644
if compgen -G '/usr/lib*/nautilus/extensions-4/libnautilus-python.so' >/dev/null ||
   compgen -G '/usr/lib/*/nautilus/extensions-4/libnautilus-python.so' >/dev/null; then
  assert_file "$D/nautilus-python/extensions/kawaro.py" 644
else
  assert_absent "$D/nautilus-python/extensions/kawaro.py"
fi
assert_absent "$SB/log/forbidden.log"
if "$UNINSTALL" > /dev/null 2>&1; then pass; else fail "uninstall exit code"; fi
assert_eq "tree restored" "$(snapshot "$HOME")" "$before"
drop_sandbox

###########################################################################
echo "== scenario D: --no-build without binaries fails cleanly"
make_sandbox
guard_home
export KAWARO_BIN_DIR="$SB/empty"
export KAWARO_FILE_MANAGERS=none KAWARO_HYPRLAND=0 KAWARO_SYSTEMD=0
if "$INSTALL" --no-build > "$SB/log/install.out" 2>&1; then fail "install without binaries exited 0"; else pass; fi
assert_absent "$HOME/.local/bin/kawaro"
drop_sandbox

finish test_install
