# S-124 Image to PDF page options: size, orientation, margin and dpi
Status: done (2026-11-04)

## Story
As a user who combines photos or scans into one PDF, I want to choose the page size, the orientation, the margin and the resolution, so that the sheet matches what I will print or send instead of always being the image's own size. This is P0 item 2 of `docs/research/pdf-feature-benchmark.md` (images to PDF with page size, fit, margin and DPI; orientation is the cheap half of the same item) and the images-to-PDF part of `docs/research/image-editing-and-preview.md` section 7.2. The grid layout stays out of this story.

## Facts found while planning
- Shipped in 0.5.0: `image.to-pdf` is a `combined` action (one PDF for the whole selection, one page per image), written by the pure Rust image engine in `crates/engines/src/image/pdf.rs`. The page was exactly the image size at one point per pixel, and the only param was `page = "fit"`, which the engine accepted and otherwise meant nothing.
- Upright 8 bit gray or RGB JPEGs are embedded as they are (`DCTDecode`); every other image is stored losslessly with Flate and alpha flattened onto white. The new options must not change that.
- The option schema from S-117 is in the tree. A block with fields needs `kind = "form"` because `kind = "none"` allows no fields (ADR 0012 decision 6). The engine parses its own validated values from `task.params`, so a hand written TOML or a future CLI `--set` reaches it the same way.
- A4 is 595.276 by 841.89 points and Letter 612 by 792 points, both portrait. One point is 1/72 inch and one inch is 25.4 mm, so one millimetre is 72/25.4 points.
- The engine never enlarges an image past its own dpi size: `fit` gives the image size plus the margin, and A4 or Letter scale the image down into the margin inset box at most to its natural size.
- `orientation = "auto"` chooses the fixed page orientation from the image: wider than tall is landscape, otherwise portrait. On `fit` the page follows the image, so orientation has no effect there.
- The legacy `page = "fit"` param stays for older action files and for the existing `image_pdf.rs` test that rejects `page = "a4"`; `page_size` is the new field and the new name.
- The seven acceptance tests already exist in `crates/engines/tests/s124_image_pdf_options.rs` in this block, written from the same contract; the ACs below map to them one to one.

## Acceptance criteria
- AC1 (action contract): `actions/images.toml` for `image.to-pdf` has `[action.options]` with `kind = "form"` and exactly four fields, in this order:
  - `page_size`, type `enum`, `maps_to = "page_size"`, default `"fit"`, values `fit`, `a4`, `letter` with Arabic and English labels.
  - `orientation`, type `enum`, `maps_to = "orientation"`, default `"auto"`, values `auto`, `portrait`, `landscape` with Arabic and English labels.
  - `margin_mm`, type `float`, `maps_to = "margin_mm"`, default `0.0`, `min = 0.0`, `max = 50.0`, `unit = "mm"`, Arabic and English label.
  - `dpi`, type `int`, `maps_to = "dpi"`, default `72`, `min = 72`, `max = 600`, Arabic and English label.
  The action keeps `output = { mode = "combined", ext = "pdf" }` and `[action.params]` with `page = "fit"`, `page_size = "fit"`, `orientation = "auto"`, `margin_mm = 0.0` (a float) and `dpi = 72`. `OptionsSpec::validate` passes and every field and choice label is non empty in both languages. No other action in the file changes.
- AC2 (fit default): with no params, or with `page = "fit"`, `page_size = "fit"`, `orientation = "auto"`, `margin_mm = 0.0` and `dpi = 72` together, a 64 by 48 PNG gives one page with `MediaBox [0 0 64 48]` and the placement matrix `[64 0 0 48 0 0]`; a 48 by 64 image gives `[0 0 48 64]`; two images give two pages; the output starts with `%PDF-` and loads.
- AC3 (fixed page size and margin): `page_size = "a4"`, `orientation = "portrait"` and `margin_mm = 10.0` on a 64 by 48 PNG gives `MediaBox [0 0 595.276 841.89]` within 0.01, the image is not enlarged (drawn 64 by 48 points), the image is centred, and every edge stays at least 28.3465 points (10 mm) inside the page box. `letter` uses 612 by 792 points the same way.
- AC4 (orientation): with `page_size = "a4"`, `orientation = "auto"` gives landscape 841.89 by 595.276 for a wide image and portrait 595.276 by 841.89 for a tall image; forced `landscape` and `portrait` override the image shape; `letter` with `auto` on a wide image gives 792 by 612. On `fit` the orientation is accepted and has no effect on the page box.
- AC5 (dpi): `page_size = "fit"` and `dpi = 144` on a 64 by 48 PNG gives `MediaBox [0 0 32 24]` and placement `[32 0 0 24 0 0]`; the default `dpi = 72` keeps one point per pixel. `dpi` is a TOML integer only, from 72 to 600.
- AC6 (JPEG passthrough preserved): a 40 by 20 JPEG with `page_size = "a4"`, `orientation = "landscape"` and `margin_mm = 5.0` is embedded with `/DCTDecode` and the original JPEG bytes appear in the PDF unchanged (no re-encode), the `MediaBox` is landscape A4 within 0.01, and the document loads.
- AC7 (bad params, no output): each of these is `EngineError::BadParam`, with the field name in `name`, a non empty reason and no output file left behind: `page_size = "a3"`, `page_size = "A4"`, `page_size = 5`, `orientation = "sideways"`, `orientation = "Landscape"`, `margin_mm = 60.0`, `margin_mm = 60` (integer, out of range), `margin_mm = -1.0`, `dpi = 71`, `dpi = 601` and `dpi = 144.0` (a float where a whole number is required). The legacy `page` param is still accepted only as `"fit"`; any other value is `BadParam` named `page`, as the existing `image_pdf.rs` test expects.
- AC8 (scope and gates): the change is limited to `crates/engines/src/image/pdf.rs`, `actions/images.toml` and the test file; the other `image_pdf.rs` tests and the rest of `crates/engines/tests` pass unedited, including `three_images_make_three_pages`, `rotated_jpeg_and_alpha_are_handled`, `grayscale_jpeg_is_embedded_as_is`, `bad_page_mode_is_rejected`, `bad_input_in_the_middle_names_that_file` and `cancel_between_pages`. Progress starts at 0, never decreases and ends at 1.0; a cancelled token still leaves no output. `cargo fmt --check`, `cargo clippy -p kawaro-engines --all-targets -D warnings` and `cargo test -p kawaro-engines` pass. No new dependency and no new `Cargo.lock` package (pdf-writer, image and miniz_oxide are already there), no network, MSRV 1.85. No change to core, UI, daemon, CLI, MCP, docs, `CHANGELOG.md` or Cargo files in this block; the input file is never modified and the only output is the one PDF.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | `action_declares_the_four_pdf_options` reads `actions/images.toml` through `ActionDef`: params, `kind`, field count and order, types, defaults, ranges, unit, choices and bilingual labels | unit |
| AC2 | `fit_keeps_one_point_per_pixel_by_default` | unit |
| AC3 | `a4_portrait_keeps_the_image_inside_the_margin` | unit |
| AC4 | `orientation_auto_and_forced_swap_the_page` | unit |
| AC5 | `dpi_144_halves_the_physical_image` | unit |
| AC6 | `jpeg_bytes_survive_a4_page_sizing` | unit |
| AC7 | `bad_page_options_are_bad_params` plus the existing `image_pdf.rs::bad_page_mode_is_rejected` for the legacy `page` guard | unit |
| AC8 | The existing image engine tests unedited, then `cargo fmt`, clippy and `cargo test -p kawaro-engines` | unit, gate |

## Out of scope
A grid or N-up layout, several images on one page, a page range for the images, per-image options, custom page sizes (A3, Legal, or a size the user types), a background colour, crop or straighten before embedding, using the image's own EXIF or pHYs resolution instead of `dpi`, page numbers or stamps. The hive form and the readout (S-119, still a draft): the options are in the action and reachable through the resolved params of S-120 and MCP, not through a new UI. Any change to the PDF page operations (S-123), the core option schema, the MCP tool list, docs, `CHANGELOG.md`, `docs/architecture.md` or `docs/engines.md`.

## Review log
- 2026-11-04, planner: written from the Lead contract and the P0 images-to-PDF item in `docs/research/pdf-feature-benchmark.md` section 8 and `docs/research/image-editing-and-preview.md` section 7.2. Decisions recorded: (1) `page_size` replaces the old `page` param but `page = "fit"` stays accepted so older action files and the existing rejection test keep working; (2) `fit` is the image at its dpi size plus a symmetric margin, and A4 or Letter never enlarge it past that size; (3) `orientation` only turns fixed A4 and Letter pages, `fit` follows the image, and there is no cross-field hiding in this block; (4) the seven tests exist in `crates/engines/tests/s124_image_pdf_options.rs` in this block and the ACs map to them one to one. The S-117 AC10 annotation (45 at S-117 time, 48 after S-123, test now `builtin_actions_carry_options_and_all_48_parse`) is appended to that story's log in the same change.

- 2026-11-04, implementer: crates/engines/src/image/pdf.rs reads page_size, orientation, margin_mm and dpi; actions/images.toml gains the four options with bilingual labels; crates/engines/tests/s124_image_pdf_options.rs has 7 tests; the existing image_pdf tests stay unedited.
- 2026-11-04, tester: crates/core/tests/registry.rs image.to-pdf row extended to the five exact params; cargo test -p kawaro-core is 769 passed and core, engines, mcp, cli and daemon suites are green.
- 2026-11-04, lead: docs/engines.md image to-pdf bullet rewritten, CHANGELOG bilingual line added.
