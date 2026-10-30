[English](release-notes-0.4.6.md) | [العربية](release-notes-0.4.6.ar.md)

# Kawaro 0.4.6

## Status

When Kawaro cannot convert a file, the hive now says why instead of showing nothing or a bare error: the format is unsupported, an engine is missing (with an install hint), or the selection is mixed. This release also carries a small cleanup of the settings window foreground repeat on Windows, with no change you will notice.

## What to do

- Linux and Windows users: update in the app, install the 0.4.6 package, or use the apt repository or the Scoop bucket (see the [distribution guide](distribution.md)).

## Highlights

- Reasons in the hive (S-096, S-108): a reason line and a suggestion line sit under the centre cell, skipped files get greyed cells, and a request cell for an unsupported format opens a prefilled issue link only when you click it. Nothing is sent until you click.
- Mixed selections: actions run on the supported files only, and the notification names the reason and the next step.

## Known gaps

- The manual checks of S-108 were not run: the hive screenshots (M1), the 150 ms timing check (M2) and the Windows Explorer mixed selection (M3).
- Linux (Hyprland 0.57): `kawaro drop` without `--at` can swallow the first click (S-105).
- Windows: the executables and the MSI are still TEST-SIGNED with a self-signed certificate, so Windows SmartScreen or Defender may warn ("Windows protected your PC", "Unknown publisher"). Check your download against SHA256SUMS first, then click "More info", then "Run anyway".
- Linux: the focus and activation tests on real GNOME and KDE desktops are still pending.

## Updates

Nothing changes in how updates are verified: a signed manifest from the project's GitHub releases, checked against a public key built into the app, and every package checked against the size and SHA-256 in that manifest.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update.

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the package and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
