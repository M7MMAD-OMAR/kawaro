#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copy the selection of the focused window, then open it in SbarHive.
# Bind it in hyprland.conf; see README.md next to this file.
#
# Ctrl+C makes a file manager put the selected files on the clipboard as
# text/uri-list; `sbarhive open --from-clipboard` reads them with wl-paste.
# In a terminal Ctrl+C interrupts the running program, so bind this for file
# managers only.
set -eu

# scripts/install.sh replaces this default with the absolute installed path.
SBARHIVE_BIN="${SBARHIVE_BIN:-sbarhive}"

hyprctl dispatch sendshortcut CTRL,C,activewindow >/dev/null
# Give the file manager time to own the clipboard.
sleep "${SBARHIVE_COPY_DELAY:-0.15}"
exec "$SBARHIVE_BIN" open --from-clipboard
