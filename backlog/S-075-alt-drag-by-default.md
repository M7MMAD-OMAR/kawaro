# S-075 Alt then drag works out of the box from the installed package
Status: approved (2026-10-26, manager, maintainer's top priority)

## Story
As a user of the installed package, I want to hold Alt while dragging files and drop them on the hive, with nothing to set up, so that the drag path (my main way to use Kawaro) works the day I install it, as it did in the developer install.

## Background (found 2026-10-26)
- On the maintainer's machine Alt-drag stopped after the developer install in `~/.local/bin` was replaced by the RPM: his Hyprland binds called `$HOME/.local/bin/kawaro` (`~/.config/hypr/custom/keybinds.lua`, comment "Needs the developer install"). Nothing in the package registers the shortcut, so a normal user gets no Alt-drag at all.
- The maintainer's rule: the drag, started with Alt, is THE way to use Kawaro. The right-click menu is secondary.

## Decisions
1. The shortcut is Alt held during a drag (`drop --hold`), default on, on every supported desktop: Hyprland (installed bind snippet written by `kawaro integration install`, run by the package post-install for the active user where possible, and by the daemon at first start otherwise), KDE and GNOME (GlobalShortcuts portal, S-014e). Never a path under `$HOME/.local`; always the binary found through `PATH` or `/usr/bin`.
2. The daemon checks at start whether the Alt-hold binding is present and working for this session and, if not, repairs it silently and reports the state in `kawaro doctor`.
3. The key is configurable (S-076); Alt is always the default.

## Acceptance criteria
- AC1: after `dnf install` of the RPM on a fresh user with Hyprland, holding Alt past `hold_ms` during a real file drag shows the drop hive at the drag position, with no manual step. Verified in the private Hyprland harness.
- AC2: same on KWin (private `--virtual`) through the portal path.
- AC3: no generated file or bind contains `.local/bin`; a test greps the generator output.
- AC4: removing the binding and restarting the daemon restores it; `kawaro doctor` prints the binding state in one line.
- AC5: an install over an old developer install (binds pointing to `~/.local/bin`) rewrites them to the installed binary.
- AC6: the 150 ms budget after `hold_ms` still holds.

## Out of scope
The shortcut picker UI (S-076), X11 (S-015).

## Review log
(empty)
