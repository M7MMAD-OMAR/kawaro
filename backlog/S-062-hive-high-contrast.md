# S-062 Make the hive high contrast and opaque so it reads on any desktop
Status: draft

## Story
As a user with a dark (or light) file manager open behind the hive, I want every cell, edge and label to be clearly readable, so that I never lose a cell in the background or mistake one label for another.

Owner report: cells use `surface-2` (#27231C) with border #3E372C on a dark desktop and nearly vanish; the window background is `transparent` (`crates/ui/ui/hive.slint`), so file manager text shows through the gaps between cells; TIFF, WEBP and 50% are hard to tell apart.

Decision (keeps it to one day): high contrast becomes the only hive look. No "Standard" vs "High contrast" setting now; a setting is a follow-up only if the owner asks. Settings files stay untouched.

## Acceptance criteria
Contrast ratio means the WCAG 2.1 relative luminance ratio, computed from `design/tokens.json`, for both `light` and `dark`.
- AC1: New tokens in `design/tokens.json` (single source, generated into Slint by `crates/ui/build.rs`): `color.<scheme>.hive-plate`, `hive-edge`, `hex-fill`, `hex-fill-centre`, `hex-ink`, plus `border-width.hex` = 2 and `font.size.cell` = 14 (up from 12 for the centre label; cell code stays 14 with weight semibold 600). Initial values are a proposal, tuned by the tester until AC2 to AC5 pass: dark `hex-fill` near #0B0A08, light `hex-fill` near #FFFFFF.
- AC2: Text on cells: `hex-ink` vs `hex-fill` >= 7:1 and `hex-ink` vs `hex-fill-centre` >= 7:1, in both schemes.
- AC3: Cell edge: `hive-edge` vs `hive-plate` >= 3:1 and `hive-edge` vs `hex-fill` >= 3:1, in both schemes.
- AC4: Plate rim against a plain backdrop: `hive-edge` vs #000000 (dark scheme) >= 3:1 and `hive-edge` vs #FFFFFF (light scheme) >= 3:1.
- AC5: Highlight: `accent-ink` vs `accent` >= 7:1 in both schemes, and the highlighted cell edge vs `hive-plate` >= 3:1.
- AC6: Opaque: `hive.slint` has an opaque plate (alpha 255, color `hive-plate`, radius `radius-lg`) behind all cells and the window `background` is no longer `transparent`. Sampling a rendered hive at every gap midpoint gives exactly the `hive-plate` colour (0 pixels of any other colour). No glass token is used (blur path stays for S-017).
- AC7: `hex-cell.slint` uses only the new tokens for fill, stroke (`border-width.hex` = 2 on all six sides) and ink. Highlighted state: fill `accent`, ink `accent-ink`, edge `hive-edge`. No `surface-2`, `border` or hand written value remains in that file.
- AC8: Rules: `scripts/design-lint.sh` returns 0 matches (no shadow, gradient, one sided border, hand colour, long dash).
- AC9: Existing hive snapshots are regenerated for dark and light, Arabic (RTL) and English; layout positions of all cells are unchanged (pixel centre of each cell identical to the old snapshot within 0 px), only colours, edge width and the plate differ.
- AC10: Labels of 6 Latin characters (for example `TIFF`, `WEBP`, `50%`) and the longest Arabic label are not elided at size `md` (72) with font size 14.
- AC11: Performance: time from invoke to first frame of the hive stays < 150 ms (existing budget test), measured with the plate present.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Parse `tokens.json`; assert each new key exists in both schemes, is a valid #RRGGBB, `border-width.hex == 2`, `font.size.cell == 14`; assert generated `tokens.slint` contains the same hex strings | unit |
| AC2 | `crates/ui/tests/contrast.rs`: compute ratio for both pairs per scheme; assert `>= 7.0` and print the actual ratio on failure | unit |
| AC3 | Same file: assert both edge pairs `>= 3.0` per scheme | unit |
| AC4 | Same file: dark `hive-edge` vs #000000 and light `hive-edge` vs #FFFFFF, assert `>= 3.0` | unit |
| AC5 | Same file: `accent-ink` vs `accent` `>= 7.0`; highlighted edge vs plate `>= 3.0` | unit |
| AC6 | Render the hive headless over a #FF00FF test backdrop (software renderer); read pixels at each gap midpoint and the plate corners; assert every one equals `hive-plate` RGB and none is magenta; grep `hive.slint` for `background: transparent` gives 0 hits | snapshot |
| AC7 | Grep `hex-cell.slint` for `surface-` and `Tokens.border;` gives 0 hits; render a highlighted cell and assert its centre pixel is `accent` and its edge pixel is `hive-edge`; assert edge band is 2 px wide | snapshot |
| AC8 | Run `scripts/design-lint.sh`, exit code 0 and 0 matches; search the diff for U+2013 and U+2014, 0 hits | unit |
| AC9 | Compare cell centre coordinates from old and new render (exported list), all equal; compare new PNGs to committed snapshots for 4 variants (dark/light x en/ar) | snapshot |
| AC10 | Render cells with `TIFF`, `WEBP`, `50%`, and the longest Arabic action name; assert text width <= cell inner width and the string is not elided (rendered text equals the input) | ui |
| AC11 | Run the existing hive open timing test 20 times; assert p95 < 150 ms | perf |

## Out of scope
- A "Standard" vs "High contrast" settings choice, any change to `settings.rs` or `settings.slint`.
- Glass or blur surfaces (S-017), hive geometry, cell sizes, ring layout, motion, the settings window, the tray.
- Contrast of the settings window and site.

## Review log
- (empty)
