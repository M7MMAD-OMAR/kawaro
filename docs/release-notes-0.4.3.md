[English](release-notes-0.4.3.md) | [العربية](release-notes-0.4.3.ar.md)

# Kawaro 0.4.3

## Status

This release fixes upgrading the Windows MSI, makes the in-app update clear from start to finish, and fixes the Alt and drag gesture on Linux.

## What to do

- Windows users of 0.4.0 or 0.4.2: install the 0.4.3 MSI (double click or the in-app update). Upgrading from those versions used to fail with error 1603 and roll back; it now replaces the files.
- Linux users: update with the RPM or DEB, or from inside the app.

## Highlights

- MSI upgrade really replaces the files (S-100): Windows Installer kept an installed exe it judged modified by the user, and the upgrade failed with 1603 (StartDaemon error 2753). The two exes now carry a file version and every component GUID changes with each build, so the old product is removed first and the new files are written. User data in `%APPDATA%` is untouched.
- The update page shows what really happens (S-099, S-101): the installed and the new version at the start, the download percent with its size, a line while installing, an automatic restart that reopens settings with "Updated to X" and a short What's new, a plain message when an update fails, and a retry after a failure.
- Alt then a late drag shows the hive on Linux (S-095): the hive appears whether Alt is pressed before or after the drag starts, and every wait for it ends after `hold_max_ms` (default 10 seconds).
- The welcome film is redone (S-094c): a live vector scene, sharp at any window size, with a Play and Pause button; nothing autoplays.

## Known gaps

- Windows: after an update, the settings window can open behind another window. Look for it in the taskbar.
- Linux (Hyprland): an Alt chord such as Alt+Tab can show a stray hive for up to 10 seconds (S-102).
- Windows: the executables and the MSI are still TEST-SIGNED with a self-signed certificate, so Windows SmartScreen or Defender may warn ("Windows protected your PC", "Unknown publisher"). Check your download against SHA256SUMS first, then click "More info", then "Run anyway".
- Linux: the focus and activation tests on real GNOME and KDE desktops are still pending.

## Updates

Nothing changes in how updates are verified: a signed manifest from the project's GitHub releases, checked against a public key built into the app, and every package checked against the size and SHA-256 in that manifest.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update.

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the package and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
