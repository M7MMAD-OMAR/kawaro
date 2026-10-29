#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Post-processes an MSI built by wixl (S-100).
#   scripts/windows/msi-fixup.sh MSI VERSION
# wixl cannot write the File.Version column. Without it the two exes are
# unversioned files, and Windows Installer then keeps an installed copy whose
# modification time is later than its creation time (it assumes the user
# changed it), so an upgrade silently skipped the new exes and StartDaemon
# failed with 2753 (measured on Windows 11). A versioned file in the package
# always replaces an unversioned installed one. VERSION is the product
# version (x.y.z); the file version becomes x.y.z.0.
set -eu
[ "$#" -eq 2 ] || { echo "usage: msi-fixup.sh MSI VERSION" >&2; exit 2; }
msi="$1"
ver="${2%%[-+]*}.0"
command -v msibuild >/dev/null || { echo "msi-fixup: msibuild not found, install msitools" >&2; exit 2; }
for f in CliExe DaemonExe; do
  msibuild "$msi" -q "UPDATE \`File\` SET \`Version\`='$ver' WHERE \`File\`='$f'"
done
