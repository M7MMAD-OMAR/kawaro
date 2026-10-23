# Drop tests (S-014)

A file dragged from a file manager, `sbarhive drop` run while the drag is
still going (what the keyboard shortcut does), and the file dropped on a hive
cell. ADR 0003 and the spike in `spikes/drop/` are the background.

Everything runs in a private headless compositor with a throwaway `HOME`, XDG
directories, `XDG_RUNTIME_DIR` and session bus, set before `dbus-run-session`,
as in `tests/focus/`. `DISPLAY`, `HYPRLAND_INSTANCE_SIGNATURE` and
`XDG_CURRENT_DESKTOP` are unset. Nothing touches the live desktop.

## Run

```
cargo build --release
cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml   # KWin only
tests/drop/kwin.sh             # KWin, focus stealing prevention Low (level=1)
tests/drop/gnome.sh            # Mutter, focus-new-windows=smart
tests/drop/kwin.sh shortcut    # S-014d AC11: Super+Z mid-drag, Esc
tests/drop/gnome.sh shortcut
tests/drop/budget.sh           # S-014d AC12, S-021 AC8: drop-mode catcher ready, 20 shows
tests/drop/docs.sh             # S-014d AC12: binding docs, no compositor
tests/drop/budget.sh 20 1 hold # S-019e AC20, S-021 AC8: hold-mode catcher ready from the threshold
tests/drop/hold_docs.sh        # S-019 AC16, AC22, AC23, S-021 AC11, AC12: docs, no compositor
tests/drop/window_path.sh      # S-021 AC10: gnome.sh smart and overlay/gnome.sh unchanged
tests/drop/gnome.sh smart portal       # S-014e2 AC18 c: fake portal, window path
tests/drop/gnome.sh smart portal-real  # S-014e2 AC20, AC21: the real GNOME portal
tests/drop/kwin.sh level=1 portal-real # S-014e2 AC20, AC21: the real KDE portal
tests/hyprland/repro.sh --portal       # S-014e3 AC20: the real Hyprland portal
tests/drop/portal_docs.sh      # S-014e3 AC22: portal docs, CHANGELOG, no compositor
tests/drop/gnome.sh strict portal-real # S-014f3 AC15 b, c: token, strict focus
tests/drop/kwin.sh level=1 portal-real rebind # S-014f4 AC16: Meta+Y in KDE
tests/drop/gnome.sh smart portal-real rebind  # S-014f4 AC16: Super+Y in GNOME
```

Each prints `PASS`/`FAIL` lines, ends with `RESULT PASS` or `RESULT FAIL (n)`,
and exits non-zero on any failure. Logs land in `tests/drop/out/` (ignored by
git; `DROP_OUT=dir` changes it).

## What a run does (S-014b: AC5, AC6, AC7)

The daemon starts with `WAYLAND_DEBUG=1` and `SBARHIVE_LANG=en`, with no
warm-up show. `source.py` (GTK 4, offers copy and move like Dolphin and
Nautilus) drags a copy of `tests/fixtures/images/alpha.png` in four shows:

1. drop on the WEBP cell: `photo one.webp` within 5 s, `file --mime-type` is
   `image/webp`;
2. over the WEBP cell, then drop on a transparent corner: the hive closes
   within 2 s of the release (not by the 10 s enter timeout), no new file,
   the source sees the drop refused;
3. drop on the WEBP cell with Shift held: the source reports `deleted=False`
   and the original's sha256 is unchanged (`source.py` deletes the original
   when a move is taken, as a file manager would);
4. drop on the centre cell: as 2.

Then, from the daemon's protocol trace: each show gets an `enter` on the
receiver's data device and requests a
`wl_data_device` after the new hive surface's `get_xdg_surface` (Slint
destroys the surface on hide), `set_actions` offers copy only, and `accept`
names `text/uri-list` only.

On KWin the hive is a layer surface (S-018): `kwin.sh` reads the hive's
rectangle from the daemon's last `hive surface: layer X Y W H` line after the
show's `get_layer_surface` (the daemon runs with `SBARHIVE_DEBUG_FOCUS=1`),
counts it shown while there are more `get_layer_surface` than
`zwlr_layer_surface_v1.destroy` requests, and takes the show's new surface from
`get_layer_surface`. In each show the drag stands still for at least 400 ms
after the catcher is mapped, then moves to the hive's centre: with no position
within 300 ms of the catcher's ready commit the catcher takes only the centred
hive rectangle and the hive never moves (S-019, manager decision 12), and the
drag enters it there. The ready commit is the catcher's first `commit` after the
first `configure` of its layer surface (`catcher_ready` in `layer.sh`, S-021
AC4): a drop show on the layer path draws no hive until the drag enters, so
there is no first frame to count from. It then runs the checks in `layer.sh`
(S-018d AC13, revised by S-020 AC1 and AC2 and by S-019 AC8 and AC9) for each
show: keyboard interactivity 0; the catcher's default input region until the
first position (the first pointer or drag enter on it), then a region of exactly
the hive rectangle; with `sbarhive drop --at X,Y` that rectangle before the
catcher's first commit; with no position (the drag standing still, no `--at`,
decision 12) that rectangle as the catcher's first region, with no enter on it
before, at most 400 ms after the catcher's ready commit by the trace timestamps,
and as its last region; after a first position, the switch at most 400 ms after
the catcher's ready commit; no typed `accept`, no `receive` and no `drop cells:`
line before the switch, and one `drop cells:` line per show; an empty input
region on the hive subsurface; one `get_data_device` after `get_layer_surface`
with every earlier data device released first; and the drag's first `enter` on
this show's data device naming the catcher, at a point inside the hive
rectangle. GNOME stays on the window path (`get_xdg_surface`).
`tests/overlay/kwin.sh drop` runs the same `layer.sh` checks (S-021: KWin sends
`wl_pointer.enter` to a catcher bound under a still pointer, which hides a drop
show with no drag): its first and third shows are a held PNG drag; its second
has no drag and a press with no motion, and must print `hive probe: hidden
(pointer)` with one destroy within 500 ms of that enter and nothing drawn; its
fourth is `sbarhive drop --at 640,400` with the pointer at (100, 700), where a
click must reach the window below once, with no pointer enter on the catcher
outside the hive and no button, while the catcher stays until its 10 s bound
(`hidden (cap)`) and draws nothing.

S-019d adds six shows to `kwin.sh` with the drag held at P (S-019 AC10 and
the manager's decision 1): a 1 px drag motion around (400, 500) from before
`sbarhive drop` (three shows, each anchored there and dropped on the WEBP
cell, the hive at (204, 319)); the same around (100, 700); a drag standing
still for 1 s and then moving to (400, 500), outside the centred hive
(decision 12: `hive anchor: none` only, one `set_position` in the middle,
and the drop on that hive's WEBP cell writes the file; a still hand is placed
by `--at` only); and `sbarhive drop --at 400,500` with the drag
still (anchored at once, one `set_position`, the file written).

The WEBP cell is located from the shipped layout rule
(`crates/ui/src/layout.rs`, `geometry.rs`, `design/tokens.json`): first cell of
ring 1, top right of the centre in the English layout.

## S-014c: waiting for the drop (AC8, AC9, AC10)

Both scripts add a fifth show: `tests/fixtures/docs/one-page.pdf` (a one page
PDF written by hand) dragged in and dropped on a corner. Both run the daemon
with `SBARHIVE_DEBUG_FOCUS=1`.

- AC8: the daemon prints one stderr line per enter of a drag into a drop-mode
  hive, once the cells for the entered files are known and before any drop:
  `drop cells: ID ID ...` (cell ids in cell order, one space apart). In every
  show each such line must equal `sbarhive actions --json FILE` for that
  show's file, run in the same sandbox, and there must be at least one. The
  PNG and PDF lists must be non-empty and differ.
- AC9, AC10 (KWin only; the window path draws the same component, S-018 AC7):
  the daemon runs with `SBARHIVE_COLOR_SCHEME=light` and
  `SBARHIVE_REDUCED_MOTION=1`. In show 1, a ScreenShot2 shot is taken before
  the drag enters and one while the drag is over the WEBP cell
  (`tests/overlay/shot.py`, started in the background because it writes its
  PNG only after 5 s). Since S-021 the layer path draws nothing before the
  enter: the shot before it must have no `text-primary`, `surface-2` or
  `surface-3` pixel in the hive's box (guarded by a shot of the source window
  alone). Over the WEBP cell `pixels.py` checks the cell is
  `color.light.accent` and the ring 1 cell across the centre is `surface-2`
  with no accent; that box is written to `out/drop-en-light-over.png` and
  compared (0.1 % of pixels or less, 8 levels) with
  `snapshots/drop-en-light-over.png`. The waiting text (AC10) is checked on
  the window path (S-021 question 4): after the long press cycles `kwin.sh`
  restarts the daemon with `SBARHIVE_SURFACE=window`, holds a PNG drag still
  under the waiting hive, checks text-primary ink in the centre, no cell and
  no S-021 line, and compares the crop of the `hive surface: window`
  rectangle with `snapshots/drop-en-light-waiting.png`. A missing reference
  is a FAIL: the maintainer approves a candidate from `out/` and it is copied to
  `snapshots/`.
- `source.py` uses a one pixel transparent drag icon, so the shots show the
  hive only.

## S-014d: the shortcut mid-drag (AC11, AC12)

`kwin.sh shortcut` and `gnome.sh shortcut` never run `sbarhive drop`
themselves. Super+Z is bound inside the private compositor to a wrapper that
writes one time line and execs `sbarhive drop`:

- KWin: a KDE custom command shortcut, as the docs describe it: a `.desktop`
  entry with `X-KDE-GlobalAccel-CommandShortcut=true` in the sandbox
  `applications` folder and `[services][net.local.sbarhive-drop.desktop]
  _launch=Meta+Z` in `kglobalshortcutsrc`, launched by the kglobalacceld in
  KWin. Keys go through `org_kde_kwin_fake_input` (evdev 125 + 44, Esc 1).
- GNOME: `global.display.grab_accelerator('<Super>z')` in the normal action
  mode through `Shell.Eval`, which spawns the wrapper (ADR 0003, result 4).
  gsd-media-keys custom shortcuts are out of scope (S-014). The overview must
  stay closed after Super+Z.

Three drags (one session) each press Super+Z mid-drag and drop a PNG on the
WEBP cell: the binding ran once, the hive appeared, `<name>.webp` is written,
`deleted=False`. Then Esc mid-drag twice, no file written in either:

- over the WEBP cell after `enter`: the source sees `drag-cancel`, and the
  daemon's trace shows the hive surface destroyed within 500 ms of the Esc
  (the trace stamps are UTC wall time, compared with `date -u` at the Esc);
- before the drag enters the hive: the compositor cancels the drag at the
  source and nothing reaches the hive (on the KWin layer path the hive takes
  no keyboard in drop mode, S-018 AC13; on GNOME Mutter's drag grab takes
  Esc), so it closes by its 10 s timeout (checked within 12 s); on the layer
  path that hidden catcher prints `hive probe: hidden (cap)` (S-021).

On KWin no `wl_keyboard` enter or key may reach the daemon in those shows.

`budget.sh` runs `crates/ui/tests/measure.sh 20 1` with `DROP=1` (`hive_bench
show --drop`): every show line must say `mode=drop`, every show is a layer
surface that takes no keyboard (no `hive focus: true`), and the median warm time
is under 150 ms. Since S-021 AC8 that time is `catcher_ms`, from the request to
the catcher's ready commit (no drag enters the bench's show, so the hive is
never drawn and no `hive reveal:` line may appear). `docs.sh` checks the GNOME,
KDE and Hyprland binding sections in `docs/`, in Arabic and in English.

## S-019e and S-019f: the long press and the preview

`kwin.sh` (level=1) adds, after the S-019d shows: show 12, where the drag moves
LATE_MS (170 ms by default) after `sbarhive drop` returns, so its first enter on
the catcher falls inside the 300 ms window of manager decision 12. Since S-021
AC4 the daemon counts that window from the placement, and the check counts it
from the catcher's ready commit (its first commit after its first configure,
`catcher_ready` in `layer.sh`), which comes earlier, so the bound is stricter
(one move to the drag, `set_position` twice, the catcher region the hive
rectangle after it); show 13, a PNG with a thumbnail seeded in the sandbox cache
held over the centre cell (200 or more pixels of the thumbnail's sky colour in
the centre, `pixels.py rgbcount`); `preview_checks` (lib.sh) on every show: one
`hive preview: ...` line per enter and none before it, `icon image` for a PNG,
`icon pdf` for the PDF, `thumb W H` for the seeded one. Then the long press
cycles, outside the per-show marks: `sbarhive drop --hold` held (no surface at
350 ms, one by 550 ms; the release closes it within 500 ms), a tap (release at
200 ms: no surface within 1000 ms), a drag over the WEBP cell that leaves and
comes back while the key is held (decision 3: it stays), then the release
(destroyed within 500 ms, no file), a release with no press (exit 0, nothing
shown), and `hold_ms = 1000` written to the sandbox config with the daemon
running (the next press shows after 950 ms). Every command there must exit 0
within 200 ms, or the negative checks would pass on a rejected option.
`gnome.sh` adds show 6 (the seeded thumbnail, `thumb W H`) and `preview_checks`.

`budget.sh 20 1 hold` runs `measure.sh` with `DROP=1 HOLD=1`: every show
line says `mode=hold hold_ms=400`, `press_to_catcher_ms` is `catcher_ms`
plus 400 to 450 ms, and the median `catcher_ms` (from the end of the
threshold to the catcher's ready commit, S-021 AC8; it was `first_frame_ms`
before S-021) is under 150 ms.

## S-021: the drop hive only for a drag of files with an action

On the layer path a drop show binds a hidden catcher and draws the hive only
once a drag of files with an action enters it. With `SBARHIVE_DEBUG_FOCUS` the
daemon prints exactly one of these lines per drop show there: `hive reveal: N`
(N the ids of the `drop cells:` line before it) or `hive probe: hidden (R)`, R
one of `pointer`, `no-uri-list`, `no-actions`, `release`, `cap` and, since
S-022, `replaced`. `layer.sh` reads them per show (`probe_checks`): one line, no
`attach` on the hive subsurface before a reveal or in a hidden show, and the
hive's first `attach` after the drag's first `wl_data_device.enter` on the
catcher. `kwin.sh` checks shows 1 to 13 that way, a drag of `notes.unknownext`
(no action, `hidden (no-actions)`) and the long press release (`hidden
(release)`); `tests/hyprland/repro.sh` adds the cases with no drag (`hidden
(pointer)`, `hidden (release)`), a text drag (`source.py` line `text:WORDS`,
`hidden (no-uri-list)`) and the cycle of AC9. AC8: in every revealed show of
`repro.sh` (eight since S-022) and of `kwin.sh` level=1 (15 since S-022) the
hive's first `attach` comes at most 150 ms after that first enter, by the trace
timestamps (`reveal_ms` and `reveal_budget` in `layer.sh`). `window_path.sh`
runs `gnome.sh smart` and `tests/overlay/gnome.sh` unchanged and finds no probe
or reveal line in their logs (AC10). `hold_docs.sh` checks the Hyprland README,
in Arabic and English, for the hive showing only during a drag of files with an
action, the drag starting first, and nothing shown otherwise,
`docs/architecture.md` for the probe and its reasons (six since S-022), and the
CHANGELOG line (AC11, AC12).

## S-022: three follow-ups of the drop hive

From `backlog/S-022-drop-followups.md`. AC1 in `tests/hyprland/repro.sh`
and AC2 in `kwin.sh` (`hold_cycles`, after the hold-over show): a PNG drag,
`drop --hold`, the drag over the revealed hive's WEBP cell and `drop
--release` there (the layer destroyed within 500 ms, the button let go in
place writes no WEBP), then a second PNG drag whose enter must be read
(`layer.sh` `again_checks`: one `drop cells:` line, `hive reveal: N` with N
the `sbarhive actions` count, an `accept` naming a type, the WEBP written)
and no protocol error. AC4 in `repro.sh`: a PNG drag rests 400 ms at the
centre of a revealed hive; the `attach` requests on the hive subsurface in
the 200 ms from its first (`attach_burst`) are 4 or more with motion and 1
or 2 on a daemon with `SBARHIVE_REDUCED_MOTION=1`, and a normal show's count
is an INFO line. AC6 in `kwin.sh`: the `over` and `thumb` shots start 300 ms
or more after their show's first `attach` (`shot_times`). AC7 in both
scripts: two `drop --at` shows with no drag, the second while the first
still probes, then a move into the second's rectangle: `hidden (replaced)`
then `hidden (pointer)`, the first between the two shows' `hive surface:`
lines, nothing drawn (`replace_checks`). `hold_docs.sh` checks
`docs/architecture.md` for the six reasons and the opening animation at the
reveal, the S-022 CHANGELOG line, and that no prose line of this file is
longer than 80 columns (AC9, AC10).

## S-014e1: the GlobalShortcuts portal (AC13, AC14, AC17 to AC19)

`portal.sh` runs the daemon against `shortcuts_portal.py`, a fake portal
with Registry and GlobalShortcuts version 2 that logs every call with a UTC
stamp (`portal_log.py` reads the log). It uses one private KWin and a
private bus with its own configuration and an empty service directory, so
nothing is D-Bus activated and the real portal never starts. Phases: `ac13`
(the call order, the BindShortcuts fields and the `portal shortcuts:` line,
then a portal without Registry), `ac17` (the timed signals and the AC18 a
and b shows), `ac19` (six fallback modes, 30 s each, and the remembered
denial in `config.toml`) and `hold` (five `sbarhive open` shows while the
BindShortcuts Response is held). `deps.sh` is AC14: the portal client
dependency, one zbus, no tokio in the daemon, and a `docs/engines.md` row
for every new package in `Cargo.lock`.

## S-014e2: the real portal in GNOME and KDE (AC18 c, AC20, AC21)

`portal_real.sh` holds the shared part of two modes. `gnome.sh smart portal`
starts `shortcuts_portal.py ok` on the private bus (the installed portal stays
blocked by `tests/no_portal.sh`): a PNG drag, the fake's `Activated`, the hive
a window centred in the work area with the waiting text before enter, no
`hive probe:` or `hive reveal:` line, and the drop on WEBP before
`Deactivated` writes the file.

`portal-real` is the one place where the installed `xdg-desktop-portal` runs:
`gnome.sh` with xdg-desktop-portal-gnome (its dialog comes from
gnome-control-center's global shortcuts provider) and `kwin.sh` with
xdg-desktop-portal-kde. Nothing changes for the live desktop: the private bus
starts the services itself, with `XDG_CURRENT_DESKTOP` set before
`dbus-run-session`, and the throwaway data directory gets the shipped
`packaging/io.github.m7mmad_omar.SbarHive.desktop`. Checks: the portal process
is on this bus, the live desktop's portal PIDs are the same after the run, and
no private portal process is left. The daemon's output goes through
`real_portal.py stamp` (a UTC stamp per `portal` line) and `dbus-monitor`
logs `Activated` and `Deactivated` of the backend and of the frontend.

AC21 run 1: the backend dialog is found (a window of neither the daemon nor
the source), accepted by a click on "Add" (GNOME) or "OK" (KDE), and the
daemon prints one `portal shortcuts: bound drop (T)`. AC20 (a): three drags,
Super+Z held until the hive shows, the drop on WEBP with the key held. (b):
no drag, Super+Z held 600 ms: `deactivated` 500 ms or more after `activated`
and within 200 ms of the release. (c): a 150 ms tap makes no hive surface
within 1000 ms. AC21 run 2: the daemon and every portal process end, a new
daemon on the same profile: no new window within 5 s, the bound line, and one
held drop writes the file.

## S-014e3: the real portal in Hyprland (AC20) and the portal docs (AC22)

`tests/hyprland/repro.sh --portal` (with `tests/hyprland/portal.sh`) runs
`real_portal_run` in the nested Hyprland with xdg-desktop-portal-hyprland.
The bus gets the nested names through `dbus-update-activation-environment`
(no `--systemd`), and Hyprland runs with `HYPRLAND_NO_SD_VARS=1`. After the
bound line, the hook `portal_after_bound` reads the ID from `hyprctl
globalshortcuts` and appends its Lua `global` bind to the private config.
T in `bound drop (T)` must equal the reply's `trigger_description` on the
bus (`real_portal.py triggers`); xdg-desktop-portal-hyprland sends "".
Two last cases let Super+Z go over a revealed hive, and after the drag has
ended over the source: the layer goes within 500 ms of the key-up.
`portal_docs.sh` reads the portal section of `docs/drop-shortcut` in both
languages, the Hyprland README pair and CHANGELOG.md.

## S-014f3: the portal's activation token (AC15)

`token.sh` holds the five fake cases of AC15 (a), run by `portal.sh ac15`
(KWin, layer path) and by `gnome.sh smart portal` (window path): a held PNG
drag, the fake's timed presses (`seq drop own 0:A:fake-A 100:A:fake-B`) or
`drop --hold --token X`, the hive, a drop on the centre cell, the release.
Each counts `xdg_activation_v1.activate` in the trace of that show.
`gnome.sh strict portal-real` runs AC15 (b) with the real portal under
focus-new-windows strict: the trace's token against the last frontend
`Activated` before it (`real_portal.py last`), and the focus window by
`Shell.Eval`; then the control (c), `drop --hold` with no token.
`portal.sh ro`: a denial whose save fails leaves memory as the file is.

## S-014f4: a key changed in the desktop's store (AC16)

`rebind` after `portal-real` runs `rebind_run` of `portal_real.sh` instead
of AC20 and AC21. After the consent, `rebind_store.py` sets `drop` to
Super+Y in the store on the private bus: on KDE with the kglobalaccel
setter named in its `Introspect`, on GNOME in the `shortcuts` key that
`dconf list` finds. Then Super+Y, Super+Z and Super+Y held mid-drag with
the same daemon (KDE checks, a GNOME INFO line), a restart of the daemon
and of every portal process, one bound line naming Y, and the three cases
again. `REBIND_CONTROL=1` skips the store write, so the Y checks fail.
`portal_docs.sh` reads that GNOME INFO line from the output folder
(`gnome-smart-portal-real-rebind.txt`), for AC28.

## Input

- KWin: `kwin-input/`, a small client of `org_kde_kwin_fake_input` (copied
  from the spike, outside the workspace). KWin allows it only with
  `KWIN_WAYLAND_NO_PERMISSION_CHECKS=1`, which the script sets for its own
  private compositor.
- GNOME: `input_gnome.py`, Mutter's RemoteDesktop API on the private bus, with
  `keydown`/`keyup` to hold Shift at the drop.
  Since S-014e2 it also knows `Return`, for the portal dialog.
