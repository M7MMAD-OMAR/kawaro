#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Builds kawaro-VERSION-1.x86_64.rpm and kawaro_VERSION_amd64.deb from
# release binaries. Needs rpmbuild and dpkg-deb.
#
#   packaging/linux/build-packages.sh BIN_DIR OUT_DIR
#
# Build the binaries on the oldest system you support (CI uses Ubuntu
# 22.04): they run on any newer glibc.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN_DIR="$(cd "$1" && pwd)"
mkdir -p "$2"
OUT="$(cd "$2" && pwd)"
# A hyphen means a prerelease in Cargo but a final release in rpm and dpkg;
# a tilde sorts below the plain version in both (0.4.0~beta.1 < 0.4.0).
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1 | tr - '~')"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

"$ROOT/packaging/linux/stage.sh" "$BIN_DIR" "$WORK/stage"

# RPM
rpmbuild -bb "$ROOT/packaging/linux/kawaro.spec" \
  --define "_topdir $WORK/rpm" \
  --define "pkg_version $VERSION" \
  --define "stage $WORK/stage" >/dev/null 2>"$WORK/rpmbuild.log" || { cat "$WORK/rpmbuild.log"; exit 1; }
cp "$WORK"/rpm/RPMS/x86_64/kawaro-"$VERSION"-1*.rpm "$OUT/kawaro-$VERSION-1.x86_64.rpm"

# DEB
D="$WORK/deb"
cp -a "$WORK/stage" "$D"
mkdir -p "$D/DEBIAN" "$D/usr/share/doc/kawaro"
mv "$D/usr/share/licenses/kawaro/LICENSE" "$D/usr/share/doc/kawaro/copyright"
rm -r "$D/usr/share/licenses"
GLIBC="$(objdump -T "$BIN_DIR/kawarod" "$BIN_DIR/kawaro" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -Vu | tail -1)"
cat > "$D/DEBIAN/control" <<CONTROL
Package: kawaro
Version: $VERSION
Architecture: amd64
Maintainer: MUHMAD HAJ HMDO <73592103+M7MMAD-OMAR@users.noreply.github.com>
Installed-Size: $(du -sk "$D/usr" | cut -f1)
Depends: libc6 (>= $GLIBC), libgcc-s1, libfontconfig1, libfreetype6
Recommends: ffmpeg, ghostscript, libreoffice-writer | libreoffice-core, wl-clipboard, curl
Suggests: python3-nautilus
Section: utils
Priority: optional
Homepage: https://github.com/M7MMAD-OMAR/kawaro
Description: Local file converter with a honeycomb picker in your file manager
 Kawaro converts and processes files entirely on your machine. Select
 files in Dolphin, Nautilus or Nemo, choose Kawaro from the right-click
 menu, and pick an action from the hexagonal hive. The result is saved next
 to the original, which is never modified. Arabic and English.
CONTROL
cat > "$D/DEBIAN/postinst" <<'SCRIPT'
#!/bin/sh
set -e
if [ "$1" = configure ]; then
  systemctl --global enable kawaro.service >/dev/null 2>&1 || true
fi
SCRIPT
cat > "$D/DEBIAN/prerm" <<'SCRIPT'
#!/bin/sh
set -e
if [ "$1" = remove ]; then
  systemctl --global disable kawaro.service >/dev/null 2>&1 || true
fi
SCRIPT
chmod 755 "$D/DEBIAN/postinst" "$D/DEBIAN/prerm"
dpkg-deb --root-owner-group --build "$D" "$OUT/kawaro_${VERSION}_amd64.deb" >/dev/null

ls -l "$OUT"
