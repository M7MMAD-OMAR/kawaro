# Shortcuts

## Linux: the drop shortcut

Hold **Super+Z** while dragging files and the hive appears. Let go and it closes. A short tap shows nothing.

- GNOME 48 and later, KDE 6: Kawaro registers the shortcut through the desktop portal. The first time, the desktop asks you to allow it.
- Older desktops and Hyprland: bind `kawaro drop` yourself. The steps are in the [full guide](https://github.com/M7MMAD-OMAR/kawaro/blob/main/docs/drop-shortcut.md).
- Change the hold time with `hold_ms` in [settings](settings.md).

## Windows

Drag a file with the right mouse button and the hive opens during the drag, at the dragged file. Drop it on the cell of the action you want. No key is held while you drag, so Explorer keeps its own drag behavior: a drop outside the hive copies, moves or creates shortcuts as it always does, and the original file is never touched. A short right click shows nothing.
