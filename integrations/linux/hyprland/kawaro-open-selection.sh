#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copy the selection of the focused window, then open it in Kawaro.
# Bind it in hyprland.conf; see README.md next to this file.
#
# Ctrl+C makes a file manager put the selected files on the clipboard as
# text/uri-list; `kawaro open --from-clipboard` reads them with wl-paste.
# In a terminal Ctrl+C interrupts the running program, so bind this for file
# managers only.
set -eu

# scripts/install.sh replaces this default with the absolute installed path.
KAWARO_BIN="${KAWARO_BIN:-kawaro}"

hyprctl dispatch sendshortcut CTRL,C,activewindow >/dev/null
# Give the file manager time to own the clipboard.
sleep "${KAWARO_COPY_DELAY:-0.15}"
exec "$KAWARO_BIN" open --from-clipboard
