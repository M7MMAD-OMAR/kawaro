[English](release-notes-0.4.4.md) | [العربية](release-notes-0.4.4.ar.md)

# Kawaro 0.4.4

## Status

This release makes the Alt gesture on Linux more reliable, adds support for Hyprland 0.57, and adds two ways to install: a signed apt repository and a Scoop bucket.

## What to do

- Linux users: update with the RPM or DEB, from inside the app, or switch to the apt repository on Debian and Ubuntu (install line in [apt.md](distribution-channels/apt.md)).
- Windows users: install the 0.4.4 MSI (double click or the in-app update), or use the Scoop bucket (see the [distribution guide](distribution.md)).

## Highlights

- Alt chords (S-102): Alt+a and other Alt chords no longer leave a stray hive on Hyprland, because a key hook sends `kawaro drop --other-key`. `kawaro doctor` prints a `drop key:` line and says "unknown" when it cannot tell. The shortcut docs gain an "Alt tap" section.
- Hyprland 0.57 (S-104): Hyprland 0.57 enters a new layer surface under a still pointer, which used to hide the hive when Alt was pressed before the drag. This is handled, and it builds on the late drag fix of 0.4.3 (S-095).
- New install options (S-103): a signed apt repository served by GitHub Pages (`https://m7mmad-omar.github.io/kawaro-apt`) and a Scoop bucket (`https://github.com/M7MMAD-OMAR/scoop-kawaro`). AUR and Fedora COPR are not live yet.

## Known gaps

- Linux (Hyprland 0.57): `kawaro drop` without `--at` can swallow the first click (S-105).
- Windows: after an update, the settings window can open behind another window. Look for it in the taskbar.
- Windows: the executables and the MSI are still TEST-SIGNED with a self-signed certificate, so Windows SmartScreen or Defender may warn ("Windows protected your PC", "Unknown publisher"). Check your download against SHA256SUMS first, then click "More info", then "Run anyway".
- Linux: the focus and activation tests on real GNOME and KDE desktops are still pending.

## Updates

Nothing changes in how updates are verified: a signed manifest from the project's GitHub releases, checked against a public key built into the app, and every package checked against the size and SHA-256 in that manifest.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update.

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the package and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
