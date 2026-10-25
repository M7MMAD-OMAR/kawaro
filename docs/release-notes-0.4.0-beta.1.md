[English](release-notes-0.4.0-beta.1.md) | [العربية](release-notes-0.4.0-beta.1.ar.md)

# Kawaro 0.4.0-beta.1

## Status

This is the first public beta of Kawaro, for Linux and for Windows. It is a prerelease: expect rough edges, and please report what you find. Nothing in it is production ready, and the stable channel does not offer it.

## What works

- Select files in your file manager, open the hexagonal hive from the right click menu and pick an action. The result is saved next to the original, which is never modified. Arabic and English, with full right to left layout.
- Linux (Wayland first, Fedora and Ubuntu families): RPM and DEB packages, the drop hive and the global shortcut through the system portal.
- Windows 10 and 11, 64 bit: a portable zip and a per-user MSI that needs no administrator rights, with the right click menu entry and the tray icon.
- Updates from inside the app on both systems, described below.

## Known gaps

- Windows: the zip and the MSI are unsigned, because there is no code signing certificate yet. Windows SmartScreen may say "Windows protected your PC". Click "More info", then "Run anyway", after you have checked your download against SHA256SUMS.
- Nothing on Windows has been verified on a live Windows machine yet: the builds are cross compiled on Linux and tested there, and the live checklist (S-052) is the next step.
- Windows: the drop hive (drag files from Explorer onto a cell, S-047) and the notification when a job ends with an Open folder button (S-049) are built and unit tested but have not run on a live Windows machine yet. Until the app id is registered, the notification may be labelled with another app's name.
- Linux: the focus and activation tests on real GNOME and KDE desktops are still pending. On those desktops the hive may open without keyboard focus.
- The tray menu has not been checked on real desktops yet, and the in-app download percentage in the tray tooltip has not been tested live.
- The Windows update helper, which installs a new MSI while the app is closed, is built and unit tested but has not run on a live Windows machine yet.

## Updates

Kawaro checks once a day for a new version. The check reads a signed manifest from the project's GitHub releases; the signature is verified against a public key built into the app before anything is downloaded, and every package is checked against the size and SHA-256 in that manifest. There are two channels, `beta` and `stable`. A beta build follows the `beta` channel, so you will be offered later betas and the final release. To turn the automatic check off, set `update_check = false` in `config.toml`. A manual check ignores that setting: run `kawaro update --check` to check, `kawaro update` to download and verify, and `kawaro update --apply` to install (S-054c). There is no menu entry for it yet.

On Windows an update installs quietly for your user. On Linux the package is downloaded and verified in the background, and it installs only after you choose "Update now" in the tray menu and approve the system password prompt.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update, which GitHub sees like any other download (your address and a fixed user agent).

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the packages and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
