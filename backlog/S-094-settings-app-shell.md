# S-094 Settings and welcome window as a real app shell
Status: approved (2026-10-29)

## Story
As a user, I want the settings window to look and behave like an application (header with name and version, page rail, centred content, fixed Save row), so that I can find a setting without scrolling one long form and trust what I installed.

## Why
Owner complaint after the live Windows 11 test of the S-090 build (2026-10-28): one long column of rows, no app name, no version, no pages. This story absorbs and supersedes S-093 D1 to D4 and the owner complaint about the Introduction film. Split (each part is about one day): S-094b adds the About actions (links, copy version, update shortcut); S-094c rebuilds the demo film content and sharpness. S-094 keeps the film's layout rules (AC9).

## Design decisions
- Navigation: the vendored `NavigationRail` (`crates/ui/material/ui/components/navigation_rail.slint`, role tab-list, items with `selected` state) with 5 items, rail on the reading side (left in English, right in Arabic, by reversing the layout order). No hand built rail unless the implementer proves in the review log that the vendored one cannot meet AC2, AC5 or AC7; the keyboard handler is a plain `FocusScope` (the vendored rail has none).
- New tokens in `design/tokens.json` group `layout`: window-width 880, window-height 640, window-min-width 560, window-min-height 560, header-height 64, header-logo 40, rail-width 96, content-max-width 640, row-breakpoint 520, label-column 220, film-max-width 560. No other literal values.
- Logo: `design/brand/kawaro-cells.svg` (light) and `kawaro-cells-dark.svg` (dark), chosen by the colour scheme. Header name is the single string `كوارو | Kawaro` in both languages.

## Acceptance criteria
- AC1 (header): a header of `header-height` px stays visible on every page, holding the logo (`header-logo` px square), the name `كوارو | Kawaro` and the installed version, which equals `env!("CARGO_PKG_VERSION")` (0.4.0 today) character for character.
- AC2 (pages): the rail has exactly 5 items in this order: General (language, theme), Conversion (where results go, folder name, quality), Notifications and integration (notify, notify long only, open folder, drop shortcut row), Updates (the S-090 section unchanged in behaviour), About. Selecting an item shows only its rows. The About page shows name, version, a one line description, the licence `GPL-3.0-or-later` (read from the workspace package field) and the privacy statement (English: "No telemetry. No file leaves your computer."; Arabic equivalent).
- AC3 (content column): content is centred, at most `content-max-width` px wide; at a 1920 px window the left and right margins differ by at most 1 px.
- AC4 (rows): every setting is a label plus control pair. At content width >= `row-breakpoint` the pair is one row (label column `label-column` px, control after it); below it the label sits above the control. At every window width from `window-min-width` to 3840 the gap between a label and its control is at most `content-max-width` px, and a switch is never farther than that from its label.
- AC5 (size): the default window is `window-width` by `window-height`; minimum is `window-min-width` by `window-min-height`. At the default size, each of the 5 pages has content height <= its visible height (0 px of scrolling) in Arabic and English; at the minimum size the rail shows all 5 items unclipped.
- AC6 (footer and state): the Save and Cancel row is visible at the minimum size on every page. A marker "Unsaved changes" (Arabic equivalent) shows when any value differs from the loaded one and hides again after reverting or after Save. The selected page is remembered while the daemon runs (in memory only, never written to `config.toml`); a new window opens on it. The first ever open is General. The update state line has a reserved two line height: starting a check changes the y of every row below it by 0 px (S-093 D3), and the last row is never cut off.
- AC7 (RTL, labels, keys): in Arabic the rail, header order (logo, name, version) and row order mirror; every control, rail item and the logo has a non empty accessible label in both languages; Ctrl+Tab and Ctrl+Shift+Tab move to the next and previous page and wrap (5 presses return to the start); Up and Down move inside the focused rail; Esc still closes without saving.
- AC8 (one shell): `SettingsUi` and the welcome window's settings tab use the same shell component. The welcome window has a 6th item, Introduction, first, shown only in the first run welcome (the standalone settings window never shows it); its old top tab buttons are removed; Cancel in the welcome window returns to Introduction. The Introduction page has no Save row.
- AC9 (film layout): on the Introduction page the demo film sits in a centred card of at most `film-max-width` px, aspect 16:9, `image-fit: contain`, never taller than the visible content area. At default, maximized (1920x1080) and 3840x2160 windows the card is fully inside the window (0 px overflow). The old film (an unbounded 100% wide box, about 1000 px tall when maximized, upscaled about 3x, cut off mid scene) is rejected as a defect, not a style choice. Frame content and sharpness are S-094c.
- AC10 (design rules): all colours, spacings, radii and durations come from tokens; no shadow, gradient or one sided border; `scripts/design-lint.sh` clean; no long dashes.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Open the window in both languages: header version text equals the package version string; logo and name elements found by accessible label | ui (testing backend, new `crates/ui/tests/s094_shell.rs`) |
| AC2 | Select each item in turn: the set of accessible labels on screen equals the expected list for that page and no label of another page is present; About shows `GPL-3.0-or-later` and the exact privacy string | ui (testing backend) |
| AC3 | Resize to 1920x1080: the content column width <= 640 and margin difference <= 1 | ui (testing backend, element geometry) |
| AC4 | Widths 560, 700, 880, 1920, 3840: label-to-control gap <= 640 for every row; below 520 content width the control y > label y, at or above it the y values overlap | ui (testing backend) |
| AC5 | At 880x640, scroll extent minus visible height <= 0 for each page, en and ar; at 560x560 the rail's 5 items all inside the rail | ui (testing backend) |
| AC6 | Change theme: marker visible; change back: hidden; change and Save: hidden. Press a page, drop the window, reopen: same page. Start a check with a fake IPC: y of the row below the state line identical before and after | ui, integration (extends `s090_updates.rs` helpers) |
| AC7 | Arabic: rail x greater than content x, header logo x greater than the version x; all labels non empty; send Ctrl+Tab 5 times and observe page 0, 1, 2, 3, 4, 0; Esc closes | ui (testing backend) |
| AC8 | Welcome window first run: 6 items with Introduction at index 0; settings window: 5 items; both expose the same row labels for the same page; Cancel returns to index 0 | ui (testing backend, `welcome/` tests) |
| AC9 | Render the Introduction at 880x640, 1920x1080 and 3840x2160: card rect inside the window and height <= visible height | ui, snapshot |
| AC10 | `scripts/design-lint.sh`; tokens listed above exist in `tokens.json` with the stated values | gate, unit |

Snapshots: extend `crates/ui/tests/settings_snapshots.sh` (private headless KWin) with the shell. The old references are replaced only after the manager approves candidates.

## Screenshots required for approval
- A: General page, {English, Arabic} x {light, dark} x {default, maximized}: 8 images.
- B: Conversion, Notifications and integration, Updates, About and Introduction, each in English light and Arabic dark at default size: 10 images.
- C: owner captures from the Windows 11 virtual machine of A (default and maximized), a live check left to the owner (precedent S-079).

## Sequencing
S-091 and S-092 edit `crates/core/src/ipc.rs`, `crates/daemon/src/ui.rs` and `tray/`, and S-091 edits `crates/ui/src/updates.rs`. S-094 edits `crates/ui/ui/`, `crates/ui/src/settings.rs`, `welcome.rs`, `design/tokens.json` and its tests, and touches no daemon file (page memory lives in `kawaro_ui`). The only possible overlap is `updates.rs`; S-094 does not change its signatures. So S-094 goes first (owner ranks it above); whoever lands second rebases.

## Out of scope
- New settings or update logic; links, copy version and update shortcut (S-094b); new film frames and sharpness (S-094c); a confirm dialog on Cancel with unsaved changes.

## Review log
- 2026-10-29 tester: tests written from the acceptance criteria only.
  - `crates/ui/tests/s094_static.rs` (file reads: AC8 shared shell, AC9 image-fit, AC10 tokens and design rules). Red run: 4 passed 0, 4 failed: `layout` group missing in `design/tokens.json`; `crates/ui/ui/shell.slint` does not exist (3 tests).
  - `crates/ui/tests/s094_shell.rs` (behaviour in the testing backend, AC1 to AC9, both windows through one `Shell` trait). Red run: does not compile, 33 errors E0599 so far, the type errors stop the compiler before the import errors (`WelcomeWindow::select_page`, `rail_labels`, `page_rows`, `header_rect`, ... not found; `Rect`, `reset_page_memory`, `privacy_statement` are also not exported yet). The hook list is in the file header; the implementer adds them as `#[doc(hidden)] pub` inherent methods with the same name on `SettingsWindow` and `WelcomeWindow`.
  - Allowed test edits for the implementer, only to point source scans at `shell.slint`: `s087_welcome.rs` `ac6_rtl_mirrors_the_button_row_in_arabic` and `ac4_both_buttons_carry_descriptive_accessible_labels`, `s090_updates.rs` `ac7_slint_source_has_an_updates_section...` and `ac6_one_version_line_...`. Nothing else in them changes. The four welcome PNG snapshots stay until the manager approves candidates.
  - Not covered by these tests: screenshots (A, B), the snapshot script extension, the Windows 11 live check (C), the real rendered look and the sharpness of the film.
