[English](release-notes-0.5.0.md) | [العربية](release-notes-0.5.0.ar.md)

# Kawaro 0.5.0

## Status

X11 support. On an X11 session the hive opens as an ordinary window at the pointer, kept inside the screen, and `kawaro drop` opens it above other windows so a file you drag can be dropped on a cell. Nothing changes on Wayland or Windows, and nothing changes in how files are converted.

X11 was checked only in a private virtual X session (Xvfb with the xfwm4 window manager). It was not checked on a real GNOME or KDE X11 desktop, so treat it as a first version and report what you see.

## What to do

- Linux and Windows users: update in the app, install the 0.5.0 package, or use the apt repository or the Scoop bucket (see the [distribution guide](distribution.md)).
- X11 users: open files from the file manager menu as before; for drop, run `kawaro drop` and then drag (see the [drop shortcut guide](drop-shortcut.md)).

## Highlights

- Hive window on X11 (S-113): the hive is placed at the pointer and kept inside the screen, with an opaque plate in the `surface-0` colour. It takes focus, and Esc or moving focus elsewhere closes it.
- Private session checks (S-114): `tests/x11/run.sh` starts a private X session and measures placement, focus, dismissal, the plate and the 150 ms budget without touching the desktop you work in.
- Drop cell (S-115): `kawaro drop` on X11 opens the hive always on top, and a file dragged onto a cell runs that action. The original file is left untouched. The hive opens at the screen centre for a drop and does not follow the pointer, and the shortcut during a drag is not supported on X11.
- Placement fix (S-116): under xfwm4 the hive used to open at the screen centre for every pointer. The window is now moved once to the planned position after it is mapped, and the `hive surface` log line reports the real position.

## Known gaps

- X11 was checked only in a private virtual X session with xfwm4. Not checked: a real GNOME or KDE X11 desktop, window managers other than xfwm4, Shift reaching a drag source as a move request, and Nautilus or Dolphin as drag sources.
- The earlier manual checks that were pending in 0.4.8 are still pending: the S-094b real clipboard behaviour, the S-108 manual checks M1 to M3, the S-110 and S-111 nested Hyprland and live session checks, and the S-105 KWin scripts.
- Windows: the executables and the MSI are still TEST-SIGNED with a self-signed certificate, so Windows SmartScreen or Defender may warn ("Windows protected your PC", "Unknown publisher"). Check your download against SHA256SUMS first, then click "More info", then "Run anyway".
- Linux: the focus and activation tests on real GNOME and KDE desktops (Wayland) are still pending.

## Updates

Nothing changes in how updates are verified: a signed manifest from the project's GitHub releases, checked against a public key built into the app, and every package checked against the size and SHA-256 in that manifest.

## Privacy

No telemetry, no accounts and no processing servers. Your files never leave your machine. The only network traffic is the update check and the download of an update.

## Verify your download

Every release lists the file `SHA256SUMS`. Download it next to the package and run `sha256sum -c SHA256SUMS --ignore-missing` in that folder; every file you downloaded must say OK. On Windows, `certutil -hashfile <file> SHA256` prints the value to compare with the line in `SHA256SUMS`.
