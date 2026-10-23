[English](drop-shortcut.md) | [العربية](drop-shortcut.ar.md)

# Drop shortcut

The drop shortcut shows the hive while you drag a file, so you can drop it on the cell of the action you want. The shortcut runs `sbarhive drop`; the hive it shows takes no keyboard, so the drag stays in your hand. The default shortcut is Super+Z: two keys on the left of the keyboard, unused in GNOME and KDE. A modifier alone (Shift, Ctrl or Alt) does not work on GNOME and KDE, because during a drag it changes the drop kind in the file manager, and Super alone opens the Activities overview or the application launcher, which ends the drag.

If `~/.local/bin` is not on your PATH, use the full path of `sbarhive` as the installer prints it, without `~`.

## GNOME

On GNOME, bind the shortcut by hand in Settings:

1. Open Settings, then Keyboard, then View and Customize Shortcuts, then Custom Shortcuts.
2. Add a new shortcut: name it "SbarHive: drop", with the command:

   ```
   sbarhive drop
   ```

3. Press Super+Z to set it, then choose Add.

On GNOME the hive is a window in the middle of the screen, not at the pointer. That a GNOME custom shortcut fires during a drag is not verified yet: the headless test grabbed the key with `grab_accelerator('<Super>z')` inside GNOME Shell through `Shell.Eval`, not through Settings.

## KDE

On KDE Plasma, bind the shortcut in System Settings:

1. Open System Settings, then Keyboard, then Shortcuts.
2. Choose Add New, then Command or Script, and enter the command:

   ```
   sbarhive drop
   ```

3. Set the shortcut to Super+Z (KDE calls it Meta+Z), then apply.

On KDE the hive is a layer at the dragged file.

## Hyprland

On Hyprland, add a bind line to your config yourself. The lines below are assumed, not verified yet on a real Hyprland session. `--at` reads the pointer from `hyprctl cursorpos` at the press. The hive shows only during a drag of files that have an action: with such a drag it shows at the dragged file, otherwise nothing shows.

Super+Z shows the hive while you drag files that have an action; on Hyprland it appears with the drag's first motion after the press, hyprlang form (`hyprland.conf`):

```
bind = SUPER, Z, exec, ~/.local/bin/sbarhive drop --at "$(hyprctl cursorpos)"
```

Or hold left Alt: the hive shows after a long press (`hold_ms`, 400 ms by default) and closes when you let the key go. The press bind is non consuming (the flag `n`) so combinations such as Alt+Tab keep working, and ignores modifiers (the flag `i`) because Alt is not held yet on its own first press, so a bind that requires Alt matches only from the second press. The release bind ignores modifiers too, and is non consuming as well (the flags `r`, `n` and `i`): Hyprland swallows the press of any key that has a consuming release bind, so without `n` Alt would never reach applications. Being non consuming, Alt still reaches the file manager and may change the drop kind there; not verified:

```
bindni = , ALT_L, exec, ~/.local/bin/sbarhive drop --hold --at "$(hyprctl cursorpos)"
bindrni = , ALT_L, exec, ~/.local/bin/sbarhive drop --release
```

The release bind needs a running daemon, so start it with the session: `exec-once = ~/.local/bin/sbarhived`. The Lua form, and holding Super+Z instead of Alt, are in [the Hyprland guide](../integrations/linux/hyprland/README.md).

## Portal shortcut

On GNOME 48 and later and on KDE 6, the daemon registers Super+Z itself through the desktop's global shortcuts portal, so the manual bind above is not needed. The first time, the desktop opens a dialog asking you to allow the shortcut. On GNOME this dialog may open behind a full screen window; leave full screen to see it.

If you bound Super+Z by hand above, remove that manual bind once the portal has registered the shortcut, so the two do not clash.

To change the key: on GNOME, open Settings, then Apps, choose SbarHive, then Global Shortcuts; on KDE, open System Settings, then Keyboard, then Shortcuts.

The portal shortcut works by holding, not by a tap: hold Super+Z during a drag and the hive shows after the long press (`hold_ms`, 400 ms by default), then closes when you let the key go. A short tap shows nothing.

If you refuse the dialog, the daemon remembers it as `portal_shortcut = false` in `~/.config/sbarhive/config.toml` and does not ask again at the next login. The settings switch "Register the drop shortcut with the desktop" turns registration back on or off. It applies within seconds, with no restart of the daemon. With the switch off, the daemon does not use the portal, and the manual bind above is the way to show the hive.

Where there is no portal (an older GNOME or KDE, or another desktop), the manual bind above stays the way to set the shortcut.

On Hyprland the `exec` bind above with `--at` stays the first way. As an alternative, bind the shortcut the daemon registers with a `global` line instead (hyprlang form, not verified; the verified Lua form is in [the Hyprland guide](../integrations/linux/hyprland/README.md)):

```
bind = SUPER, Z, global, io.github.m7mmad_omar.SbarHive:drop
```
