[English](release-notes-0.4.8.md) | [العربية](release-notes-0.4.8.ar.md)

# Kawaro 0.4.8

## Status

Small features and tooling: the About page gets working actions, notifications get one clear choice, the hive can open at the pointer through `kawaro open --at`, `kawaro doctor --repair` sets up the Hyprland Alt drag bind, and the AppStream file is ready for app stores. Nothing changes in how files are converted.

## What to do

- Linux and Windows users: update in the app, install the 0.4.8 package, or use the apt repository or the Scoop bucket (see the [distribution guide](distribution.md)).
- Hyprland users: run `kawaro doctor --repair` once to write the Alt drag bind in a file Kawaro owns.

## Highlights

- About page (S-094b): buttons that open the website and the source code, a Copy version button that reads Copied for two seconds, and a Check for updates button that opens the Updates page and starts the check.
- Notifications (S-109): one choice with three options, Off, every finished job, or only jobs that took 10 seconds or more. Old settings keep their behaviour.
- Open at the pointer (S-110): `kawaro open --at X,Y` opens the hive at a position, and the Hyprland launcher reads the pointer position once and passes it. An old client without the field still works.
- Hyprland bind repair (S-111): `kawaro doctor --repair` writes, updates and reports the Alt drag bind in a Kawaro owned file and removes stale launchers. `kawaro doctor` also prints a `drop key state:` line.
- App store metadata (S-112): the AppStream file gains an icon, two real screenshots and Arabic release notes. Every install command now comes from one file, `packaging/install-commands.json`, with a status per channel. A `sign-checksums` job signs `SHA256SUMS` on tags and stays idle until its key is configured.

## Known gaps

- The manual checks were not run: the S-094b real clipboard behaviour on desktops, the S-108 hive screenshots (M1), 150 ms timing check (M2) and Windows Explorer mixed selection (M3), the S-110 and S-111 nested Hyprland and live session checks, and the S-105 KWin scripts.
- The `sign-checksums` job has never run, so the signed `SHA256SUMS` file is not yet published or verified.
- Windows: the executables and the MSI are still TEST-SIGNED with a self-signed certificate, so Windows SmartScreen or Defender may warn ("Windows protected your PC", "Unknown publisher"). Check your download against SHA256SUMS first, then click "More info", then "Run anyway".
- Linux: the focus and activation tests on real GNOME and KDE desktops are still pending.

## Updates

Nothing changes in how updates are verified: a signed manifest from the project's GitHub releases, checked against a public key built into the app, and every package checked against the size and SHA-256 in that manifest.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update.

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the package and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
