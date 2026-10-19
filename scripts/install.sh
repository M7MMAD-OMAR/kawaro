#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# User-local install of SbarHive. No sudo, idempotent.
#
#   scripts/install.sh [--no-build]
#
# Environment:
#   PREFIX                      install prefix, default $HOME/.local
#   XDG_DATA_HOME, XDG_CONFIG_HOME, XDG_STATE_HOME   honoured as usual
#   SBARHIVE_BIN_DIR            built binaries, default <repo>/target/release
#   SBARHIVE_BRAND_DIR          icons, default <repo>/design/brand
#   SBARHIVE_FILE_MANAGERS      comma list (dolphin,nautilus,nemo,thunar) or
#                               "none"; default: detect what is installed
#   SBARHIVE_HYPRLAND           1 or 0; default: 1 when hyprctl exists
#   SBARHIVE_SYSTEMD            1 or 0; default: 1 when a systemd user
#                               manager answers
#   SBARHIVE_THUNAR_SYSTEM_UCA  default /etc/xdg/Thunar/uca.xml
#
# Every file and directory this script creates is listed in
# $XDG_STATE_HOME/sbarhive/install-manifest; scripts/uninstall.sh removes
# exactly those.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="io.github.m7mmad_omar.SbarHive"

PREFIX="${PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
CONF="${XDG_CONFIG_HOME:-$HOME/.config}"
STATE="${XDG_STATE_HOME:-$HOME/.local/state}"
BIN_SRC="${SBARHIVE_BIN_DIR:-${CARGO_TARGET_DIR:-$REPO_ROOT/target}/release}"
BRAND="${SBARHIVE_BRAND_DIR:-$REPO_ROOT/design/brand}"
SYS_UCA="${SBARHIVE_THUNAR_SYSTEM_UCA:-/etc/xdg/Thunar/uca.xml}"
MANIFEST="$STATE/sbarhive/install-manifest"

BUILD=1
for arg in "$@"; do
  case "$arg" in
    --no-build) BUILD=0 ;;
    -h|--help) sed -n '3,22p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "install.sh: unknown option $arg" >&2; exit 64 ;;
  esac
done

die () { echo "install.sh: $*" >&2; exit 1; }

# Paths end up in Exec lines, a systemd unit and Python source. Quoting all of
# those for arbitrary bytes is not worth it; refuse the few characters that
# would need it.
case "$PREFIX" in
  *[\"\$\`\\%]*) die "PREFIX must not contain \" \$ \` \\ or %: $PREFIX" ;;
esac

ENTRIES=()
record () { ENTRIES+=("$1"); }

# ensure_dir DIR: mkdir -p, recording every directory that did not exist.
ensure_dir () {
  local d="$1" missing=()
  while [ ! -d "$d" ]; do missing+=("$d"); d="$(dirname "$d")"; done
  [ "${#missing[@]}" -eq 0 ] && return 0
  mkdir -p "$1"
  local m
  for m in "${missing[@]}"; do record "d $m"; done
}

# put SRC DEST MODE: copy a file.
put () {
  ensure_dir "$(dirname "$2")"
  install -m "$3" "$1" "$2"
  record "f $2"
}

# put_text DEST MODE TEXT: write TEXT plus a newline, atomically.
put_text () {
  local dest="$1" mode="$2" text="$3" tmp
  ensure_dir "$(dirname "$dest")"
  tmp="$(mktemp "$(dirname "$dest")/.sbarhive.XXXXXX")"
  printf '%s\n' "$text" > "$tmp"
  chmod "$mode" "$tmp"
  mv -f "$tmp" "$dest"
  record "f $dest"
}

# replace_all TEXT FROM TO: literal replacement of every FROM. Both sides are
# quoted so glob characters and "&" in paths stay literal.
replace_all () { printf '%s' "${1//"$2"/"$3"}"; }

# Argument form of a path for Exec lines, systemd and Thunar commands.
quote_arg () {
  case "$1" in
    *[!A-Za-z0-9/._+@=-]*) printf '"%s"' "$1" ;;
    *) printf '%s' "$1" ;;
  esac
}

SBARHIVE_ARG="$(quote_arg "$BIN/sbarhive")"
SBARHIVED_ARG="$(quote_arg "$BIN/sbarhived")"

old_manifest_has () { [ -f "$MANIFEST" ] && grep -qxF -- "$1" "$MANIFEST"; }

want_fm () {
  local name="$1"
  if [ -n "${SBARHIVE_FILE_MANAGERS+x}" ]; then
    case ",$SBARHIVE_FILE_MANAGERS," in *",$name,"*) return 0 ;; *) return 1 ;; esac
  fi
  case "$name" in
    dolphin) command -v dolphin >/dev/null 2>&1 ;;
    nautilus) command -v nautilus >/dev/null 2>&1 && have_nautilus_python ;;
    nemo) command -v nemo >/dev/null 2>&1 ;;
    thunar) command -v thunar >/dev/null 2>&1 ;;
    *) return 1 ;;
  esac
}

have_nautilus_python () {
  local so
  for so in /usr/lib*/nautilus/extensions-4/libnautilus-python.so \
            /usr/lib/*/nautilus/extensions-4/libnautilus-python.so; do
    [ -e "$so" ] && return 0
  done
  return 1
}

want_flag () {
  # want_flag VALUE_OR_EMPTY DETECT_COMMAND...
  local value="$1"; shift
  case "$value" in
    1) return 0 ;;
    0) return 1 ;;
    *) "$@" >/dev/null 2>&1 ;;
  esac
}

systemd_user_available () {
  command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1
}

DONE_EN=()
DONE_AR=()
HINTS=()
done_item () { DONE_EN+=("$1"); DONE_AR+=("$2"); }

########################################################################
# 1. Binaries
if [ "$BUILD" -eq 1 ]; then
  command -v cargo >/dev/null 2>&1 || die "cargo not found; install Rust or use --no-build"
  (cd "$REPO_ROOT" && cargo build --release --locked --bin sbarhive --bin sbarhived)
fi
for b in sbarhive sbarhived; do
  [ -x "$BIN_SRC/$b" ] || die "missing $BIN_SRC/$b (build first, or drop --no-build)"
done
put "$BIN_SRC/sbarhive" "$BIN/sbarhive" 0755
put "$BIN_SRC/sbarhived" "$BIN/sbarhived" 0755
done_item "binaries in $BIN" "الملفات التنفيذية في $BIN"

########################################################################
# 2. Icons, desktop entry, AppStream metadata
ICONS="$DATA/icons/hicolor"
if [ -f "$BRAND/sbarhive.svg" ]; then
  put "$BRAND/sbarhive.svg" "$ICONS/scalable/apps/sbarhive.svg" 0644
  done_item "icon" "الأيقونة"
else
  echo "icons: no sbarhive.svg in design/brand yet ($BRAND), skipped | الأيقونات: لا يوجد sbarhive.svg في design/brand بعد، تم التخطي"
fi
if [ -f "$BRAND/sbarhive-symbolic.svg" ]; then
  put "$BRAND/sbarhive-symbolic.svg" "$ICONS/symbolic/apps/sbarhive-symbolic.svg" 0644
fi

app="$(cat "$REPO_ROOT/packaging/$APP_ID.desktop")"
put_text "$DATA/applications/$APP_ID.desktop" 0644 "$(replace_all "$app" "Exec=sbarhive " "Exec=$SBARHIVE_ARG ")"
put "$REPO_ROOT/packaging/$APP_ID.metainfo.xml" "$DATA/metainfo/$APP_ID.metainfo.xml" 0644
done_item "desktop entry and AppStream metadata" "ملف سطح المكتب وبيانات AppStream"

########################################################################
# 3. File manager integrations
INT="$REPO_ROOT/integrations/linux"
if want_fm dolphin; then
  menu="$(cat "$INT/dolphin/sbarhive.desktop")"
  put_text "$DATA/kio/servicemenus/sbarhive.desktop" 0755 "$(replace_all "$menu" "Exec=sbarhive " "Exec=$SBARHIVE_ARG ")"
  done_item "Dolphin menu" "قائمة Dolphin"
  HINTS+=("Dolphin: reopen Dolphin windows | أعد فتح نوافذ Dolphin")
fi
if want_fm nautilus; then
  ext="$(cat "$INT/nautilus/sbarhive.py")"
  pybin="$(python3 -c 'import json,sys; print(json.dumps(sys.argv[1], ensure_ascii=False))' "$BIN/sbarhive")"
  put_text "$DATA/nautilus-python/extensions/sbarhive.py" 0644 "$(replace_all "$ext" 'SBARHIVE_BIN = "sbarhive"' "SBARHIVE_BIN = $pybin")"
  done_item "Nautilus menu" "قائمة Nautilus"
  HINTS+=("Nautilus: run nautilus -q | Nautilus: نفّذ nautilus -q")
fi
if want_fm nemo; then
  act="$(cat "$INT/nemo/sbarhive.nemo_action")"
  put_text "$DATA/nemo/actions/sbarhive.nemo_action" 0644 "$(replace_all "$act" "Exec=sbarhive " "Exec=$SBARHIVE_ARG ")"
  done_item "Nemo action" "إجراء Nemo"
fi
if want_fm thunar; then
  uca="$CONF/Thunar/uca.xml"
  if [ -e "$uca" ] && ! old_manifest_has "thunar created $uca"; then
    record "thunar existing $uca"
  else
    record "thunar created $uca"
  fi
  ensure_dir "$CONF/Thunar"
  python3 "$INT/thunar/merge_uca.py" add --file "$uca" --seed "$SYS_UCA" \
    --command "$SBARHIVE_ARG open -- %F" >/dev/null
  done_item "Thunar action" "إجراء Thunar"
  HINTS+=("Thunar: run thunar -q | Thunar: نفّذ thunar -q")
fi

########################################################################
# 4. Hyprland helper (never edits hyprland.conf)
if want_flag "${SBARHIVE_HYPRLAND:-}" command -v hyprctl; then
  helper="$(cat "$INT/hyprland/sbarhive-open-selection.sh")"
  # shellcheck disable=SC2016 # literal text to find in the helper
  put_text "$BIN/sbarhive-open-selection" 0755 "$(replace_all "$helper" '${SBARHIVE_BIN:-sbarhive}' "\${SBARHIVE_BIN:-$BIN/sbarhive}")"
  done_item "Hyprland helper" "سكربت Hyprland"
  HINTS+=("Hyprland bind (add to hyprland.conf) | ربط Hyprland (أضفه إلى hyprland.conf):"
          "  bind = SUPER SHIFT, H, exec, $BIN/sbarhive-open-selection"
          "  bind = SUPER, mouse:273, exec, $BIN/sbarhive-open-selection"
          "  (mouse:273 clashes with the default resizewindow bindm | يتعارض مع ربط تغيير الحجم الافتراضي)")
fi

########################################################################
# 5. Daemon autostart: systemd user unit, else XDG autostart
if want_flag "${SBARHIVE_SYSTEMD:-}" systemd_user_available; then
  unit="$(cat "$REPO_ROOT/packaging/sbarhive.service")"
  put_text "$CONF/systemd/user/sbarhive.service" 0644 "$(replace_all "$unit" "ExecStart=%h/.local/bin/sbarhived" "ExecStart=$SBARHIVED_ARG")"
  record "systemd sbarhive.service"
  if old_manifest_has "f $CONF/autostart/sbarhive.desktop"; then
    rm -f "$CONF/autostart/sbarhive.desktop"
  fi
  systemctl --user daemon-reload
  if systemctl --user is-active --quiet graphical-session.target; then
    systemctl --user enable --now sbarhive.service
    done_item "systemd user service, running" "خدمة systemd للمستخدم، تعمل الآن"
  else
    systemctl --user enable sbarhive.service
    done_item "systemd user service, enabled" "خدمة systemd للمستخدم، مفعّلة"
    HINTS+=("graphical-session.target is not active (plain Hyprland?). The daemon starts on the first sbarhive open, or add: | الهدف graphical-session.target غير نشط. تبدأ الخدمة عند أول sbarhive open، أو أضف:"
            "  exec-once = $BIN/sbarhived")
  fi
  if ! systemctl --user show-environment 2>/dev/null | grep -q '^WAYLAND_DISPLAY='; then
    HINTS+=("systemd user manager lacks WAYLAND_DISPLAY; add to your session start | أضف إلى بدء الجلسة:"
            "  dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP")
  fi
else
  auto="$(cat "$REPO_ROOT/packaging/sbarhive-autostart.desktop")"
  put_text "$CONF/autostart/sbarhive.desktop" 0644 "$(replace_all "$auto" "Exec=sbarhived" "Exec=$SBARHIVED_ARG")"
  done_item "autostart entry (no systemd user session)" "تشغيل تلقائي (لا توجد جلسة systemd للمستخدم)"
fi

########################################################################
# 6. Manifest: old entries plus new ones, without duplicates.
ensure_dir "$(dirname "$MANIFEST")"
{
  [ -f "$MANIFEST" ] && cat "$MANIFEST"
  printf '%s\n' "${ENTRIES[@]}"
} | awk '!seen[$0]++' > "$MANIFEST.tmp"
mv -f "$MANIFEST.tmp" "$MANIFEST"

case ":$PATH:" in
  *":$BIN:"*) ;;
  *) HINTS+=("$BIN is not in PATH | المجلد $BIN ليس في PATH") ;;
esac

########################################################################
echo
echo "اكتمل تثبيت صبار هايف:"
for item in "${DONE_AR[@]}"; do echo "  - $item"; done
echo "SbarHive installed:"
for item in "${DONE_EN[@]}"; do echo "  - $item"; done
if [ "${#HINTS[@]}" -gt 0 ]; then
  echo "Next steps | الخطوات التالية:"
  for h in "${HINTS[@]}"; do echo "  $h"; done
fi
echo "Uninstall | الإزالة: scripts/uninstall.sh"
