#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copy the selection of the focused window, then open it in Kawaro.
# Bind it in hyprland.conf; see README.md next to this file.
#
# Ctrl+C makes a file manager put the selected files on the clipboard as
# text/uri-list; `kawaro open --from-clipboard` reads them with wl-paste.
# The hive opens at the pointer (`--at`), as `hyprctl cursorpos` prints it.
# In a terminal Ctrl+C interrupts the running program, so bind this for file
# managers only.
set -eu

# scripts/install.sh replaces this default with the absolute installed path.
KAWARO_BIN="${KAWARO_BIN:-kawaro}"

# The pointer, read once: "X, Y" in global logical coordinates (S-110). With
# no pointer the helper stops; it never opens at 0,0 or centred instead.
pos="$(hyprctl cursorpos 2>/dev/null | tr -d ' ')" || pos=""
if ! printf '%s' "$pos" |
  grep -Eq '^-?[0-9]+(\.[0-9]+)?,-?[0-9]+(\.[0-9]+)?$'; then
  echo "kawaro-open-selection: cannot read the pointer from hyprctl cursorpos" >&2
  exit 1
fi

hyprctl dispatch sendshortcut CTRL,C,activewindow >/dev/null
# Give the file manager time to own the clipboard.
sleep "${KAWARO_COPY_DELAY:-0.15}"
exec "$KAWARO_BIN" open --from-clipboard --at "$pos"
