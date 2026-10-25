#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Lays out a system-wide install under DESTDIR/usr for the RPM and DEB
# packages. scripts/install.sh is the user-local install; this is the same
# set of files with system paths, minus Thunar (its menu lives in each
# user's uca.xml, which a package must not edit).
#
#   packaging/linux/stage.sh BIN_DIR DESTDIR
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN_DIR="$1"
DEST="$2"
APP_ID="io.github.m7mmad_omar.Kawaro"
U="$DEST/usr"
INT="$ROOT/integrations/linux"

rm -rf "$DEST"
install -Dm755 "$BIN_DIR/kawaro" "$U/bin/kawaro"
install -Dm755 "$BIN_DIR/kawarod" "$U/bin/kawarod"
install -Dm755 "$INT/hyprland/kawaro-open-selection.sh" "$U/bin/kawaro-open-selection"

install -Dm644 "$ROOT/packaging/$APP_ID.desktop" "$U/share/applications/$APP_ID.desktop"
install -Dm644 "$ROOT/packaging/$APP_ID.metainfo.xml" "$U/share/metainfo/$APP_ID.metainfo.xml"
install -Dm644 "$ROOT/design/brand/kawaro.svg" "$U/share/icons/hicolor/scalable/apps/kawaro.svg"
install -Dm644 "$ROOT/design/brand/kawaro-symbolic.svg" "$U/share/icons/hicolor/symbolic/apps/kawaro-symbolic.svg"
for s in 16 32 48 64 128 256 512; do
  install -Dm644 "$ROOT/design/brand/png/kawaro-$s.png" "$U/share/icons/hicolor/${s}x$s/apps/kawaro.png"
done

# The service starts with every graphical session; the package scripts
# enable it globally. `kawaro open` also starts it on demand.
sed 's|^ExecStart=.*|ExecStart=/usr/bin/kawarod|' "$ROOT/packaging/kawaro.service" \
  | grep -v '^# Install to\|^#   systemctl\|^# scripts/install.sh' \
  > "$DEST/kawaro.service"
install -Dm644 "$DEST/kawaro.service" "$U/lib/systemd/user/kawaro.service"
rm "$DEST/kawaro.service"

install -Dm644 "$INT/dolphin/kawaro.desktop" "$U/share/kio/servicemenus/kawaro.desktop"
install -Dm644 "$INT/nautilus/kawaro.py" "$U/share/nautilus-python/extensions/kawaro.py"
install -Dm644 "$INT/nemo/kawaro.nemo_action" "$U/share/nemo/actions/kawaro.nemo_action"

install -Dm644 "$ROOT/LICENSE" "$U/share/licenses/kawaro/LICENSE"
install -Dm644 "$ROOT/README.md" "$U/share/doc/kawaro/README.md"
install -Dm644 "$ROOT/README.ar.md" "$U/share/doc/kawaro/README.ar.md"
install -Dm644 "$ROOT/CHANGELOG.md" "$U/share/doc/kawaro/CHANGELOG.md"
