# ADR 0004: The hive as a screen overlay, not a window, with the file itself in the centre cell

Status: accepted (2026-10-20, option A). The preview fills the centre hexagon at its current size, with no layout change, to be reviewed with S-017.
Date: 2026-10-20

## Context

On first real use on Hyprland 0.56 (focus follows the mouse) the hive came up as an ordinary xdg_toplevel window: Hyprland tiled it with a border, placed it off centre and clipped, and moving the pointer out closed it, because focus followed the pointer and focus loss closes the hive. The owner wants the hive never to behave like a window, but as a polished layer over the screen, and wants the centre cell to show the file itself (a thumbnail or its type icon) instead of the text "1 document".

The spike in `spikes/overlay/` ran only in private headless compositors (KWin 6.7.5 virtual, GNOME Shell 50.5 headless), isolated as in `tests/e2e/run.sh`. It shows the real hive from `crates/ui` over a maximised GTK 4 window that logs the clicks and focus it receives. Sway and Hyprland facts come from source (Hyprland v0.56.2, sway master); KWin source references are tag v6.8.0 unless v6.7.5 is named.

(verified) means a headless run or a line read in source with its location; (assumed) means an untested inference.

## Findings

### 1. Who supports wlr-layer-shell

- **KWin since Plasma 5.20 (verified).** `layershellv1client.cpp` exists in tag v5.20.0 and not in v5.19.5. KWin 6.x advertises version 5 (`src/wayland/layershell_v1.cpp`), seen in the run as `zwlr_layer_shell_v1 v5`, with no restriction to privileged clients (`KWinDisplay::allowInterface` in `src/wayland_server.cpp`).
- **Hyprland 0.56 advertises version 5 (verified in source, `src/managers/ProtocolManager.cpp`)**, as do Sway and other wlroots compositors.
- **GNOME does not (verified).** Absent from headless GNOME Shell 50.5 and from Mutter's protocol list (`src/meson.build`); merge request #1922 was closed in 2021 unmerged. The spike fell back to xdg_toplevel through winit on its own.
- `wp_viewporter`, `wp_fractional_scale_v1`, `wp_single_pixel_buffer_v1` and `xdg_activation_v1` exist in KWin and GNOME (verified by running) and in Hyprland (verified in source); in Sway assumed, not read.

### 2. Design tried: a full screen catcher with the hive as a subsurface

- **Catcher:** a layer-shell surface on the overlay layer, anchored to all four edges, `exclusive_zone = -1`, keyboard `exclusive`. Its buffer is one transparent pixel stretched to the screen by `wp_viewporter`, so its memory cost is negligible (a screen sized buffer would cost about 33 MB at 4K). Any click outside the hive lands on it and closes the hive.
- **Hive:** a `wl_subsurface` of the catcher, at its centre, holding a Slint frame from the software renderer in a `wl_shm` buffer. We compute the position, so the compositor neither places nor clips it.
- **An input region of only the hexagons cannot close on an outside click:** a click outside the region goes to the window below and we get no event. So the catcher takes all input.
- **Drop mode (ADR 0003) is the opposite:** the catcher's input region is empty and keyboard `none`, so the user starts the drag from the window below, and closing follows ADR 0003 (`leave`, timeout, Esc). Changed by S-020: after the first `configure` the catcher's region becomes the hive's rectangle, the subsurface takes no input and the receiver is on the catcher, because Hyprland sends no drag motion to a subsurface of a layer surface.

Run on KWin 6.7.5 (1280 by 800 screen, each case once, under a maximised GTK window):

| Case | Result |
| --- | --- |
| Position and shape | Hive 392 by 361 at (444, 219), exactly centred, no border or tiling (`out/hive-kwin.png`) |
| Click outside the hive | Closed it; the window below got 0 clicks |
| Click on a cell | Picked `image.to-png` |
| Right arrow then Enter | Picked a cell with the pointer over the other window's area |
| Pointer roaming over the other window, then Esc | Focus stayed in the hive and Esc closed it, even with focus follows mouse |
| Drop mode | A click outside the hive reached the window below (one click); the hive did not take the keyboard |
| Fractional scale 1.5 (`kscreen-doctor`) | `preferred_scale` = 1.5 arrived before the first configure; the hive drew at the real screen resolution with no blur (`out/hive-kwin-scale1.5.png`) |
| An ordinary winit window in the same process | Appeared as a normal xdg_toplevel beside the layer, so settings stays a window |

### 3. Focus and closing

- **Hyprland (verified in source only).** An `exclusive` surface takes focus when mapped (`CLayerSurface::onMap`), and focus follows mouse cannot take it away while it is shown: `rawWindowFocus` refuses focus "because of an exclusive layer surface", and `mouseMoveUnified` sets `allowKeyboardRefocus = false`. `on_demand` loses focus when the pointer crosses a window. Hyprland applies no tiling or border to layer surfaces, but plays the `layersIn` animation by default; users can turn it off with a layer rule on the namespace `sbarhive` (the `noanim` effect exists in source; the 0.56 line syntax is unverified, assumed). This addresses the four symptoms the owner saw, not yet run (assumed).
- **Sway (verified in source only).** `seat_set_focus` keeps an exclusive surface from losing focus.
- **KWin treats `exclusive` and `on_demand` alike (verified in source)** and applies focus stealing prevention to layer surfaces. `Workspace::mayActivate` (v6.7.5, `src/activation.cpp`) has an exception that grants focus without a token to a top or overlay surface anchored to all four edges on the active window's output, which is our catcher. The token check at Extreme rejects before reaching it, so the exception works only at lower levels (Medium and High assumed, not run). Results by running:

| KWin policy | Esc with a still pointer | No activation token | Token from a click in a GTK window |
| --- | --- | --- | --- |
| Focus follows mouse, prevention Low (default) | Works | Focus on show | Focus on show |
| Focus follows mouse, prevention Extreme | Fails | Focus about 300 ms after the first pointer motion (the catcher is always under the pointer) | Focus on show, Esc works |
| Click to focus, prevention Extreme | Fails | Focus only on the first click | Focus on show, Esc works |

  The catcher turns focus follows mouse into a source of focus, since the pointer never leaves it, and the xdg-activation token works with it as with a window (the spike injected the token into `xdg_activation_v1.activate`, not through `Hive::present`). On the layer path the token is sent on every show, so no component rebuild is needed as in S-013. Clicks outside and on a cell work without focus under every policy.
- **Focus loss does not close the hive in the new design.** It closes on three things only: Esc, a click on the catcher, or picking a cell. Focus going to another window (the settings window took it in the spike) does not close it.

### 4. Rendering Slint 1.18.1 on our surface

- **(A) A custom platform over winit (tried, verified).** A thin `Platform` forwards everything to `i_slint_backend_winit::Backend` except `create_window_adapter` when the next window is marked as a layer; then it returns our adapter (`SoftwareRenderer`, `slint::Window::new`, drawing into `wl_shm`). A private wayland-client connection is read on a thread and handled on Slint's thread through `invoke_from_event_loop`. Animation is driven by `wl_surface.frame`, because winit redraws only its own windows. Event loop, timers, settings window, GNOME, X11 and accesskit stay on winit.
  - No new Wayland crates: `wayland-client`, `wayland-protocols` and `wayland-protocols-wlr` are already in the tree through winit and smithay-client-toolkit 0.19.2.
  - New: direct dependencies on `i-slint-backend-winit` and `i-slint-core` at `=1.18.1`, the second only for the `InternalToken` and `SlintContextWeak` types in forwarded signatures. Their API is unstable, so every Slint upgrade reviews this file. Slint is already pinned and its upgrade is a separate task.
- **(B) Community crates (rejected).** `layer-shika` 0.3.1 (AGPL-3.0, Slint 1.14, femtovg and EGL) and `spell-framework` 1.0.6 (GPL-3.0, Slint 1.17.1, Skia, implements the internal `WindowAdapterInternal` so it breaks with any other version). Both render on the GPU, three times over the memory budget (ADR 0001, finding 3). `spell-framework` is useful as reading only.
- **(C) A second process (rejected).** Each Slint process costs about 28 MB (ADR 0001), so two do not fit the 40 MB limit, plus IPC between them.

Measurement (headless KWin, development machine): `show` to first frame 6 to 22 ms, process start to first frame 55 to 80 ms; about 107 ms when a winit window opens at the same moment, since window creation holds the same thread. Resident memory 27 MB after two shows with a clean profile (a font heavy profile nearly doubles it, ADR 0001).

**Risks:**

- **Accessibility:** our adapter has no accesskit, so a screen reader cannot read cell names. Use xdg_toplevel when a screen reader is on (refined in the S-019 amendment).
- **Layer choice on every component build:** `present` with a token, and a language or theme change, build a new component and call `create_window_adapter` again. A flag consumed once (as in the spike) silently falls back to xdg_toplevel, so the choice must cover every build.
- **Colour scheme:** `Palette.color-scheme` comes from an internal adapter API, so on the layer path we set `Tokens.dark` from the portal setting (`org.freedesktop.appearance color-scheme`).
- **Cursor:** needs `wp_cursor_shape_v1`, present on KWin and GNOME (verified), unchecked on Hyprland and Sway.
- **Keyboard:** evdev codes for Esc, Enter, Space, Tab and the arrows, independent of layout. No text input, so no xkbcommon or input method.
- **Drop receiver (S-014b)** moves to the layer connection, the only device bound there, so the Hyprland first `wl_data_device` limit (ADR 0003, finding 3) is met.
- **X11 and Windows:** unchanged, on winit.

### 5. The file itself in the centre cell

- **Image source: the thumbnail cache the file manager writes, so no render cost for us (verified).**
  - Path `$XDG_CACHE_HOME/thumbnails/{normal,large,x-large,xx-large}/MD5(uri).png`.
  - A thumbnail is accepted when `Thumb::URI` equals the URI and `Thumb::MTime` equals the file's mtime in seconds; `Thumb::Size` is compared when present.
  - A file in `fail/` means an earlier attempt failed.
  - A thumbnail written by Nautilus's gnome-desktop factory for `صورة 1+&~,(x).png` read as valid and decoded in 0.2 to 0.4 ms (`out/preview-thumbnail.png`); after `touch` it was rejected and the icon shown.
- **The URI must match GLib byte for byte.** Unescaped: letters, digits and ``- . _ ~ ! ' ( ) * + , $ & = : @ /``; everything else is `%XX` in upper case (`g_escape_file_uri` in `glib/gconvert.c`). Our encoding matched `Gio.File.get_uri()` for 6 of 6 names, including Arabic, space, `;`, `#`, `?` and `[]`.
  - KIO (Dolphin) encodes through Qt identically for all of these except `;`, which Qt leaves literal, and symlinks, where KIO encodes the target (verified in source, `src/gui/filepreviewjob.cpp`). So we try both forms when the name has `;`, and the target when the file is a symlink.
- **Sizes:** Nautilus writes `large` at scale 1 and `x-large` at scale 2. Dolphin at the default icon size writes only `normal` (128) up to scale 2. So we look for `large`, then `x-large`, then `xx-large`, then `normal`, and small ones are scaled up.
- **No thumbnail:** show the type icon. Icon themes gave PDF the coloured Adwaita `x-office-document` (`out/preview-icon.png`), which breaks the design tokens and varies by theme. Instead: the MIME type from shared-mime-info `globs2` (verified in the spike), then a monoline icon from our set in `crates/ui/icons` by kind (image, video, audio, PDF, document, spreadsheet, slides, text), coloured by tokens, with the format code (PDF). System theme icons are not used.
- **Several files:** two offset hexagons behind, and the count in a badge. Show the first file's thumbnail if any, else the shared type icon, else a generic icon.
- **Crates:** `md-5` (MIT or Apache-2.0) and `png` (MIT or Apache-2.0, already in dev-dependencies). We decode PNG ourselves because Slint is built without `image-default-formats`. Reading globs2 takes twenty lines, so no `xdg-mime` (Apache-2.0) or `freedesktop-icons` (MIT). Both crates are recorded in `docs/engines.md`.
- **Display:** an image inside the square inscribed in the hexagon, with `image-fit: contain`. Slint does not clip images to a hexagon, so the inscribed square is the substitute.
- **Size:** the centre cell is about 72 px wide, so its inscribed square is about 50 px (the spike drew at a 96 px radius). Growing it for a preview is the owner's call; for now it stays the current size (see Status).

## Decision

- **The corrected CLAUDE.md rule** (replacing "do not rely on wlr-layer-shell"): on Wayland the hive is a `zwlr_layer_shell_v1` surface on the overlay layer when the compositor offers it (Hyprland, KDE Plasma since 5.20, Sway and other wlroots). On GNOME (Mutter lacks it), X11 and Windows it stays an xdg_toplevel window or its equivalent through winit. No feature is built on layer-shell without a fallback that works on GNOME. With a screen reader running, the window is used everywhere.
- The thin platform over winit with the layer adapter (option A), the catcher and subsurface design, and drop mode with a catcher that takes no input.
- Close on Esc, a click on the catcher or a pick, not on focus loss. The activation token from IPC is sent to `xdg_activation_v1.activate` on the catcher on every show, and the layer choice covers every hive component build.
- Centre cell: the thumbnail from the shared cache, then our icon by type, then a generic icon; a count for several files.

## Consequences

Delivered in S-018: the layer platform and adapter, the centre cell preview, and KWin checks in `tests/overlay/`.

Still open:

- [ ] Hyprland by running: position, no border, focus with follows mouse, outside clicks, the `layersIn` animation.
- [ ] Sway by running.
- [x] Drop on a layer subsurface, on KWin and Hyprland: works on KWin; Hyprland sends it no `motion` (`tests/hyprland/repro.sh`), so the drop moved to the catcher with the hive's region (S-020), which works on both.
- [ ] KWin at Medium and High prevention with no token: does the four edges exception grant focus on show as the source says?
- [ ] The token through `Hive::present` in `crates/ui`, not by injection, with real Nautilus and Dolphin.
- [ ] Several screens: which screen the hive appears on (the compositor picks when we pass `output = None`).
- [ ] A low end machine measurement, and the first show after the service starts with a real accessibility bus (ADR 0001).

## Amendments

### S-019 (2026-10-21, manager decision delegated by Mohammad)

- **Screen reader rule:** the `org.a11y.Status.ScreenReaderEnabled` flag alone is not enough; it often stays true from a GNOME setting with no reader running, which forced the hive into a window for Mohammad on Hyprland. The hive is a window for a screen reader only when the flag is set and the name `org.gnome.Orca.Service` has an owner on the session bus (`NameHasOwner`, then `NameOwnerChanged`). The `hive_surface` key in `config.toml` (`auto`, `layer` or `window`; missing means `auto`) overrides the rule, and `SBARHIVE_SURFACE` overrides everything, as before. Every change of the decision prints one line `hive surface choice: KIND, REASON`, with no environment variable needed. Implemented in S-019a.
- **Anchor:** on the layer path the hive appears at the pointer or the drag position, not the centre. The anchor is clamped so the centre cell and its margin stay on screen, and the hive does not follow the pointer after its first position (after the first frame on a normal show; after `ANCHOR_WAIT` on a drop show, since S-021). The pure computation (`layout::plan_at`) is in S-019b, wiring into the show in S-019c and S-019d.
- **Adaptive shape:** near the edges only slots inside the screen are used, ring by ring up to `MAX_RINGS` = 4, with no margin beyond `PAD`. First ring cells never displace second ring cells, and in a corner fewer than 18 cells may show.
- **Catcher phase in a drop:** the catcher takes input briefly until the first drag position, then returns to the hive's region as in S-020. With no position within 50 ms the hive appears centred and moves once if the first position arrives within 300 ms, counted from the position (the end of `ANCHOR_WAIT`; it was from the first frame before S-021, but a drop show on the layer path is not drawn before the drag enters). After that (decision 12) the catcher's region becomes the centred hive's rectangle with no move, so the catcher never holds input for more than 300 ms (no freeze, S-020 AC4). A still hand is placed by `--at` alone. `sbarhive drop --at X,Y` passes the position where available (Hyprland through `hyprctl cursorpos`). Implemented in S-019d.
- **The window path is unchanged:** on GNOME and with a running screen reader the hive stays centred.
