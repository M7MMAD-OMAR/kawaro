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
- AC2 (amended by D2): in a rendered snapshot of the hive with 8 or more cells (software renderer, `tests/snapshots.sh` path), every pixel in the four window corners has alpha 0; every pixel outside the union of all cell hexagons grown by `reach` + 1 px (and outside the stack hexagons and count badge when several files are shown) has alpha 0, with at least 20 such points per scenario; every pixel at a cell centre has alpha 255.
- AC3 (amended by D2, same gap rule as AC2): the same holds for a 1 file and a several-files hive in light and dark schemes, and in LTR and RTL.
- AC4: `geometry` and `drop_tests` pass unchanged (picking is the same); the centre cell still shows the full name and thumbnail.
- AC5: `text-primary` text still has 4.5:1 or more on every surface it sits on (`contrast.rs` passes unchanged), since no text sits on the removed plate.
- AC6: nested Hyprland `tests/hyprland/repro.sh`: the screenshot of the shown hive has desktop pixels (the background colour of the test client) in the gap between two cells and in the corners of the hive rectangle. Recorded as an image in the story log.
- AC7: window path: a test (or a recorded check) shows the created window requests an alpha surface; the GNOME result is marked "assumed" in the story log with the reason.
- AC8: gates in CLAUDE.md pass; snapshots that legitimately change are regenerated and listed in the log; one `CHANGELOG.md` line (English and Arabic).

## Out of scope
Glass or blur (S-017), new motion, any change to cell look.

## Review log
- Tester (2026-10-26): tests in `crates/ui/src/s074_tests.rs` (registered in `lib.rs` under `#[cfg(test)]`). Pixel tests use a private `slint::platform::Platform` with `MinimalSoftwareWindow` (the testing backend cannot `take_snapshot`), a zeroed RGBA buffer, the real `Hive` window, `KAWARO_COLOR_SCHEME` for light and dark, `Lang::En` and `Lang::Ar`.
  - AC1 `ac1_hive_slint_has_no_plate_and_a_transparent_window`: FAILS (`hive.slint still uses Tokens.hive-plate`).
  - AC2 `ac2_eight_cells_corners_and_gaps_clear_cell_centres_opaque` (8 cells): FAILS (`corner pixel (0, 0) has alpha 255`).
  - AC3 `ac3_one_file_light_ltr`, `ac3_one_file_dark_rtl`, `ac3_several_files_light_rtl`, `ac3_several_files_dark_ltr` (9 cells): all FAIL the same way (corner alpha 255).
  - AC7 `ac7_window_path_requests_an_alpha_surface`: FAILS (`no code asks winit for a transparent window (with_transparent(true))`); source level, it accepts `with_transparent(true)` or `set_transparent(true)` in lib.rs, platform.rs or win_window.rs. GNOME stays "assumed".
  - AC4 and AC5: existing `drop_tests`, `geometry` and `contrast.rs` must pass unchanged; they pass now and are the regression guard (not new tests).
  - Removed S-062's `ac6_hive_slint_has_an_opaque_plate_and_no_transparent_window` from `crates/ui/tests/contrast.rs`: it asserts the plate and contradicts AC1 (the rest of contrast.rs is unchanged).
  - AC6 is not automated by the tester: it needs the 1170-line nested Hyprland harness inside a private KWin and a screenshot; recorded as INFO to run by the reviewer/implementer (grab the hive rectangle, check gap and corner pixels equal the test client's colour).
  - Run (`cargo test -p kawaro-ui --lib s074`): `test result: FAILED. 0 passed; 7 failed`.
  - Manager decision D1: S-062's `ac6_hive_slint_has_an_opaque_plate_and_no_transparent_window` is removed from `contrast.rs` because S-074 supersedes it.
- Implementer (2026-10-26), STOPPED, tests need a manager decision:
  - Done: `plate` deleted, root `background: transparent` (hive.slint); `with_transparent(true)` added to `token_hook` (lib.rs, Linux window and layer path) and to the Windows hive attributes (win_window.rs). Slint 1.18.1 only sets transparency itself on macOS, so the explicit call is needed. Layer path already renders into a zeroed ARGB `Bgra` buffer (`Bgra::default()`), no opaque clear.
  - Passing: AC1, AC7. Failing: AC2 and the four AC3 tests (`s074_tests`).
  - Cause 1: the gap sample (midpoint of two neighbour centres, plus or minus 1 px) sees the S-062 edge hexagon (`hive-edge`, `border-hex` 2 px drawn outside the fill, `reach` in hex-cell.slint): alpha 4 to 75 in a 6 px gap. With `reach: 0px` AC2 and the 1-file tests pass, so this is the only cause for them. Fixing it means changing the cell look (out of scope).
  - Cause 2: with several files, the stack hexagons (`stack-offset` 4, two layers = 8 px, wider than the 6 px gap) are opaque in the gap between the centre and ring-1 cells: alpha 255 at the gap midpoint (3 files, both LTR and RTL). That is the existing S-0xx design, not the plate.
  - Also failing, same class as D1: `contrast.rs` `ac8_design_lint_is_clean_with_the_new_look_in_place` and `ac8_no_long_dashes_in_the_files_this_story_touches` assert `hive.slint` contains `Tokens.hive-plate`. They contradict AC1 and need the same removal as the S-062 ac6 test.
  - Gates run: fmt OK, clippy OK, design-lint clean; `cargo test --workspace`: lib 173 passed, 5 failed (s074 above); contrast.rs 8 passed, 2 failed (above); everything else green. Snapshots, CHANGELOG, AC6 not done (blocked on the decision).
  - Manager decision D2: the implementation is right and the tests over-specified. The cell edge ring (`reach` 2 px outside each cell fill, alpha 4 to 75) and the stack hexagons (8 px offset, behind the centre when preview-count > 2) are part of the cell look and stay. AC2 and AC3 are amended above: corners alpha 0; gap check is every point outside all hexagons grown by `reach` + 1 px, the stack hexagons and the badge, alpha 0, at least 20 points per scenario; cell centres alpha 255.
  - D2 part 2: `contrast.rs` `ac8_design_lint_is_clean_with_the_new_look_in_place` and `ac8_no_long_dashes_in_the_files_this_story_touches` no longer require `Tokens.hive-plate` in hive.slint (they contradicted AC1); design lint clean and no long dashes stay asserted.
