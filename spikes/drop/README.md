# Drop spike

Throwaway experiment for ADR 0003 (drop a dragged file on a hive cell).
Nothing here is merged into `crates/` as is.

- `src/main.rs`: a Slint 1.18.1 window (winit 0.30, software renderer) that
  receives Wayland drops. winit 0.30 has no Wayland drag and drop, so it opens
  a second wayland-client `Connection` on winit's `wl_display`, binds
  `wl_data_device` on its own queue and thread, filters events by winit's
  `wl_surface`, accepts copy only, and maps the drop point to a cell with the
  real `crates/ui/src/geometry.rs`.
- `source.py`: GTK 4 window that drags one file (copy or move offered, like a
  file manager).
- `kwin.sh`, `gnome.sh`: private headless KWin and GNOME Shell runs. A global
  shortcut is pressed mid-drag, the spike is started mid-drag, and the file is
  dropped on a cell. Input comes from `org_kde_kwin_fake_input` (`src/bin/kwin_input.rs`,
  allowed only inside the private KWin) and Mutter's RemoteDesktop API on the
  private bus (`input_gnome.py`). Results land in `out/`.

```bash
cargo build --release
./kwin.sh [LEVEL]          # focus stealing prevention 0..4, default 1
./gnome.sh [smart|strict]  # Mutter focus-new-windows, default smart
FIRST=1 ./kwin.sh          # hive shown before the drag starts (fails: source is raised over it)
```

Hyprland was not run: it has no safe headless mode here.
