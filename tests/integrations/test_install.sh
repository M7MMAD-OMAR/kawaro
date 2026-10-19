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
  mkdir -p "$HOME/.local/bin" "$XDG_CONFIG_HOME/Thunar" "$XDG_CONFIG_HOME/sbarhive"
  printf '#!/bin/sh\n' > "$HOME/.local/bin/unrelated-tool"
  chmod 0755 "$HOME/.local/bin/unrelated-tool"
  cp "$FX/uca-user.xml" "$XDG_CONFIG_HOME/Thunar/uca.xml"
  echo 'theme = "dark"' > "$XDG_CONFIG_HOME/sbarhive/config.toml"
}

brand_dir () {
  mkdir -p "$SB/brand"
  cp "$FX/icon.svg" "$SB/brand/sbarhive.svg"
  cp "$FX/icon.svg" "$SB/brand/sbarhive-symbolic.svg"
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
export SBARHIVE_FILE_MANAGERS=dolphin,nautilus,nemo,thunar SBARHIVE_HYPRLAND=1 SBARHIVE_SYSTEMD=1
SBARHIVE_BRAND_DIR="$(brand_dir)"; export SBARHIVE_BRAND_DIR
export SBARHIVE_THUNAR_SYSTEM_UCA="$FX/uca-system.xml"

if "$INSTALL" --no-build > "$SB/log/install1.out" 2>&1; then pass; else fail "install #1 exit code"; cat "$SB/log/install1.out" >&2; fi

assert_file "$BIN/sbarhive" 755
assert_file "$BIN/sbarhived" 755
assert_file "$BIN/sbarhive-open-selection" 755
assert_file "$D/applications/io.github.m7mmad_omar.SbarHive.desktop" 644
assert_file "$D/metainfo/io.github.m7mmad_omar.SbarHive.metainfo.xml" 644
assert_file "$D/icons/hicolor/scalable/apps/sbarhive.svg" 644
assert_file "$D/icons/hicolor/symbolic/apps/sbarhive-symbolic.svg" 644
assert_file "$D/kio/servicemenus/sbarhive.desktop" 755
assert_file "$D/nautilus-python/extensions/sbarhive.py" 644
assert_file "$D/nemo/actions/sbarhive.nemo_action" 644
assert_file "$C/systemd/user/sbarhive.service" 644
assert_absent "$C/autostart/sbarhive.desktop"
assert_eq "binary copied" "$(cat "$BIN/sbarhive")" "$(cat "$SB/release/sbarhive")"

# AC6: absolute paths everywhere a launcher runs us.
assert_eq "Dolphin Exec" "$(ini_get "$D/kio/servicemenus/sbarhive.desktop" 'Desktop Action sbarhive' Exec)" "$BIN/sbarhive open -- %F"
assert_eq "Nemo Exec" "$(ini_get "$D/nemo/actions/sbarhive.nemo_action" 'Nemo Action' Exec)" "$BIN/sbarhive open -- %F"
assert_eq "app Exec" "$(ini_get "$D/applications/io.github.m7mmad_omar.SbarHive.desktop" 'Desktop Entry' Exec)" "$BIN/sbarhive open -- %F"
assert_eq "unit ExecStart" "$(ini_get "$C/systemd/user/sbarhive.service" Service ExecStart)" "$BIN/sbarhived"
assert_contains "Nautilus binary" "$D/nautilus-python/extensions/sbarhive.py" "SBARHIVE_BIN = \"$BIN/sbarhive\""
assert_contains "Hyprland binary" "$BIN/sbarhive-open-selection" "$BIN/sbarhive"
assert_contains "Thunar command" "$C/Thunar/uca.xml" "<command>$BIN/sbarhive open -- %F</command>"
assert_contains "Thunar other action" "$C/Thunar/uca.xml" "<unique-id>1700000000000001-1</unique-id>"
validate_desktop "$D/applications/io.github.m7mmad_omar.SbarHive.desktop"

# AC5: systemd calls, through the fake only.
assert_contains "daemon-reload" "$SB/log/systemctl.log" "--user daemon-reload"
assert_contains "enable --now" "$SB/log/systemctl.log" "--user enable --now sbarhive.service"
assert_absent "$SB/log/forbidden.log"

# AC4, AC9: bind line and bilingual summary.
assert_contains "bind line printed" "$SB/log/install1.out" "bind = SUPER, mouse:273, exec, $BIN/sbarhive-open-selection"
assert_contains "Arabic summary" "$SB/log/install1.out" "اكتمل تثبيت صبار هايف"
assert_contains "English summary" "$SB/log/install1.out" "SbarHive installed"
check "hyprland.conf untouched" test ! -e "$C/hypr/hyprland.conf"

# AC7: second run gives the same tree.
once="$(snapshot "$HOME")"
if "$INSTALL" --no-build > "$SB/log/install2.out" 2>&1; then pass; else fail "install #2 exit code"; fi
twice="$(snapshot "$HOME")"
assert_eq "idempotent tree" "$twice" "$once"
assert_eq "one Thunar action" "$(grep -c '<unique-id>sbarhive</unique-id>' "$C/Thunar/uca.xml")" "1"

# AC8: uninstall restores the tree, keeps config.
: > "$SB/log/systemctl.log"
if "$UNINSTALL" > "$SB/log/uninstall.out" 2>&1; then pass; else fail "uninstall exit code"; cat "$SB/log/uninstall.out" >&2; fi
after="$(snapshot "$HOME")"
assert_eq "tree restored" "$after" "$before"
if [ "$after" != "$before" ]; then diff <(echo "$before") <(echo "$after") >&2; fi
assert_eq "Thunar ours gone" "$(grep -c '<unique-id>sbarhive</unique-id>' "$C/Thunar/uca.xml")" "0"
assert_contains "Thunar other action kept" "$C/Thunar/uca.xml" "<unique-id>1700000000000001-1</unique-id>"
assert_file "$C/sbarhive/config.toml" 644
assert_contains "disable" "$SB/log/systemctl.log" "--user disable --now sbarhive.service"

# --purge removes user config too.
"$INSTALL" --no-build > /dev/null 2>&1
if "$UNINSTALL" --purge > /dev/null 2>&1; then pass; else fail "uninstall --purge exit code"; fi
assert_absent "$C/sbarhive"
assert_absent "$BIN/sbarhive"
drop_sandbox

###########################################################################
echo "== scenario B: nothing present, no systemd, no brand icons"
make_sandbox
guard_home
before="$(snapshot "$HOME")"
export SBARHIVE_FILE_MANAGERS=none SBARHIVE_HYPRLAND=0 SBARHIVE_SYSTEMD=0
export SBARHIVE_BRAND_DIR="$SB/no-brand"
export SBARHIVE_THUNAR_SYSTEM_UCA="$FX/uca-system.xml"
BIN="$HOME/.local/bin"; D="$XDG_DATA_HOME"; C="$XDG_CONFIG_HOME"
if "$INSTALL" --no-build > "$SB/log/install.out" 2>&1; then pass; else fail "install exit code"; cat "$SB/log/install.out" >&2; fi
assert_file "$BIN/sbarhive" 755
assert_file "$BIN/sbarhived" 755
assert_absent "$BIN/sbarhive-open-selection"
assert_absent "$D/kio/servicemenus/sbarhive.desktop"
assert_absent "$D/nautilus-python/extensions/sbarhive.py"
assert_absent "$D/nemo/actions/sbarhive.nemo_action"
assert_absent "$C/Thunar/uca.xml"
assert_absent "$C/systemd/user/sbarhive.service"
assert_absent "$D/icons/hicolor/scalable/apps/sbarhive.svg"
assert_file "$C/autostart/sbarhive.desktop" 644
assert_eq "autostart Exec" "$(ini_get "$C/autostart/sbarhive.desktop" 'Desktop Entry' Exec)" "$BIN/sbarhived"
validate_desktop "$C/autostart/sbarhive.desktop"
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
export SBARHIVE_FILE_MANAGERS=thunar SBARHIVE_HYPRLAND=0 SBARHIVE_SYSTEMD=1 FAKE_GST_ACTIVE=1
export SBARHIVE_BRAND_DIR="$SB/no-brand"
export SBARHIVE_THUNAR_SYSTEM_UCA="$FX/uca-system.xml"
C="$XDG_CONFIG_HOME"
if "$INSTALL" --no-build > "$SB/log/install.out" 2>&1; then pass; else fail "install exit code"; cat "$SB/log/install.out" >&2; fi
assert_file "$PREFIX/bin/sbarhive" 755
assert_absent "$HOME/.local/bin/sbarhive"
assert_contains "Thunar seeded" "$C/Thunar/uca.xml" "<unique-id>1-1</unique-id>"
assert_contains "Thunar command uses PREFIX" "$C/Thunar/uca.xml" "<command>$PREFIX/bin/sbarhive open -- %F</command>"
assert_eq "unit ExecStart uses PREFIX" "$(ini_get "$C/systemd/user/sbarhive.service" Service ExecStart)" "$PREFIX/bin/sbarhived"
assert_contains "enable without --now" "$SB/log/systemctl.log" "--user enable sbarhive.service"
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
export SBARHIVE_BRAND_DIR="$SB/no-brand"
export SBARHIVE_THUNAR_SYSTEM_UCA="$FX/uca-system.xml"
D="$XDG_DATA_HOME"; C="$XDG_CONFIG_HOME"
if "$INSTALL" --no-build > "$SB/log/install.out" 2>&1; then pass; else fail "install exit code"; cat "$SB/log/install.out" >&2; fi
assert_file "$D/kio/servicemenus/sbarhive.desktop" 755
assert_file "$D/nemo/actions/sbarhive.nemo_action" 644
assert_contains "Thunar detected" "$C/Thunar/uca.xml" "<unique-id>sbarhive</unique-id>"
assert_file "$HOME/.local/bin/sbarhive-open-selection" 755
assert_file "$C/systemd/user/sbarhive.service" 644
if compgen -G '/usr/lib*/nautilus/extensions-4/libnautilus-python.so' >/dev/null ||
   compgen -G '/usr/lib/*/nautilus/extensions-4/libnautilus-python.so' >/dev/null; then
  assert_file "$D/nautilus-python/extensions/sbarhive.py" 644
else
  assert_absent "$D/nautilus-python/extensions/sbarhive.py"
fi
assert_absent "$SB/log/forbidden.log"
if "$UNINSTALL" > /dev/null 2>&1; then pass; else fail "uninstall exit code"; fi
assert_eq "tree restored" "$(snapshot "$HOME")" "$before"
drop_sandbox

###########################################################################
echo "== scenario D: --no-build without binaries fails cleanly"
make_sandbox
guard_home
export SBARHIVE_BIN_DIR="$SB/empty"
export SBARHIVE_FILE_MANAGERS=none SBARHIVE_HYPRLAND=0 SBARHIVE_SYSTEMD=0
if "$INSTALL" --no-build > "$SB/log/install.out" 2>&1; then fail "install without binaries exited 0"; else pass; fi
assert_absent "$HOME/.local/bin/sbarhive"
drop_sandbox

finish test_install
