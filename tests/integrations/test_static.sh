#!/usr/bin/env bash
# S-006 AC1, AC5, AC8 and S-007 AC10: keys of the source files, validators,
# the shell linter and the long dash ban. Reads repository files only.
set -u
# shellcheck source=tests/integrations/lib.sh
. "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT" || exit 2

echo "== Dolphin service menu"
D=integrations/linux/dolphin/sbarhive.desktop
if [ -f "$D" ]; then
  assert_file "$D" 755
  assert_eq "Type" "$(ini_get "$D" 'Desktop Entry' Type)" "Service"
  assert_eq "ServiceTypes" "$(ini_get "$D" 'Desktop Entry' X-KDE-ServiceTypes)" "KonqPopupMenu/Plugin"
  assert_eq "MimeType" "$(ini_get "$D" 'Desktop Entry' MimeType)" "all/allfiles;"
  assert_eq "Priority" "$(ini_get "$D" 'Desktop Entry' X-KDE-Priority)" "TopLevel"
  assert_eq "Protocols" "$(ini_get "$D" 'Desktop Entry' X-KDE-Protocols)" "file"
  assert_eq "Actions" "$(ini_get "$D" 'Desktop Entry' Actions)" "sbarhive;"
  assert_eq "action Name" "$(ini_get "$D" 'Desktop Action sbarhive' Name)" "SbarHive"
  assert_eq "action Name[ar]" "$(ini_get "$D" 'Desktop Action sbarhive' 'Name[ar]')" "صبار هايف"
  assert_eq "action Icon" "$(ini_get "$D" 'Desktop Action sbarhive' Icon)" "sbarhive"
  assert_eq "action Exec" "$(ini_get "$D" 'Desktop Action sbarhive' Exec)" "sbarhive open -- %F"
  assert_eq "one action section" "$(grep -c '^\[Desktop Action ' "$D")" "1"
  assert_contains "token note" "$D" "XDG_ACTIVATION_TOKEN"
else
  fail "missing $D"
fi

echo "== Nemo action"
N=integrations/linux/nemo/sbarhive.nemo_action
if [ -f "$N" ]; then
  assert_eq "Selection" "$(ini_get "$N" 'Nemo Action' Selection)" "notnone"
  assert_eq "Extensions" "$(ini_get "$N" 'Nemo Action' Extensions)" "any;"
  assert_eq "Exec" "$(ini_get "$N" 'Nemo Action' Exec)" "sbarhive open -- %F"
  assert_eq "Name" "$(ini_get "$N" 'Nemo Action' Name)" "SbarHive"
  assert_eq "Name[ar]" "$(ini_get "$N" 'Nemo Action' 'Name[ar]')" "صبار هايف"
  assert_eq "Icon-Name" "$(ini_get "$N" 'Nemo Action' Icon-Name)" "sbarhive"
else
  fail "missing $N"
fi

echo "== Thunar snippet"
T=integrations/linux/thunar/sbarhive-uca.xml
if [ -f "$T" ]; then
  check "snippet is well formed XML" xmllint --noout "$T"
  assert_contains "unique-id" "$T" "<unique-id>sbarhive</unique-id>"
  assert_contains "command" "$T" "<command>sbarhive open -- %F</command>"
else
  fail "missing $T"
fi

echo "== packaging"
U=packaging/sbarhive.service
if [ -f "$U" ]; then
  assert_eq "ExecStart" "$(ini_get "$U" Service ExecStart)" "%h/.local/bin/sbarhived"
  assert_eq "Restart" "$(ini_get "$U" Service Restart)" "on-failure"
  assert_eq "PartOf" "$(ini_get "$U" Unit PartOf)" "graphical-session.target"
  assert_eq "After" "$(ini_get "$U" Unit After)" "graphical-session.target"
  assert_eq "WantedBy" "$(ini_get "$U" Install WantedBy)" "graphical-session.target"
else
  fail "missing $U"
fi
APP=packaging/io.github.m7mmad_omar.SbarHive.desktop
AUTO=packaging/sbarhive-autostart.desktop
META=packaging/io.github.m7mmad_omar.SbarHive.metainfo.xml
for f in "$APP" "$AUTO"; do
  if [ -f "$f" ]; then
    if command -v desktop-file-validate >/dev/null; then
      check "desktop-file-validate $f" desktop-file-validate "$f"
    else
      echo "  skip desktop-file-validate (not installed)"
    fi
  else
    fail "missing $f"
  fi
done
[ -f "$APP" ] && assert_eq "app Icon" "$(ini_get "$APP" 'Desktop Entry' Icon)" "sbarhive"
[ -f "$APP" ] && assert_eq "app Name[ar]" "$(ini_get "$APP" 'Desktop Entry' 'Name[ar]')" "صبار هايف"
if [ -f "$META" ]; then
  check "metainfo well formed" xmllint --noout "$META"
  if command -v appstreamcli >/dev/null; then
    check "appstreamcli validate" appstreamcli validate --no-net "$META"
  else
    echo "  skip appstreamcli (not installed)"
  fi
else
  fail "missing $META"
fi

echo "== Hyprland helper"
H=integrations/linux/hyprland/sbarhive-open-selection.sh
assert_file "$H" 755
R=integrations/linux/hyprland/README.md
if [ -f "$R" ]; then
  assert_contains "mouse bind" "$R" "bind = SUPER, mouse:273, exec, ~/.local/bin/sbarhive-open-selection"
  check "README has Arabic" grep -q 'ربط' "${R%.md}.ar.md"
  check "README has a keyboard bind" grep -qE '^bind = SUPER( SHIFT)?, [A-Z0-9]+, exec' "$R"
else
  fail "missing $R"
fi

echo "== scripts"
for s in scripts/install.sh scripts/uninstall.sh; do
  assert_file "$s" 755
done

echo "== shellcheck"
if ! command -v shellcheck >/dev/null; then
  sudo -n dnf install -y ShellCheck >/dev/null 2>&1 || true
fi
if command -v shellcheck >/dev/null; then
  mapfile -t SH < <(ls scripts/install.sh scripts/uninstall.sh "$H" tests/integrations/*.sh 2>/dev/null)
  check "shellcheck" shellcheck -x "${SH[@]}"
else
  echo "  skip shellcheck (not installed)"
fi

echo "== python syntax"
for p in integrations/linux/nautilus/sbarhive.py integrations/linux/thunar/merge_uca.py; do
  check "python parses $p" python3 -c "import ast,sys; ast.parse(open(sys.argv[1]).read())" "$p"
done

echo "== no long dashes"
OWNED=(integrations/linux packaging scripts/install.sh scripts/uninstall.sh tests/integrations
       backlog/S-006-file-manager-integrations.md backlog/S-007-install.md)
hits="$(LC_ALL=C.UTF-8 grep -rnP '\x{2014}|\x{2013}' "${OWNED[@]}" 2>/dev/null || true)"
assert_eq "em or en dash in owned files" "$hits" ""

finish test_static
