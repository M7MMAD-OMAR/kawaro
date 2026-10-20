# Focus tests (S-013)

Do GNOME (Mutter) and KDE (KWin) give the hive keyboard focus when the
background daemon opens it, with and without an xdg-activation token?

Everything runs in a private headless compositor with a throwaway `HOME`,
XDG directories, `XDG_RUNTIME_DIR` and session bus, set before
`dbus-run-session`. `DISPLAY`, `HYPRLAND_INSTANCE_SIGNATURE` and
`XDG_CURRENT_DESKTOP` are unset. Nothing touches the live desktop, the real
profile or its dconf. Screenshots are of the private compositor only.

## Run

```
cargo build --release
tests/focus/gnome.sh notoken token bogus            # Mutter, focus-new-windows=smart (default)
tests/focus/gnome.sh strict notoken token bogus     # Mutter, focus-new-windows=strict
tests/focus/kwin.sh notoken token bogus level=1     # KWin, focus stealing prevention Low (default)
tests/focus/kwin.sh notoken token bogus level=4     # KWin, Extreme
```

Each case starts its own compositor and takes 30 to 40 s. `NO_WARMUP=1` skips
the warm-up show, so the measured show is the first one after the daemon
starts. `FOCUS_OUT=dir` changes the output directory (default
`tests/focus/out/`, ignored by git). A different binary directory can be
passed as the last argument.

## What a case does

1. Starts the compositor, then `sbarhived` with `SBARHIVE_DEBUG_FOCUS=1`
   (prints `hive focus: true|false` on the hive window's focus events) and
   `WAYLAND_DEBUG=1` (protocol trace, to see whether
   `xdg_activation_v1.activate(token, surface)` is sent).
2. Warm-up: `sbarhive open` once and waits 10 s (the first show can be slow,
   ADR 0001 finding 8).
3. Starts `client.py`, a GTK 4 window that stands in for the file manager.
   It launches `sbarhive open` the way the Nautilus extension does:
   `token` gets a real token from `Gdk.Display.get_app_launch_context()
   .get_startup_notify_id()`, `notoken` passes none, `bogus` passes a made-up
   token (control: a policy that honours tokens must reject it).
4. Records the compositor's focused window, windows that demand attention and
   shell notifications before and after, plus the daemon's focus lines.

## Compositor specifics

- GNOME: `gnome-shell --headless --wayland --virtual-monitor 1280x800
  --unsafe-mode`. Unsafe mode only opens `Shell.Eval` and `Shell.Screenshot`
  to the test on the private bus. The shell starts in the overview, where no
  window has focus and keys go to the search entry, so the script hides it
  first. The keyboard comes from Mutter's RemoteDesktop D-Bus API
  (`input_gnome.py`: `CreateSession`, `Start`, `NotifyKeyboardKeysym`), which
  adds a virtual keyboard to the seat. The client is focused through
  `Meta.Window.activate` (a click would do the same), then typed into, so the
  token carries a real input serial. The case keys are F5 (token), F6 (none)
  and F7 (bogus). A screenshot lands in `out/gnome-*.png`.
- KWin: `kwin_wayland --virtual`, with `FocusStealingPreventionLevel` written
  to the throwaway `kwinrc`. No input is injected (ydotool works through
  `/dev/uinput`, which would reach the live session, so it is never used);
  `client.py` launches by itself 3 s after start (`CLIENT_AUTO`). Since S-018c the warm-up hive is closed with one Esc through KWin's
  own `org_kde_kwin_fake_input` (`tests/drop/kwin-input`, allowed by
  `KWIN_WAYLAND_NO_PERMISSION_CHECKS=1` in this private KWin only), because on
  the layer path focus loss no longer closes it. KWin issues
  tokens to the active window without an input serial. State is read with a
  KWin script loaded over D-Bus that prints to KWin's log.

## Nautilus

`tests/focus/gnome.sh nautilus` installs the real extension into the
throwaway `XDG_DATA_HOME`, opens Nautilus on the work folder and drives it
with the keyboard: select all, Shift+F10, then `NAUTILUS_MENU_KEYS` (default
`End Up`, the entry above Properties) and Return. Other nautilus-python
extensions installed on the system can open windows of their own (Nextcloud's
setup wizard did); the script closes them first.

Status (2026-10-20): the context menu opens with the SbarHive entry in it
(`out/gnome-smart-nautilus-menu.png`), but in two attempts the key sequence
did not launch it, so this route is unverified. The `client.py` route uses the
same token call as the extension.
