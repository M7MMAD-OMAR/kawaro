[English](release-notes-0.4.5.md) | [العربية](release-notes-0.4.5.ar.md)

# Kawaro 0.4.5

## Status

This release fixes four small Windows defects found after 0.4.4: the settings window now comes to the front, old update files are cleaned up, the notification header reads Kawaro, and the window titles read Kawaro.

## What to do

- Windows users: install the 0.4.5 MSI (double click or the in-app update), or use the Scoop bucket (see the [distribution guide](distribution.md)).

## Highlights

- Foreground (S-106, S-107): the settings window opens in front of the window you were using, including after an update. The result of the foreground request is now verified instead of trusted, and repeated once after the window shows.
- Cleanup (S-106): downloaded update folders at or below the installed version and old helper copies are removed.
- Names (S-106): the notification header reads Kawaro and the window titles read Kawaro.

## Known gaps

- Linux (Hyprland 0.57): `kawaro drop` without `--at` can swallow the first click (S-105).
- Windows: the executables and the MSI are still TEST-SIGNED with a self-signed certificate, so Windows SmartScreen or Defender may warn ("Windows protected your PC", "Unknown publisher"). Check your download against SHA256SUMS first, then click "More info", then "Run anyway".
- Linux: the focus and activation tests on real GNOME and KDE desktops are still pending.

## Updates

Nothing changes in how updates are verified: a signed manifest from the project's GitHub releases, checked against a public key built into the app, and every package checked against the size and SHA-256 in that manifest.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update.

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the package and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
