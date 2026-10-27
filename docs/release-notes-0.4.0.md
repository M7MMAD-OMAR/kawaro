[English](release-notes-0.4.0.md) | [العربية](release-notes-0.4.0.ar.md)

# Kawaro 0.4.0

## Status

This is the first official release of Kawaro, for Linux and for Windows. It follows the public beta 0.4.0-beta.1 and adds what the beta testers reported: the app icon and the name Kawaro | كوارو everywhere, a Start entry that opens a window, a drag trigger that never changes what Explorer does with your drag, a first-run welcome with a sample to try, and an honest download page. The Windows builds are test-signed, not publisher verified: read the known gaps before installing.

## What works

- Select files in your file manager, open the hexagonal hive from the right click menu and pick an action. The result is saved next to the original, which is never modified. Arabic and English, with full right to left layout.
- Linux (Wayland first, Fedora and Ubuntu families): RPM and DEB packages, the drop hive and the global shortcut through the system portal. The hive is glass where the compositor blurs (KDE, Hyprland), opaque on GNOME, and its motion respects the system's reduced motion setting.
- Windows 10 and 11, 64 bit: a portable zip and a per-user MSI that needs no administrator rights, with the right click menu entry, the tray icon, the app icon in Start and the name Kawaro | كوارو. Clicking Kawaro in Start opens the Settings window at once. To convert while you drag: drag the file with the right mouse button and the hive opens during the drag. No key is held while you drag, so Explorer keeps its own drag behavior.
- First run: the first window shows the whole flow in a short demonstration, can try it on a sample image with one click, and leads to the simplified Settings.
- Updates from inside the app on both systems, described below.

## Known gaps

- Windows: the executables and the MSI are TEST-SIGNED with a self-signed certificate (subject "Kawaro test signing"), not a certificate from a trusted authority, so this is not a verified publisher. Windows SmartScreen will say "Windows protected your PC" with "Unknown publisher". Check your download against SHA256SUMS first, then click "More info", then "Run anyway". A trusted OV certificate from a real certificate authority is still needed for full trust; the warning fades only as the same certificate builds reputation.
- Nothing on Windows is verified on a live Windows machine by this release: the builds are cross compiled on Linux and tested there. The live checklist (S-079) is run by the project owner, and every Windows live result is owner pending until that report exists.
- Linux: the focus and activation tests on real GNOME and KDE desktops are still pending. On those desktops the hive may open without keyboard focus.
- The tray menu has not been checked on real desktops yet, and the in-app download percentage in the tray tooltip has not been tested live.
- The Windows update helper, which installs a new MSI while the app is closed, is built and unit tested but has not run on a live Windows machine yet.
- Install and update from signed Linux repositories are not shipped yet; the RPM and DEB packages come from the release page, and the package manager warns that the OpenPGP check is skipped until a signed repository exists.

## Updates

Kawaro checks once a day for a new version. The check reads a signed manifest from the project's GitHub releases; the signature is verified against a public key built into the app before anything is downloaded, and every package is checked against the size and SHA-256 in that manifest. There are two channels, `beta` and `stable`; this release is offered on the stable channel, and a beta install is offered the final release too. To turn the automatic check off, set `update_check = false` in `config.toml`. A manual check ignores that setting: run `kawaro update --check` to check, `kawaro update` to download and verify, and `kawaro update --apply` to install (S-054c). There is no menu entry for it yet.

On Windows an update installs quietly for your user. On Linux the package is downloaded and verified in the background, and it installs only after you choose "Update now" in the tray menu and approve the system password prompt.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update, which GitHub sees like any other download (your address and a fixed user agent).

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the packages and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
