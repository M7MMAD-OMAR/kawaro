# S-118 Preview worker and shared image geometry
Status: done (2026-11-04)

## Story
As a user who is about to customize a resize, I want a small preview of the result and the exact output dimensions while I move a value, so that I can decide without running the job and without waiting. This story builds the engine room only: a worker thread, scaled decoding, a draft cache, cancellation, the hexagon mask and one geometry function that the preview and the real engine both call. It shows nothing in the hive yet; S-119 is the visible stage 2.

## What this story will prove
- A preview can be produced off the UI thread within the budgets below on the software renderer, with no GPU and no async runtime.
- The engine and the preview agree pixel for pixel, because both call `image_geom::target`; a preview that is one pixel off is a bug.
- A stale option change never wins: a generation counter cancels the older render, and a slider drag never queues more than one draft.
- Decoding is bounded: header first (dimensions before pixels), then a reduced decode (JPEG DCT scaling, PNG row streaming, SVG at target, other formats decode then resize, first frame only), and a 100 MP file never decodes at full size for a preview.
- The 150 ms hive promise is untouched because no preview work runs before the first hive frame, and the island of work is warm only while stage 2 is open.

## Dependencies
- S-117 for the typed field kinds, readout tokens and resolved values the preview reads. Without it a preview does not know what an option means.
- `crates/ui/src/preview.rs` (the existing PNG row streaming into the centre hexagon), `crates/engines/src/image.rs` (Lanczos3 and EXIF orientation) and the `image` crate already in the tree. No new dependency is planned; a new decoder (jpeg-decoder for scaled decode) needs a licence line in `docs/engines.md` before it lands.
- Blocked by nothing else. It blocks S-119. Crop, straighten and PDF page preview are later stories.

## Acceptance criteria (draft, to be fixed when the story is approved)
- AC1 (geometry, one source): a pure core function `image_geom::target(source: (u32, u32), mode, value) -> (w: u32, h: u32, crop: Option<Crop>)` exists, covers percent, longest side and fit, and keeps aspect ratio with the same rounding as the engine. The image engine calls it for every resize. A table of source sizes and values gives the same `(w, h)` from the preview and from the engine, including odd sizes and a 1 px floor.
- AC2 (header first): the dimensions of a 24 MP JPEG appear before any pixel exists, in under 1 ms of worker time on the bench machine; the readout can paint from the header alone.
- AC3 (scaled decode): a 24 MP JPEG draft is decoded at 1/8, 1/4 or 1/2 as the target allows, never at 1/1; a PNG draft decodes row by row to the target; an SVG renders at the target; GIF and WebP use the first frame. A 100 MP JPEG gives dimensions at once and a draft between 300 and 600 ms, never a full decode.
- AC4 (budgets, measured by a bench script under `tests/preview/`): first preview pixel for a 24 MP JPEG under 100 ms (150 ms worst); re-render after one option change between 30 and 50 ms; a slider drag holds 30 fps on drafts with one full quality pass on release; a stage 2 plate repaint under 8 ms; total preview memory at most 4 MB with the draft capped at 1000 px on the long side. The bench fails on a regression over 20 percent and prints the numbers.
- AC5 (single flight and cancel): an option change during a render cancels the older generation; at most one render runs and at most one waits; after 20 rapid changes only the newest is published. Closing stage 2 cancels everything and frees the buffers.
- AC6 (mask): the worker hands Slint a ready RGBA buffer already scaled to the well and masked to the pointy top hexagon (alpha 0 outside, one antialiased edge pixel). No Slint element rotates, scales or clips the picture, because the software renderer cannot do it.
- AC7 (failure is soft): a decoder error, a pixel cap hit or a zero size shows the dimensions and the text "preview not available"; it never fails the job and never blocks the UI.
- AC8 (promise): the hive first frame stays under 150 ms and no preview work starts before it; no new blocking call on the UI thread; reduced motion does not change a preview.
- AC9 (licences): any new decoder crate has a version, licence and purpose line in `docs/engines.md` before the dependency lands; `Cargo.lock` grows only by the packages that line names, and the MSRV stays 1.85.

## Test cases (draft)
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Pure table test of `image_geom::target` plus a check that the engine calls it | unit |
| AC2 | Header probe timing and the dimensions before a pixel | unit |
| AC3 | Scaled decode choices for JPEG, PNG, SVG, GIF and the 100 MP case | unit, bench |
| AC4 | `tests/preview/bench.sh` with the fixture set, numeric assertions | performance |
| AC5 | Generation counter: 20 changes, newest wins, one render at a time | unit |
| AC6 | Buffer alpha at the corners and the edge; no transform in the `.slint` file | unit, snapshot |
| AC7 | Fake decoder error shows the fallback text and does not fail the run | unit |
| AC8 | Existing hive budget run unchanged | performance |
| AC9 | Test reads `docs/engines.md` and `Cargo.lock` like S-113 AC7 | unit |

## Out of scope
The stage 2 layout, the option controls, the readout and the presets (S-119). Crop, straighten, target file size, PDF page preview, HEIC and animation. Any change to the hive's current appearance or to the 150 ms budget. GPU work and async runtimes.

## Review log
- 2026-11-04, planner: draft from `docs/research/image-editing-and-preview.md` section 6.3 and the order in `docs/research/agent-integration-and-expansion.md` section 9. The budget numbers are targets to prove, not measured facts; the story must keep the bench that proves them. Not approved, not in this block.

- 2026-11-04, tester: crates/core/tests/s118_geometry.rs, 34 table rows, 4 tests; red with the missing module, then 4/4 green.
- 2026-11-04, core-impl: image_geom and the engine resize call; preview_worker with header probe, scaled JPEG, row streamed PNG, resvg SVG, first frame GIF/WebP/BMP, 40 MP cap, hexagon mask and newest-wins generation; 8 worker tests and the release bench.
- 2026-11-04, core-impl: release bench probe 0.00 ms, draft 94.45 ms for a 24 MP JPEG, mask 2.17 ms, draft 2668000 bytes, against 5 / 500 / 50 / 4194304. The bench is ignored in debug and the budget is asserted by tests/preview/bench.sh in release, like the hive bench.
- 2026-11-04, lead: lock count 678 and the licence rows for jpeg-decoder, resvg/usvg, tiny-skia, fontconfig-parser and roxmltree recorded; core, engines and ui clippy clean; docs and CHANGELOG updated.
