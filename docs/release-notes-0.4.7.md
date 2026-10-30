[English](release-notes-0.4.7.md) | [العربية](release-notes-0.4.7.ar.md)

# Kawaro 0.4.7

## Status

On Hyprland 0.57, `kawaro drop` without `--at` no longer swallows the first click: a click made while the hive is armed now reaches the file manager. Nothing changes on Windows or on other desktops.

## What to do

- Linux and Windows users: update in the app, install the 0.4.7 package, or use the apt repository or the Scoop bucket (see the [distribution guide](distribution.md)).

## Highlights

- First click kept (S-105): on Hyprland 0.57 the drop catcher is replaced once, at the map time pointer enter, by the `--at` form with its hole, so the first press is no longer swallowed. It was checked in a private Hyprland 0.57 harness.

## Known gaps

- The manual checks of S-108 were not run: the hive screenshots (M1), the 150 ms timing check (M2) and the Windows Explorer mixed selection (M3).
- The S-105 KWin scripts and the drop budget script were not run.
- Windows: the executables and the MSI are still TEST-SIGNED with a self-signed certificate, so Windows SmartScreen or Defender may warn ("Windows protected your PC", "Unknown publisher"). Check your download against SHA256SUMS first, then click "More info", then "Run anyway".
- Linux: the focus and activation tests on real GNOME and KDE desktops are still pending.

## Updates

Nothing changes in how updates are verified: a signed manifest from the project's GitHub releases, checked against a public key built into the app, and every package checked against the size and SHA-256 in that manifest.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update.

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the package and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
