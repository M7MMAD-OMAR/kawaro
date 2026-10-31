# Launch asset manifest (S-072)

Every file the launch kit uses, listed in place. Nothing is copied or edited, so the hash cannot drift from the source. Only real assets appear: brand files drawn by `design/brand/build.py`, screenshots that a test script rendered from the app, and the site's own Open Graph image and hero films. No stock images, no hand drawn mock-ups.

## Assets

| file | source | produced by | language | theme | sha256 |
|---|---|---|---|---|---|
| `design/brand/kawaro-cells.svg` | `design/brand/kawaro-cells.svg` | `design/brand/build.py` | none | light | `a9b622c8e202e3ce84af95f48c153d4b30f2fc21e8564921107520b0cc1892a2` |
| `design/brand/kawaro-cells-dark.svg` | `design/brand/kawaro-cells-dark.svg` | `design/brand/build.py` | none | dark | `cb632c40e5a7bdcd9afa81b5f11c1586b9fca1a9c95fba2d4edf76a074335a9a` |
| `design/brand/wordmark.svg` | `design/brand/wordmark.svg` | `design/brand/build.py` | none | light | `9218b54010d6b8aa1c8ef3316d0b3016dd119f8701d6da20b61290cb1d410f6b` |
| `design/brand/wordmark-dark.svg` | `design/brand/wordmark-dark.svg` | `design/brand/build.py` | none | dark | `4e2ead9cc119c06884108085c6563676a7bf3b85332447a81bc5297b04279509` |
| `design/brand/png/kawaro-512.png` | `design/brand/png/kawaro-512.png` | `design/brand/build.py` | none | none | `922cf5d53921e9fd813374d023aaa2432fd4aa9712b98eac955f2e91b02ae009` |
| `design/brand/png/kawaro-128.png` | `design/brand/png/kawaro-128.png` | `design/brand/build.py` | none | none | `26af98f8f7bcb218dff813d7e7905f42032643912f09593d3241c25599646bdf` |
| `docs/screenshots/s094/A-general-en-light-default.png` | `docs/screenshots/s094/A-general-en-light-default.png` | `crates/ui/tests/settings_snapshots.sh`, S-094 | en | light | `0564ab8e202c6c5fda39f26d008e3bb2e7515bc959e3c729e18ce5ccf2a4e4b6` |
| `docs/screenshots/s094/A-general-en-dark-default.png` | `docs/screenshots/s094/A-general-en-dark-default.png` | `crates/ui/tests/settings_snapshots.sh`, S-094 | en | dark | `512ffe5f50f0b9b3450f9d2ebe7e13b673aaeeaad4a39038ec335b70f40807da` |
| `docs/screenshots/s094/A-general-ar-light-default.png` | `docs/screenshots/s094/A-general-ar-light-default.png` | `crates/ui/tests/settings_snapshots.sh`, S-094 | ar | light | `62badc3756376721edb0e095edcf394e0a1bb94cb00171728b3ee37d803ae238` |
| `docs/screenshots/s094/A-general-ar-dark-default.png` | `docs/screenshots/s094/A-general-ar-dark-default.png` | `crates/ui/tests/settings_snapshots.sh`, S-094 | ar | dark | `82ff3e254993d624cb8744ef7afc7e9fcfb943ed9221ab3bae6e0bca26898c7f` |
| `site/og.png` | `site/og.png` | committed site asset, no generator named | none | none | `12fadfb92c16c11d69c2b92ec546cbddb34b01b83290b9156029932c0fd64f46` |
| `site/media/hero-en.mp4` | `site/media/hero-en.mp4` | committed site asset (hero film, English) | en | none | `12a6cd8016ab37cc13f7ec4651a6eeb90c9b9e4ceb9761fc8208ac41b9090270` |
| `site/media/hero.mp4` | `site/media/hero.mp4` | committed site asset (hero film, Arabic) | ar | none | `be8344462b7e9eb5c03a5817ca68362ce6ac67d7f0a1981b17782cbcb3a9d841` |

Notes:

- The logo is the saguaro cactus built from hive cells (`kawaro-cells*.svg`, `wordmark*.svg`); the single cell with the cactus cut out is the app icon (`png/kawaro-*.png`). See `design/brand/README.md` and `docs/adr/0008-name-kawaro.md`.
- The four screenshots are the Settings window, General page, at the default 880 x 640 size. The Arabic ones show the right to left layout (rail on the right, text right aligned). They were recaptured on 2026-10-31 by `crates/ui/tests/settings_snapshots.sh` (private headless KWin), so the header shows the version label 0.5.0, the workspace version at capture time. They are unedited; a later version needs a new capture by the same script, not an edit.
- A by eye check of every listed screenshot found no username, home path or machine name (2026-10-31).

## Claims

Every sentence of the announcement and the hero that states a platform, a format or a number maps to a row here. Platform wording is limited to what `README.md` (Platform support) says.

| claim | evidence |
|---|---|
| Kawaro 0.5.0 is the current release | `Cargo.toml`, `docs/release-notes-0.5.0.md` |
| X11 support: the hive opens as an ordinary window at the pointer, kept inside the screen | `docs/release-notes-0.5.0.md` |
| On X11, `kawaro drop` opens the hive above other windows so a dragged file can be dropped on a cell | `docs/release-notes-0.5.0.md`, `docs/drop-shortcut.md` |
| X11 was checked only in a private virtual X session (Xvfb with xfwm4), not on a real GNOME or KDE X11 desktop | `docs/release-notes-0.5.0.md`, `README.md` |
| Nothing changes on Wayland or Windows, nor in how files are converted | `docs/release-notes-0.5.0.md` |
| Windows: installed and run in a virtual machine (Windows 11), not on physical hardware | `README.md` |
| The Windows build is signed with a self-signed certificate, so SmartScreen warns on first run | `README.md`, `docs/windows-smartscreen.md` |
| macOS: not supported yet | `README.md` |
| WinGet: waiting for review | `README.md`, `docs/distribution-status.md` |
| apt and Scoop are live | `README.md`, `docs/distribution-status.md` |
| Linux on Wayland (Hyprland, KDE Plasma, GNOME) tested | `README.md` |
| No uploads, no servers, no telemetry; the original file is never modified | `CLAUDE.md`, `docs/release-notes-0.5.0.md` |
| Free software under GPLv3 (GPL-3.0-or-later) | `LICENSE`, `README.md` |
| The hive appearing within 150 ms is a budget, not a measured result | `CLAUDE.md` |
| The project was called SbarHive before the rename to Kawaro; the cactus mark stayed the same | `docs/adr/0008-name-kawaro.md`, `design/brand/README.md` |
| Kawaro comes from kawwara (an old Arabic word for a beehive) | `docs/adr/0008-name-kawaro.md` |
| Hero: select a file, right click, convert locally, result saved next to the original | `README.md`, `site/en/index.html` |

Evidence for the old name: the ADR calls it "the first name" without spelling it out; the spelling is in git history (`README.md` at commit 88a269e^, the parent of the rename commit).
