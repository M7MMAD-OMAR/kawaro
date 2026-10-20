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
APP_ID="io.github.m7mmad_omar.SbarHive"
U="$DEST/usr"
INT="$ROOT/integrations/linux"

rm -rf "$DEST"
install -Dm755 "$BIN_DIR/sbarhive" "$U/bin/sbarhive"
install -Dm755 "$BIN_DIR/sbarhived" "$U/bin/sbarhived"
install -Dm755 "$INT/hyprland/sbarhive-open-selection.sh" "$U/bin/sbarhive-open-selection"

install -Dm644 "$ROOT/packaging/$APP_ID.desktop" "$U/share/applications/$APP_ID.desktop"
install -Dm644 "$ROOT/packaging/$APP_ID.metainfo.xml" "$U/share/metainfo/$APP_ID.metainfo.xml"
install -Dm644 "$ROOT/design/brand/sbarhive.svg" "$U/share/icons/hicolor/scalable/apps/sbarhive.svg"
install -Dm644 "$ROOT/design/brand/sbarhive-symbolic.svg" "$U/share/icons/hicolor/symbolic/apps/sbarhive-symbolic.svg"
for s in 16 32 48 64 128 256 512; do
  install -Dm644 "$ROOT/design/brand/png/sbarhive-$s.png" "$U/share/icons/hicolor/${s}x$s/apps/sbarhive.png"
done

# The service starts with every graphical session; the package scripts
# enable it globally. `sbarhive open` also starts it on demand.
sed 's|^ExecStart=.*|ExecStart=/usr/bin/sbarhived|' "$ROOT/packaging/sbarhive.service" \
  | grep -v '^# Install to\|^#   systemctl\|^# scripts/install.sh' \
  > "$DEST/sbarhive.service"
install -Dm644 "$DEST/sbarhive.service" "$U/lib/systemd/user/sbarhive.service"
rm "$DEST/sbarhive.service"

install -Dm644 "$INT/dolphin/sbarhive.desktop" "$U/share/kio/servicemenus/sbarhive.desktop"
install -Dm644 "$INT/nautilus/sbarhive.py" "$U/share/nautilus-python/extensions/sbarhive.py"
install -Dm644 "$INT/nemo/sbarhive.nemo_action" "$U/share/nemo/actions/sbarhive.nemo_action"

install -Dm644 "$ROOT/LICENSE" "$U/share/licenses/sbarhive/LICENSE"
install -Dm644 "$ROOT/README.md" "$U/share/doc/sbarhive/README.md"
install -Dm644 "$ROOT/CHANGELOG.md" "$U/share/doc/sbarhive/CHANGELOG.md"
