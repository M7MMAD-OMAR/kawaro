#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Removes what scripts/install.sh installed.
#
#   scripts/uninstall.sh [--purge]
#
# Reads $XDG_STATE_HOME/sbarhive/install-manifest and removes exactly the
# files and directories listed there, takes our action out of Thunar's
# uca.xml and disables the systemd user unit. Without a manifest it removes
# the known install paths. User settings in $XDG_CONFIG_HOME/sbarhive stay
# unless --purge is given; --purge also removes SbarHive data, cache and
# state directories.
# Same environment as install.sh (PREFIX, XDG_*, SBARHIVE_THUNAR_SYSTEM_UCA).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="io.github.m7mmad_omar.SbarHive"

PREFIX="${PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
CONF="${XDG_CONFIG_HOME:-$HOME/.config}"
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}"
STATE="${XDG_STATE_HOME:-$HOME/.local/state}"
SYS_UCA="${SBARHIVE_THUNAR_SYSTEM_UCA:-/etc/xdg/Thunar/uca.xml}"
MANIFEST="$STATE/sbarhive/install-manifest"
MERGE="$REPO_ROOT/integrations/linux/thunar/merge_uca.py"

PURGE=0
for arg in "$@"; do
  case "$arg" in
    --purge) PURGE=1 ;;
    -h|--help) sed -n '3,14p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "uninstall.sh: unknown option $arg" >&2; exit 64 ;;
  esac
done

ENTRIES=()
if [ -f "$MANIFEST" ]; then
  mapfile -t ENTRIES < "$MANIFEST"
else
  echo "No manifest at $MANIFEST; removing the known install paths | لا توجد قائمة تثبيت، تُحذف المسارات المعروفة"
  for f in "$BIN/sbarhive" "$BIN/sbarhived" "$BIN/sbarhive-open-selection" \
           "$DATA/icons/hicolor/scalable/apps/sbarhive.svg" \
           "$DATA/icons/hicolor/symbolic/apps/sbarhive-symbolic.svg" \
           "$DATA/applications/$APP_ID.desktop" "$DATA/metainfo/$APP_ID.metainfo.xml" \
           "$DATA/kio/servicemenus/sbarhive.desktop" \
           "$DATA/nautilus-python/extensions/sbarhive.py" \
           "$DATA/nemo/actions/sbarhive.nemo_action" \
           "$CONF/autostart/sbarhive.desktop" "$CONF/systemd/user/sbarhive.service"; do
    [ -e "$f" ] && ENTRIES+=("f $f")
  done
  [ -f "$CONF/Thunar/uca.xml" ] && ENTRIES+=("thunar existing $CONF/Thunar/uca.xml")
  [ -f "$CONF/systemd/user/sbarhive.service" ] && ENTRIES+=("systemd sbarhive.service")
fi

FILES=()
DIRS=()
UCA_FILE=""
UCA_CREATED=0
SYSTEMD=0
for e in "${ENTRIES[@]}"; do
  case "$e" in
    "f "*) FILES+=("${e#f }") ;;
    "d "*) DIRS+=("${e#d }") ;;
    "thunar created "*) UCA_FILE="${e#thunar created }"; UCA_CREATED=1 ;;
    "thunar existing "*) [ -z "$UCA_FILE" ] && UCA_FILE="${e#thunar existing }" ;;
    "systemd "*) SYSTEMD=1 ;;
  esac
done

# 1. Stop and disable the daemon before its binary goes away.
if [ "$SYSTEMD" -eq 1 ] && command -v systemctl >/dev/null 2>&1; then
  systemctl --user disable --now sbarhive.service >/dev/null 2>&1 || true
fi

# 2. Thunar: drop our action, keep every other one.
if [ -n "$UCA_FILE" ] && [ -f "$UCA_FILE" ]; then
  if [ "$UCA_CREATED" -eq 1 ]; then
    python3 "$MERGE" remove --file "$UCA_FILE" --seed "$SYS_UCA" >/dev/null
  else
    python3 "$MERGE" remove --file "$UCA_FILE" >/dev/null
  fi
fi

# 3. Files, then the manifest, then directories we created, deepest first,
#    only when empty.
for f in "${FILES[@]}"; do rm -f -- "$f"; done
rm -f -- "$MANIFEST"
if [ "${#DIRS[@]}" -gt 0 ]; then
  printf '%s\n' "${DIRS[@]}" | awk '{ print length($0) "\t" $0 }' | sort -rn | cut -f2- |
    while IFS= read -r d; do rmdir -- "$d" 2>/dev/null || true; done
fi

if [ "$SYSTEMD" -eq 1 ] && command -v systemctl >/dev/null 2>&1; then
  systemctl --user daemon-reload >/dev/null 2>&1 || true
fi

# 4. Optional purge of user data.
if [ "$PURGE" -eq 1 ]; then
  rm -rf -- "$CONF/sbarhive" "$DATA/sbarhive" "$CACHE/sbarhive" "$STATE/sbarhive"
  echo "Settings and data removed | حُذفت الإعدادات والبيانات"
else
  [ -d "$CONF/sbarhive" ] && echo "Settings kept in $CONF/sbarhive (use --purge) | الإعدادات باقية في $CONF/sbarhive (استخدم --purge)"
fi

echo "SbarHive removed | أُزيل صبار هايف"
