# S-074 The hive has no background plate: only the hexagonal cells show
Status: approved (2026-10-26, manager, from the maintainer's first look at the shipped hive)

## Story
As a user, I want the hive to be only its hexagonal cells over my desktop, so that it does not look like a dark square window pasted over the file manager.

## Background
- The hive paints an opaque rounded rectangle (`plate`, `Tokens.hive-plate`, `crates/ui/ui/hive.slint`) and the window itself has `background: Tokens.hive-plate`. The maintainer's screenshot shows a dark square behind the cells; he wants it removed (option 1). Glass over a compositor blur is S-017 and comes later; it will add its own surface, it does not depend on this plate.
- The picking geometry already treats gaps and corners as "no action" (`geometry.rs`), so click behaviour does not change.

## Decisions
1. The `plate` rectangle is deleted and the root `Window` background is `transparent`. The `hive-plate` token stays in `tokens.json` and `Tokens` (S-017 and the site use it); only its use in `hive.slint` goes.
2. The cells keep their own opaque `surface` fills, borders and icons. Nothing else is added behind them (no shadow, no outline, no replacement panel).
3. Layer path (Hyprland, KDE, wlroots): the hive buffer already carries alpha, so transparent pixels show the desktop. Window path (GNOME, X11, Windows, screen reader): the window must be created transparent. The implementer checks what Slint does for a transparent root background with `no-frame: true` on winit and sets the winit transparency attribute if Slint does not. Real GNOME is not testable here, so the result is recorded as "assumed" for GNOME and verified for Hyprland (nested, `tests/hyprland/repro.sh`).
4. The drop hive (S-021, S-047) uses the same component and gets the same change.

## Acceptance criteria
- AC1: `hive.slint` has no use of `Tokens.hive-plate`, and the root window background is `transparent`.
- AC2: in a rendered snapshot of the hive with 8 or more cells (software renderer, `tests/snapshots.sh` path), every pixel in the four window corners and every pixel in the gap between two neighbouring cells has alpha 0; every pixel at a cell centre has alpha 255.
- AC3: the same holds for a 1 file and a several-files hive in light and dark schemes, and in LTR and RTL.
- AC4: `geometry` and `drop_tests` pass unchanged (picking is the same); the centre cell still shows the full name and thumbnail.
- AC5: `text-primary` text still has 4.5:1 or more on every surface it sits on (`contrast.rs` passes unchanged), since no text sits on the removed plate.
- AC6: nested Hyprland `tests/hyprland/repro.sh`: the screenshot of the shown hive has desktop pixels (the background colour of the test client) in the gap between two cells and in the corners of the hive rectangle. Recorded as an image in the story log.
- AC7: window path: a test (or a recorded check) shows the created window requests an alpha surface; the GNOME result is marked "assumed" in the story log with the reason.
- AC8: gates in CLAUDE.md pass; snapshots that legitimately change are regenerated and listed in the log; one `CHANGELOG.md` line (English and Arabic).

## Out of scope
Glass or blur (S-017), new motion, any change to cell look.

## Review log
(empty)
