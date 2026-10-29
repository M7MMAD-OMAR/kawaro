[English](release-notes-0.4.1.md) | [العربية](release-notes-0.4.1.ar.md)

# Kawaro 0.4.1

## Status

This is a small follow-up to 0.4.0 for Linux and Windows. It brings a new look to the Settings window, a real Updates page, and a simpler website and README. The Windows build is still test-signed, so read the known gaps before installing.

## Highlights

- Settings app shell (S-094): the Settings window and the welcome window share one shell with a header (logo, the name Kawaro | كوارو and the version), a page rail, centred rows and a footer with Save and Cancel.
- Updates page (S-090): it shows the installed version, a "Check for updates" button and "Update now", with the automatic check, the channel and the install mode as settings.
- Simpler website and README: one download section on the home page, MSI only for Windows, and the separate download and privacy pages are gone. The README now leads with the download.
- CI fix: two timing tests of the drop hive that measure real elapsed time are skipped on CI, where shared runners are too slow for them. They still run locally.

## Known gaps

- Windows: the executables and the MSI are still TEST-SIGNED with a self-signed certificate, not a certificate from a trusted authority, so Windows SmartScreen warns ("Windows protected your PC", "Unknown publisher"). Check your download against SHA256SUMS first, then click "More info", then "Run anyway".
- The Windows warning remains until the app has a trusted signature; the home page now explains the click flow in one plain sentence.
- Windows screenshots of the new Settings window are owner pending; the new layout has been seen on a live Windows 11 machine only in part.
- The Updates page does not show the download percentage yet (S-091).
- The tray menu has no About item yet (S-092).
- The introduction film in the welcome window is still being redone (S-094c).
- Linux: the focus and activation tests on real GNOME and KDE desktops are still pending.
- Signed Linux repositories are not shipped yet; the RPM and DEB packages come from the release page.

## Updates

Nothing changes in how updates are verified: a signed manifest from the project's GitHub releases, checked against a public key built into the app, and every package checked against the size and SHA-256 in that manifest. Use the Updates page in Settings, or `kawaro update --check`, `kawaro update` and `kawaro update --apply` from the terminal.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update.

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the package and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
