# Flathub assessment (S-103g)

Question: can Kawaro ship on Flathub without losing what makes it Kawaro?
Answer: not yet. This page rates each limit as works, degraded or blocked,
from the code in this repository. Flathub review policy points are marked
"to confirm", not asserted. Nothing here has run inside a real sandbox; every
rating is a reading of the code and of the Flatpak model.

## Ratings

| Limit | Rating | Facts and what would change |
| --- | --- | --- |
| systemd user daemon | degraded | Flatpak exports no user units, so `kawaro.service` and the package `systemctl --global enable` step have no equivalent. `kawaro open` does start `kawarod` on demand (`crates/cli/src/client.rs`, 2 second wait), so it works, but a cold start sits on the path of the 150 ms hive budget. Autostart would need the Background portal; no code requests it today. The socket is `$XDG_RUNTIME_DIR/kawaro.sock` (`crates/core/src/ipc.rs`); separate `flatpak run` instances share only `$XDG_RUNTIME_DIR/app/<app-id>` (to confirm by test), so the socket must move there when `FLATPAK_ID` is set. |
| file manager integration | blocked | The Nautilus extension (`integrations/linux/nautilus/kawaro.py`), the Dolphin service menu and the Nemo action are read by the host file manager from host paths. A Flatpak exports applications, icons, metainfo and D-Bus services, not these. The right click menu is the guaranteed entry point (CLAUDE.md), so a store install with no menu entry misses the main promise. Nautilus also needs `nautilus-python` on the host. |
| writing next to the original (filesystem access) | degraded | The manifest draft uses `--filesystem=home`, with a comment, since a result goes beside the original wherever it is. Other mounts (external drives) need more grants. Whether Flathub accepts a broad filesystem grant for this purpose is to confirm. The original stays untouched and the atomic rename stays in one directory, so the rule holds. |
| host engines (FFmpeg, LibreOffice, Ghostscript) | blocked | Engines are child processes found on `PATH` (`crates/engines/src/locate.rs`, `office.rs`), so binaries in `/app/bin` are found. Ghostscript: works as a module built from source (licence check needed, `docs/engines.md`). FFmpeg: degraded, because `org.freedesktop.Platform.ffmpeg-full` carries libraries, not the `ffmpeg` and `ffprobe` command line tools, so FFmpeg must be built as a module. LibreOffice: blocked, since building it as a module is impractical and calling the separate LibreOffice Flatpak needs `--talk-name=org.freedesktop.Flatpak` (host command execution), which Flathub review is expected to reject (to confirm). Office actions would disappear. |
| layer-shell | works | The Wayland socket exposes every global the compositor offers, including `zwlr_layer_shell_v1`; the sandbox does not filter protocols (to confirm by test). ADR 0004 already falls back to a window where the global is absent. |
| xdg-activation | degraded | The protocol works inside the sandbox. The token path crosses the sandbox border: the host extension gets the token and must hand it over, preferably as `--token` on `flatpak run ... open`, since forwarding of `XDG_ACTIVATION_TOKEN` through `flatpak run` is to confirm. Focus on real GNOME and KDE is still unproven without a sandbox (S-082). |
| tray icon | works | `ksni` needs `--talk-name=org.kde.StatusNotifierWatcher` and `--own-name=org.kde.StatusNotifierItem-*`, both in the draft. GNOME shows no tray without an extension, the same as outside the sandbox. Desktop notifications through `notify-rust` are to confirm. |
| GlobalShortcuts portal | works | Portals are the sandbox's native route. The daemon calls `Registry.Register` first (`crates/daemon/src/portal_bus.rs`); a sandboxed app gets its id from the sandbox, so the call should be skipped or tolerated when `FLATPAK_ID` is set (to confirm). Alt is not usable through the portal on GNOME and KDE (CLAUDE.md), so `Super+Z` is the shortcut, as today. |

## The manifest draft

`packaging/flatpak/io.github.m7mmad_omar.Kawaro.yml`: Freedesktop 25.08
runtime with the Rust SDK extension, an offline locked cargo build of
`kawaro` and `kawarod`, Wayland only, `--filesystem=home`, the tray names.
`packaging/flatpak/cargo-sources.json` is an empty placeholder; CI generates
the real file with flatpak-cargo-generator from `Cargo.lock`.

What validates today (`tests/release/s103g_flathub.sh`): the YAML parses, the
app id equals the metainfo id, the runtime version is text, the build is
offline and locked, no host escape arguments are present, every filesystem
grant has a comment, and `flatpak-builder --show-manifest` accepts it. What
does not: `flatpak-builder-lint` is not installed here, and no build ran
(it needs the SDK and a full Rust build). CI should run:

```
flatpak-builder-lint manifest packaging/flatpak/io.github.m7mmad_omar.Kawaro.yml
flatpak-builder-lint appstream packaging/io.github.m7mmad_omar.Kawaro.metainfo.xml
```

Expect findings on the filesystem grant and, until a screenshot and a
release URL policy are met, on the metainfo (to confirm).

## What blocks, and the cheapest path

Blocking parts: the file manager entries (no way to ship them from inside the
sandbox), the LibreOffice engine, and two code changes in `crates/` (socket
path and portal registration under Flatpak) that this task does not make.

Cheapest path to a GO: one story that (1) adds `kawaro integrate`, a command
that writes the Nautilus, Dolphin and Nemo entries into the user's data
directory, calling `flatpak run <app-id> open --token ...`; (2) moves the
socket under `$XDG_RUNTIME_DIR/app/<app-id>` and guards the portal
registration when `FLATPAK_ID` is set; (3) builds FFmpeg and Ghostscript as
modules and accepts a Flatpak without Office actions (the hive must hide
actions whose engine is missing; to confirm it already does); (4) asks
Flathub reviewers about the filesystem grant before investing. That is a
reduced product with one manual step, so native RPM, DEB and COPR stay the
recommended Linux install.

Decision: NO-GO for now, because the right click entry cannot ship inside the sandbox and LibreOffice cannot run in it.
