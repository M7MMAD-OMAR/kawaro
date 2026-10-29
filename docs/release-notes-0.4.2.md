[English](release-notes-0.4.2.md) | [العربية](release-notes-0.4.2.ar.md)

# Kawaro 0.4.2

## Status

This is an urgent fix for Windows. Upgrading from an older version with the MSI, or with the in-app update, removed the program files and left a broken Start shortcut. Linux is unchanged.

## What to do

- If you installed 0.4.1 on Windows, run the 0.4.2 MSI once (or reinstall). A fresh install of 0.4.1 was fine; the problem appeared only when upgrading over an older version.
- From 0.4.2 on, upgrades with the MSI and the in-app update keep the program files.

## Highlights

- MSI upgrade keeps the program files (S-097): the old product was removed after the new files were costed, so it deleted `kawaro.exe` and `kawarod.exe` and left only the `actions` folder. The installer now removes the old product after `InstallFinalize`. Measured on Windows 11: installing 0.4.0 and then the fixed MSI leaves all four entries.

## Known gaps

- Windows: the executables and the MSI are still TEST-SIGNED with a self-signed certificate, so Windows SmartScreen warns ("Windows protected your PC", "Unknown publisher"). Check your download against SHA256SUMS first, then click "More info", then "Run anyway".
- The in-app update from the fixed version is not yet checked live (owner pending).
- Linux: the focus and activation tests on real GNOME and KDE desktops are still pending.

## Updates

Nothing changes in how updates are verified: a signed manifest from the project's GitHub releases, checked against a public key built into the app, and every package checked against the size and SHA-256 in that manifest.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update.

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the package and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
