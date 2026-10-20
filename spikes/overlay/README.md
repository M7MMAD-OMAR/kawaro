# Overlay spike

Throwaway experiment for ADR 0004 (the hive as an overlay, not a window, and
the file itself in the centre cell). Nothing here is merged into `crates/` as is.

- `src/platform.rs`: a Slint `Platform` that forwards everything to the winit
  backend (`i-slint-backend-winit =1.18.1`), except the next window marked as
  an overlay, which gets the adapter below. The settings window, GNOME, X11
  and accesskit stay on winit.
- `src/overlay.rs`: a `WindowAdapter` with the software renderer drawing into
  `wl_shm` on its own wayland-client connection (read on a thread, dispatched
  on the Slint thread). Per show: a `zwlr_layer_shell_v1` overlay surface on
  the whole output (one transparent pixel stretched by `wp_viewporter`,
  keyboard exclusive) that catches clicks outside, and the hive as a centred
  subsurface. Fractional scale through `wp_fractional_scale_v1`. Drop mode
  (`SPIKE_DROP=1`) gives the catcher an empty input region and no keyboard.
  `SPIKE_TOKEN` activates it with an xdg-activation token.
- `src/thumb.rs`: centre picture: freedesktop thumbnail cache (MD5 of the
  GLib style URI, `Thumb::URI` and `Thumb::MTime` checked), else the MIME
  type icon (shared-mime-info `globs2` and `generic-icons`, reduced icon
  theme lookup).
- `ui/preview.slint`: the centre hexagon with the picture and a count badge.
- `kwin.sh`: private `kwin_wayland --virtual` runs with the real hive from
  `crates/ui` over a maximised GTK window (`other.py`): Esc with the pointer
  still, with a token, after roaming; click outside; click on a cell; keys;
  a second present; drop mode; previews. `thumbs.py` writes thumbnails with
  gnome-desktop's factory (what Nautilus uses). Input from
  `org_kde_kwin_fake_input`, screenshots through `org.kde.KWin.ScreenShot2`
  (`shot.py`), both allowed only inside the private KWin.
- `gnome.sh`: private `gnome-shell --headless`: no layer-shell, falls back.

```bash
cargo build --release -j 4
./kwin.sh [SCALE [POLICY [FSP]]]   # e.g. ./kwin.sh 1 FocusFollowsMouse 4, ./kwin.sh 1.5
./gnome.sh
```

Results land in `out/`. Hyprland and Sway were not run: no safe headless
Hyprland here and Sway is not installed.
