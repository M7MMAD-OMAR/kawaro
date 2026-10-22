# ADR 0003: Dropping a file on a cell during a drag

Status: accepted
Date: 2026-10-20

## Context

The maintainer wants a new interaction: the user starts dragging a file in the file manager, presses a shortcut of their choice while still dragging, the hive appears, and dropping the file on a cell runs its action. Does this work on Wayland in GNOME, KDE and Hyprland, then X11, and what is the simplest route?

The spike in `spikes/drop/` ran only in private headless compositors (KWin 6.7.5 virtual, GNOME Shell 50.5 headless), isolated as in `tests/e2e/run.sh`. The drag source was a GTK 4 window that offers copy and move like a file manager.

## Findings

(verified) means a headless run or a line read in source; (assumed) means an untested inference.

1. **winit 0.30 has no drag and drop on Wayland (verified).** `DroppedFile` and `HoveredFile` exist in winit 0.30.13 on X11, macOS and Windows only; nothing in `src/platform_impl/linux/wayland/` uses `wl_data_device`. Support is in winit 0.31 beta (PR #4571, `winit-wayland/src/dnd.rs`), and Slint 1.18.1 is pinned to winit 0.30.
2. **Slint 1.18.1 does not deliver external drops (verified).** It has `DropArea` and `BackendDragEvent`, but they are reached through the internal `WindowInner::process_drag_event`, which the winit adapter never calls. So cell picking uses our `geometry.rs`, and the highlight is a property set from Rust.
3. **The receiver works on a second connection to winit's display (verified).** The spike takes `wl_display` and `wl_surface` from raw-window-handle, opens a `Connection` with `Backend::from_foreign_display` on its own event queue and thread, binds `wl_data_device_manager` and filters events by winit's surface pointer. It accepts `text/uri-list` over the cells only, copy only (`set_actions(Copy, Copy)`), so a source offering move never deletes the original after the drop. It picks the cell with `Layout::pick` from `crates/ui/src/geometry.rs`. No clash with the clipboard, since Slint's arboard opens a separate connection (`Connection::connect_to_env` in wl-clipboard-rs) with data-control. winit merge request #4729 notes that Hyprland 0.56.2 sends drags only to the first `wl_data_device` a client binds; neither winit 0.30 nor Slint binds one on this connection, so ours is first, but this was not tested on Hyprland (assumed).
4. **The global shortcut fires during a drag on GNOME and KDE (verified by running, 3 of 3 each).**
   - GNOME: an accelerator registered with `grab_accelerator`, the mechanism xdg-desktop-portal-gnome's GlobalShortcuts uses, with `ActionMode.NORMAL` only like it. It fired during the drag and the mode stayed NORMAL. Mutter handles shortcuts in `meta_display_handle_event` before the Wayland handlers and does not check client drags.
   - KDE: a shortcut registered through KGlobalAccel, the KDE portal's path, fired during the drag. `DragAndDropInputFilter::keyboardKey` takes only Esc, and the next filter (`GlobalShortcutFilter`) does not check drags.
   - Hyprland 0.56.2 (verified in source only, not run): `CKeybindManager::onKeyEvent` does not return early during a drag; only Esc cancels it.
   - GNOME: the shortcut does not work if the activities overview opens, for example when the drag crosses the hot corner, because the mode becomes OVERVIEW (assumed from source).
5. **A window that appears during a drag receives the drop (verified on GNOME and KDE, 3 of 3).** Started after the drag began, the spike got `enter` and motion, the drop on cell 2 delivered the right path, and the source reported `deleted=False`. `enter` needs no keyboard focus: on GNOME (smart) the drop worked with focus on the source, while KWin (Low) gave the window focus, so stacking order decides, not focus (finding 6). On KWin and Hyprland `enter` arrives only after the pointer moves (verified in source).
6. **Stacking order is the top risk (verified).** Under GNOME's default (smart) and KWin's default (Low) the window appeared above the file manager. Under GNOME strict and KWin Extreme the new unfocused window went under the source window and got no drop. The fix is an xdg-activation token; only the GNOME portal sends one with `Activated` (`activation_token`). The KDE and Hyprland portals send empty options.
   - **"Hive first, then drag" failed (verified on GNOME and KDE).** Pressing on the file raised the source window above the hive, the drop never arrived and the source cancelled the drag. The source was maximised, the worst case, but the guaranteed order on Wayland is pressing during the drag.
7. **X11 (assumed, not run).** winit 0.30 gives `HoveredFile` on enter and `DroppedFile` on drop, with no position, and accepts the drop on the whole window with `XdndActionPrivate`. So we read the pointer ourselves (`QueryPointer`) on drop. The shortcut during a drag probably fails, because the source grabs the pointer and often the keyboard, and a passive `XGrabKey` does not work against an active grab. X11 uses "hive first, then drag", with `WindowLevel::AlwaysOnTop`, which winit honours on X11, so a raised file manager does not cover the hive. This is not available on Wayland.

### Alternatives, by reliability on GNOME, KDE and Hyprland

1. **Shortcut during the drag, then drop.** Proven on GNOME and KDE by running, on Hyprland in source, with the finding 6 limit under strict policies. Pressing before the drag is not relied on for Wayland (finding 6).
2. **`sbarhive drop` on a custom system shortcut.** Set in GNOME keyboard settings, KDE shortcuts, or a Hyprland `bind` line. Needs no portal but carries no activation token on any desktop. GNOME custom shortcuts are assumed to take the same path as finding 4, but we tested `grab_accelerator`, not gsd-media-keys shortcuts (assumed).
3. **The right click menu.** Stays the guaranteed entry point, unrelated to dragging.
4. **Dropping on a taskbar icon.** Works on KDE (`requestOpenUrls`); not on GNOME, whose dash accepts only app icons (verified in source); Hyprland has no default bar.
5. **A permanent drop target or screen corner.** Needs wlr-layer-shell, which GNOME lacks. Rejected.
6. **The tray icon (StatusNotifierItem).** The protocol has no drop. Rejected.

### Configurable shortcut

- **org.freedesktop.portal.GlobalShortcuts:** in GNOME since 48 (xdg-desktop-portal-gnome), changed in Settings, Apps, "Global shortcuts". In KDE 6 (version 2), changed in System Settings, Shortcuts. In Hyprland (xdg-desktop-portal-hyprland) the shortcut binds only through a config line `bind = MOD, KEY, global, <appid>:<id>`, with no `ConfigureShortcuts`.
- **ashpd 0.13.13:** MIT, `global_shortcuts` feature, depends on zbus 5 with tokio by default or async-io. Not tried; an app outside flatpak may need to register its id with the portal (`org.freedesktop.host.portal.Registry`) (assumed).

## Decision

- One drop receiver in `crates/ui`, modelled on the spike, with its own thread and queue, rebound to the new surface on every `present` because Slint destroys the surface on hide (ADR 0001, finding 5). On X11, winit events plus `QueryPointer`.
- A "waiting for drop" mode: the hive does not close on focus loss or an outside click, only on `leave`, cancel, a timeout or Esc. The file list is read on `enter`, so cells are built for the real file type before the drop.
- Phase one: `sbarhive drop`, bound by the user to a custom system shortcut, with binding docs for each desktop. Works only under default policies; fails with GNOME strict and KWin Extreme (finding 6). Phase two: register the shortcut through the portal with ashpd, passing `activation_token` on GNOME to get past finding 6.
- Do not wait for Slint to move to winit 0.31. If the upgrade becomes possible and Slint delivers drops to `DropArea`, the private receiver is removed in a separate task.

## Consequences

Still open:

- [ ] Hyprland by running, not only in source (no safe headless mode here).
- [ ] Real Dolphin and Nautilus as the drag source instead of a GTK 4 window.
- [ ] Finding 6: is the portal token enough on GNOME strict? What fixes KDE at Extreme?
- [ ] X11 by running: the shortcut during a drag, and the drop with `QueryPointer` and `AlwaysOnTop`.
- [ ] GNOME custom shortcuts (gsd-media-keys) during a drag.
- [ ] The Hyprland first `wl_data_device` limit (finding 3).
- [ ] `enter` in Mutter without pointer motion (assumed from source).

## Amendments

### S-019 (2026-10-21, manager decision)

- **Holding the key:** `sbarhive drop --hold` keeps the drop hive while the key is held, and `sbarhive drop --release` closes it when the key is let go. The drag leaving the hive does not close it while the key is held; it closes on drop or release. A 10 s timeout with no enter stays as a safety net. A press without `--hold` behaves as in S-014. Implemented in S-019e.
- **Drag position:** `sbarhive drop --at X,Y` in global logical coordinates where the position is available (Hyprland: `hyprctl cursorpos -j`), otherwise the centre and then one move on the first drag motion (ADR 0004, S-019 amendment).

### S-019e (2026-10-21, long press)

- **Show after a threshold:** `sbarhive drop --hold` is only a press event; the hive appears once the key has been held for `hold_ms` (key in `config.toml`, default 400 ms, accepted 100 to 2000). The gate is pure logic in `sbarhive_core::hold`, and the timer lives in the service, not the client. A release before the threshold cancels the timer, so a brief tap shows nothing. This replaces the 300 ms saved release from the S-019 amendment.
- **The 150 ms budget:** on the `hold` path the budget starts when the threshold elapses; the wait itself is a user setting, not response time. The median from threshold to first frame is under 150 ms (since S-021, to the first ready `commit` of the catcher, with the visible frame within 150 ms of the drag's enter), and the time from the press is the threshold plus that median. Instant paths (right click, `drop` without `--hold`, `open`) keep the old definition: request to first frame under 150 ms.
- **Closing:** the gate resets whenever the hive closes for any reason (drop, 10 s timeout, Esc, a click), so a lost release never swallows the next press (manager decision 14).
