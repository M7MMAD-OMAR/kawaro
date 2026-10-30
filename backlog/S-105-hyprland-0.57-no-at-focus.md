# S-105 Hyprland 0.57: a drop show without --at takes the first click
Status: draft

## Problem
On Hyprland 0.57 and above, `kawaro drop` or `kawaro drop --hold` without `--at` (the portal shortcut, a plain bind) maps its full surface catcher under a still pointer and Hyprland enters it at once. The catcher then takes the pointer focus and swallows the first click, which would otherwise reach the application below. The Alt bind passes `--at`, so its catcher is only the hive rectangle with a 6 px hole at the pointer (S-104) and is not affected. KWin and GNOME do not enter on map.

## Measured (private Hyprland 0.57.0 harness)
- `tests/hyprland/repro.sh`, S-021 AC5 (b): pointer still at (40, 40), `kawaro drop`, a click 0.9 s later reaches keys.py 0 times (expected 1). On 0.56.2 it passed.
- S-104 ignores the map time enter for placement and for the hide, so the show survives; the click is still taken by the catcher.
- S-022 AC4 (reduced motion, 3 attach in 200 ms where 1 or 2 are allowed) also fails today, on a clean ffa916c build too. It is a frame count of the hive subsurface after a drag reveal and involves no pointer enter, so it is NOT this effect: it is listed here only as an open, unrelated flake to look at separately.

## Rejected options
- A hole at the map time enter position: Hyprland does not refocus when the region changes, so the focus stays on the catcher.
- An empty input region: the drag enter could not land.
- A delayed full region: the enter comes when the region grows and takes the same focus.
- A hole at the pointer from the start without `--at`: the catcher's first region is then the whole surface minus the hole, not the hive rectangle, which S-019 decision 12 forbids, and it cannot be placed before the anchor is known.
- `hyprctl cursorpos` as the `at` (daemon side, 6 ms): AC5 (b) passes, but the hive then anchors at the cursorpos instead of the drag's first `wl_data_device.enter` (210, 180 against 480, 270 in the harness); the hive lands at a different place than the cell the drag is pinned to, and Hyprland ends the drag with `leave` and no `drop` although the offer was accepted with a type. Decision 12 also turns red (region sent before the ready commit). Two isolating runs: without the hole the drop still fails, without the fill it passes.

## Open ideas
- Fill `at` only for a hold press (the portal path) and keep plain `kawaro drop` as is.
- Find why Hyprland withholds the drop when the hive is anchored at the cursorpos, then reconsider the fill.
- Ask upstream whether a layer surface can opt out of the enter on map.

## Harness
S-021 AC5 (b) prints `SKIP ... known limit on Hyprland >= 0.57 (S-105)` on 0.57 or newer and asserts as before on older versions.
