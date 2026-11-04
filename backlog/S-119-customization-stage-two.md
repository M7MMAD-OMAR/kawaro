# S-119 Customization stage two and the live readout
Status: done (2026-11-04)

## Story
As a user who wants more than the default resize, I want a compact option panel inside the same hive window when I ask for it, with a live preview, the resulting size and ratio, the option controls and Run and Reset, so that one click still runs the default and customization is my deliberate second gesture. This is the owner's customization request made visible. Crop, straighten and the other image families come after this story.

## What this story will prove
- The one click promise survives: the hive still appears in under 150 ms, the cells still run the default on a click, and the panel opens only from the Customize pill or Shift+Return on a highlighted customizable cell.
- The window does not resize or move: the panel is compact inside the same 393 by 362 window, so no new window is created, focus cannot be stolen and Esc or Back returns to the hive with nothing changed.
- Every control is built from the S-117 schema, not from a hand written list per action, and every value is resolved with `resolve`, so the readout, the CLI and MCP agree.
- The preview is the masked draft from the S-118 worker and never blocks the UI; the S-118 bench remains the budget gate, and this story adds no per-frame timing test.
- Arabic and English are both originals: the layout is mirrored by hand, digits stay Latin, and the reviewed contract puts the value and unit in every label and the digits in bidi isolates. The delivery gaps are recorded in the review log.

## Dependencies
- S-117 (the field schema, `kind = "form"`, readout tokens, resolve semantics).
- S-118 (preview worker, hexagon mask, `image_geom::target`, the bench).
- The vendored Material 3 controls (slider, segmented buttons, checkbox, text field) and the design tokens, no new dependency.
- Blocks the later crop story and the wider image family work. Delivered in this block; the delivery deviations are in the review log.

## Acceptance criteria (as reviewed, with the delivery deviations in the review log)
- AC1 (doors): a highlighted cell whose action carries options shows the clickable Customize pill under the centre cell. The pill and Shift+Return on the highlighted cell are the only doors: the window backends report no double clicks, so a double click is not a door, and there is no ring gesture and no always-ask switch in this delivery. A plain click on the cell body still runs the action with its defaults at once, the same on the layer and window paths. Escape or the Back button closes the panel and runs nothing.
- AC2 (compact panel, no resize): the panel is drawn inside the existing 393 by 362 window and the window neither resizes nor moves. The panel covers the hive rectangle with `surface-3`, a full 1 px `border` and the `radius-lg` token, and holds the title row, the pointy top preview well at 22 percent of the width, the option column, the preset row and the footer with Run, Reset and Back. The hive content behind is hidden while the panel is open. Every colour, length and radius is a `design/tokens.json` token; no shadow, no gradient and no one sided border. Reduced motion changes nothing because the panel has no motion of its own.
- AC3 (live preview and stage error text): the preview well is a pointy top hexagon showing the masked draft from the S-118 worker, capped at `MAX_DRAFT` 512 px, with newest wins and at most one decode at a time. When the first file is not a readable image or the decode fails, the panel shows a short stage error text instead of a picture (for example "preview not available"), the readout falls back to the resolved params and never invents a size, and the UI is never blocked. The S-118 bench remains the timing gate; this story adds no per-frame budget test.
- AC4 (readout): one line shows the tokens the action declares: the target dimensions, the ratio (for example 3:2), the percent change against the source and the `output_name` the core naming will produce. For `image.resize-50` the four tokens are `dimensions`, `ratio`, `percent` and `output_name`; for an action without image dimensions the line shows the resolved params as short key: value pairs; for a missing image it falls back to the params without a size. The line updates while a value moves.
- AC5 (fields and presets): a control is built per field kind from the S-117 schema (percent, px, px-pair, enum, bool, int, float, string); preset chips come from `preset` data; Reset returns every control to the base `[action.params]` value; Run resolves the values and submits one job. The first acceptance bar is `image.resize-50` (two fields, mutual `clears`, the full readout) and the four quality specs (one integer field with a percent unit).
- AC6 (RTL and accessibility): the Arabic build mirrors the panel and the column order by hand and mirrors the preview well placement. Western digits in the readout and the values are wrapped in bidi isolates (U+2068 FSI and U+2069 PDI) so they keep their order inside Arabic text. Every control's accessible label carries the field name, its current value and its unit. Focus is a 2 px `text-primary` outline, never colour alone, and the panel can be driven with the keyboard only.
- AC7 (performance): the hive first frame stays under 150 ms; the panel adds no frame of its own while the hive is closed, and the 150 ms budget script and the S-118 bench remain the gates. The tray progress and notification rules are unchanged.
- AC8 (agreement): for a table of field values, the preview dimensions, the readout and the final file dimensions agree exactly, because both call `image_geom::target`, and the submitted job carries exactly the resolved params.
- AC9 (Arabic and English strings): every new string exists in both languages, the Arabic holds Arabic letters and no Arabic-Indic digits, and no changed file contains a long dash.

## Test cases
The panel model has exactly ten tests in `crates/ui/tests/stage_two.rs`.

| Test | Proves | Type |
| --- | --- | --- |
| `resize_builds_number_rows_from_the_base_params` | One row per field, base values, title and resolved params (AC5) | unit |
| `readout_uses_the_shared_image_geometry` | Readout dimensions, ratio and percent from `image_geom` (AC4, AC8) | unit |
| `setting_max_side_clears_scale_and_changes_the_readout` | Mutual clears and the readout change (AC4, AC5) | unit |
| `enum_rows_carry_the_choices_and_resolve` | Enum choices and resolution (AC5) | unit |
| `bool_and_text_rows_resolve_and_respect_clears` | Bool and text fields and clears (AC5) | unit |
| `presets_apply_their_values` | Preset values reach the resolved params (AC5) | unit |
| `an_action_without_options_has_no_panel` | No panel without a spec (AC5) | unit |
| `a_missing_picture_is_a_soft_failure` | No picture, params readout and no panic (AC3, AC6) | unit |
| `the_two_languages_carry_their_own_strings` | Arabic and English strings, no Arabic-Indic digits (AC6, AC9) | unit |
| `the_panel_callbacks_fire_in_a_headless_window` | The pill, value, preset, Run, Reset and Back callbacks in a headless window (AC1, AC5) | unit |

No test covers the compact panel geometry (AC2), the stage error text, the `output_name` token, the value and unit in accessible labels or the bidi isolates. Those are reviewer checks and the deviations are recorded in the review log.

## Out of scope
Crop, straighten, target file size, metadata, PDF page preview and any new action or engine option. Per-file options. A settings page redesign. Batch previews beyond "and N more, same settings". MCP and CLI surfaces (S-120 owns the CLI, ADR 0011 the MCP). Any change to the 150 ms hive promise.

## Review log
- 2026-11-04, planner: draft from `docs/research/image-editing-and-preview.md` section 6.2 and 6.4 and the order in `docs/research/agent-integration-and-expansion.md` section 9. Stage 2 is one window, not an editor; the schema and the preview worker are the prerequisites. Budgets are targets to prove. Not approved, not in this block.

- 2026-11-04, implementer: stage 1 is green: crates/ui/src/stage_two.rs (rows from the schema, set_value with clears, apply_preset, reset, resolved, readout through image_geom, newest-wins preview thread with the hexagon mask and MAX_DRAFT 512) and 9 model tests; the panel draws in hive.slint with the Customize pill, Shift+Return, a ScrollView of Material controls and the token palette; build.rs gives the hive compile the material library path, HiveCell.customize is plumbed with the mechanical false in 19 test and example literals, and the hive window behaviour is unchanged while stage-two is false.
- 2026-11-04, implementer: gates green at the checkpoint: fmt, clippy ui+daemon, 557 ui tests (28 binaries) and 352 daemon tests (21 binaries), design-lint.
- 2026-11-04, lead: the next pass is lib.rs wiring (present_options, callbacks, preview timer), then daemon build/submit, then the headless Slint test. The 400 ms double-click delay was rejected: the door is a clickable Customize pill plus Shift+Return, so a plain click stays immediate.

- 2026-11-04, implementer: lib.rs wiring complete: on_customize, on_options_run, on_options_cancel, the open panel state, a 16 ms preview timer and the customize hint; Hive::present_options re-shows the same window with two rings and fills the panel; Run resolves and closes, Reset and presets rebuild the rows; the daemon builds StageTwoState, resolves with with_params, submits through jobs::submit and closes the gate.
- 2026-11-04, implementer: double-click was rejected because the winit backend reports no double clicks; the door is the clickable Customize pill plus Shift+Return, so a plain click stays immediate. 10 stage_two tests, including the headless stage-run and option-changed test.
- 2026-11-04, implementer: the five s108 and two s108b red tests were cell-count changes caused by S-127's three new image actions; the fixtures now use a supported kind that leaves room, exact assertions unchanged and no production change.
- 2026-11-04, lead: gates green (fmt, clippy ui+daemon, 558 ui tests, 352 daemon tests); caveats: the panel re-show path is headless only, not snapshot checked, and the Material in_out staleness is handled by reopening the panel.
- 2026-11-04, planner: ACs amended after the reviewer decisions, status done kept. Deviations recorded, not hidden: (1) the 660 by 420 window growth is gone, the panel is compact inside the same 393 by 362 window, so AC2 states that delivery and the window never resizes; (2) the ring gesture and the always-ask switch are gone, the Customize pill and Shift+Return are the only doors because the window backends report no double clicks; (3) the preview error path shows no picture and a params readout, but no explicit stage error text is delivered yet; (4) the readout shows dimensions, ratio and percent and the resolved params fallback, but not the `output_name` token; (5) accessible labels carry the field label only, not the value and unit, and no bidi isolates U+2068 and U+2069 are used; (6) the test file has the ten tests named above and covers neither AC2 nor those three gaps.

- 2026-11-04, independent review: REWORK with concrete blockers (readout tokens, layout ratios, silent resolve error, a11y/bidi, preview note, dead callback). All fixed: readout follows dimensions/ratio/percent/output_name and is bidi isolated, stage-error shows the resolver text, labels include value and unit, layout.stage-well-r and layout.stage-label-w tokens replace the ratios, enum choices are a vertical list, the readout updates only when the text changes, and a preview note appears when a picture is unavailable.
- 2026-11-04, tester: 12 stage_two tests including the resolve-error case; the stage-error wiring itself has no integration getter (noted).

- 2026-11-04, implementer: added read-only seams Hive::stage_error and Hive::set_option so the panel wiring is pinned end to end; on_option_changed delegates to the same Inner::set_option. The tester's test now presents the panel with the testing backend, sets an out-of-range max_side and asserts the error text, then sets a valid value and asserts it clears.
- 2026-11-04, tester: stage_two 12/12 green with the real wiring; clippy and fmt clean.
