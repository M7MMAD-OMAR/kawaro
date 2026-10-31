[English](announcement.md) | [العربية](announcement.ar.md)

# Kawaro 0.5.0 announcement text

## Release note

- Kawaro 0.5.0 adds X11 support.
- On X11 the hive opens as an ordinary window at the pointer, kept inside the screen.
- `kawaro drop` opens it above other windows, so a file you drag can be dropped on a cell.
- Nothing changes on Wayland or Windows, nor in how files are converted.
- Your files stay on your machine: no uploads, no telemetry, and the original file is never modified.
- Known gaps: X11 was checked only in a private virtual X session (Xvfb with xfwm4), not on a real GNOME or KDE X11 desktop. The Windows build is signed with a self-signed certificate, so SmartScreen warns on first run.

## Rename note

Kawaro is the new name of the project, which was called SbarHive until now. The name comes from كوّارة (kawwara), an old Arabic word for a beehive, and it reads the same in Arabic and Latin script: ka-wa-ro. The mark did not change: it is still the saguaro cactus built from hive cells, and the single cell with the cactus cut out is the app icon.

## Post

Kawaro 0.5.0 adds X11 support: the hive opens at the pointer and you can drop a file on a cell. Everything runs on your machine; the original is never touched. Free, GPLv3. X11 checked only in a private virtual session; Windows build is self-signed. https://kawaro.sbarah.com/
