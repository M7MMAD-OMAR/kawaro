[English](README.md) | [العربية](README.ar.md)

# Kawaro on Hyprland

`scripts/install.sh` installs `~/.local/bin/kawaro-open-selection` when it finds `hyprctl`. It never edits `hyprland.conf`; add a bind yourself.

The helper sends Ctrl+C to the active window, so the file manager copies the selected files, then runs `kawaro open --from-clipboard`, which reads them from the clipboard with `wl-paste`.

Suggested keyboard bind (safe, no clash with the defaults):

```
bind = SUPER SHIFT, H, exec, ~/.local/bin/kawaro-open-selection
```

Mouse bind, Super plus right button:

```
bind = SUPER, mouse:273, exec, ~/.local/bin/kawaro-open-selection
```

Careful: the default Hyprland config binds `SUPER + mouse:273` to window resizing (`bindm = SUPER, mouse:273, resizewindow`). To use the mouse bind, remove that line or pick another button such as `mouse:274` (middle).

Notes:
- Use it in file managers only. In a terminal Ctrl+C interrupts the running program.
- Needs `wl-paste` from `wl-clipboard`.
- If your file manager is slow to copy, raise the delay: `KAWARO_COPY_DELAY=0.3`.
- Hyprland without uwsm starts neither `graphical-session.target` nor autostart files. The daemon starts on the first `kawaro open`, or add `exec-once = ~/.local/bin/kawarod`.
- With a different PREFIX, replace `~/.local/bin`. The installer prints the line with the right path.

## Turning off the hive's animation

On Hyprland the hive is a layer-shell surface with the namespace `kawaro`, so Hyprland animates it like any layer. A layer rule turns that off. The rule is not verified yet on a real Hyprland session.

Lua form (Hyprland 0.55 and later, `hyprland.lua`):

```
hl.layer_rule({ match = { namespace = "^kawaro$" }, no_anim = true })
```

hyprlang form (Hyprland 0.53 and 0.54, `hyprland.conf`), not verified:

```
layerrule = no_anim on, match:namespace ^kawaro$
```

## Hold a key to drop

The drop hive shows only during a drag of files that have at least one action. Start the drag first, then hold the key: after a long press (400 ms by default, `hold_ms` in `config.toml`) the hive shows under the dragged file, stays while you hold the key and closes when you let it go. Without a drag, or for a file with no action, nothing shows. A short tap shows nothing either.

Once the threshold ends, the daemon waits for the compositor's first event to learn whether a drag is under way, and Hyprland sends it only with the first move, so with a still hand the hive shows as soon as you move the file a little. If you start the drag only after the threshold has ended, the hive does not show; let go of the key and try again in the right order.

That takes two binds: the press runs `kawaro drop --hold` and the release runs `kawaro drop --release`. The timer is in the daemon, not in the bind, so both commands return at once.

The release bind never starts the daemon, and a daemon started by the press may miss a quick release, so start it with the session: `exec-once = ~/.local/bin/kawarod`.

hyprlang form (`hyprland.conf`). These lines are assumed, not verified yet on a real Hyprland session:

```
bindn = SUPER, Z, exec, ~/.local/bin/kawaro drop --hold --at "$(hyprctl cursorpos)"
bindrni = SUPER, Z, exec, ~/.local/bin/kawaro drop --release
```

Lua form (`hyprland.lua`), assumed as well:

```
hl.bind("SUPER + Z", hl.dsp.exec_cmd("$HOME/.local/bin/kawaro drop --hold --at \"$(hyprctl cursorpos)\""),
    { non_consuming = true, description = "Kawaro: hold to drop" })
hl.bind("SUPER + Z", hl.dsp.exec_cmd("$HOME/.local/bin/kawaro drop --release"),
    { release = true, non_consuming = true, ignore_mods = true, description = "Kawaro: release to close" })
```

To hold left Alt instead of Super+Z (assumed as well), hyprlang form:

```
bindni = , ALT_L, exec, ~/.local/bin/kawaro drop --hold --at "$(hyprctl cursorpos)"
bindrni = , ALT_L, exec, ~/.local/bin/kawaro drop --release
```

Lua form:

```
hl.bind("ALT_L", hl.dsp.exec_cmd("$HOME/.local/bin/kawaro drop --hold --at \"$(hyprctl cursorpos)\""),
    { non_consuming = true, ignore_mods = true, description = "Kawaro: hold Alt to drop" })
hl.bind("ALT_L", hl.dsp.exec_cmd("$HOME/.local/bin/kawaro drop --release"),
    { release = true, non_consuming = true, ignore_mods = true, description = "Kawaro: release Alt to close" })
```

### Alt with another key (Alt+a, Alt+Tab)

Hyprland does not fire a release bind when another key came between the press and the release, so after Alt+a the daemon would wait for a release that never comes and show the hive 400 ms later with no Alt held. In the Lua config, add a key hook next to the two Alt binds. It only watches keys and consumes nothing; by design every combination still reaches its application. This was measured for Alt+a only; Alt+Tab and Alt+F4 were not measured in the test setup (the outer private compositor takes Alt+Tab). While Alt is down, any other key press runs `kawaro drop --other-key`, which ends the pending press (the daemon logs `hold: chord`) and shows nothing. The short `sleep` keeps it behind the press bind, which runs `hyprctl` first; the daemon also remembers an other-key for 150 ms, so a key that still wins the race cancels the press that follows it. `64` is `Alt_L` (evdev 56 plus 8); measured on Hyprland 0.57.0 in the private session of `tests/hyprland/repro.sh --altdrag` (Alt+a: 0 hive surfaces and `hold: chord`; before: 5 surfaces):

```
local alt_down = false
hl.on("input.keyboard.key", function(code, _, state)
  if code == 64 then -- Alt_L, the evdev code 56 plus 8
    alt_down = state == 1
  elseif alt_down and state == 1 then
    hl.exec_cmd("sleep 0.05; $HOME/.local/bin/kawaro drop --other-key")
  end
end)
```

hyprlang has no equivalent: its `catchall` key works in submaps only, and a submap would switch off your other binds while Alt is down. With hyprlang a chord is handled by the time limit (`hold_max_ms`) instead. `kawaro drop --other-key` never starts the daemon and ends a press only before the hive has shown; a shown hive stays until the release.

Every release bind above is non consuming (the flag `n` with `r` and `i` in hyprlang, `non_consuming` in Lua), and it must stay so. Hyprland swallows the press of a key whose release bind is consuming, and since the release bind ignores modifiers it matches the bare key: a consuming `bindri = SUPER, Z` swallows every Z typed in any application, and one on `ALT_L` every Alt. This is verified, in the Hyprland 0.56.2 source (`KeybindManager.cpp` suppresses the press for a `release` bind unless it is non consuming) and on a real session where Z stopped typing. A release with no hive shown does nothing, so the extra call on a plain Z costs nothing.

Notes, none of them verified:
- The release bind does not fire when the modifier is let go before the main key (Super before Z, say), so it needs to ignore modifiers: the flag `i` with `r` in hyprlang, `ignore_mods` in Lua.
- A press bind on Alt alone, or any modifier, must be non consuming (the flag `n` in hyprlang, `non_consuming` in Lua), so combinations such as Alt+Tab keep working. It must also ignore modifiers (the flag `i` in hyprlang, `ignore_mods` in Lua) and not require the modifier itself: on the first press Alt is not held yet, so a bind such as `ALT, ALT_L` matches only from the second press.
- `--at` is read at the first press, so the hive shows where the pointer was when the key went down.
- `kawaro drop` without `--hold` has no threshold, for a plain key press bind, and follows the same rule: the hive shows only with a drag of files that have an action.

## Portal shortcut

The `exec` binds above stay the first way. As an alternative, xdg-desktop-portal-hyprland passes on the Super+Z shortcut the daemon registers, with one `global` line instead of the press and release binds. It works by holding too (`hold_ms`). hyprlang form (`hyprland.conf`), not verified:

```
bind = SUPER, Z, global, io.github.m7mmad_omar.Kawaro:drop
```

Lua form (`hyprland.lua`):

```
hl.bind("SUPER + Z", hl.dsp.global("io.github.m7mmad_omar.Kawaro:drop"))
```

Keep only one of the two ways for Super+Z.
