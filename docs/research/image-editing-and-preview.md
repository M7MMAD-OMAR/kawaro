# Image editing, live preview and the option UI

Research note, not a decision. Author: image-researcher. Date 2026-11-04. Question: which image capabilities Kawaro should add, and how a customization step with a live preview fits a hexagonal hive drawn by the Slint software renderer. External claims link to Sources at the end; anything not verified by fetching a primary page is marked **unverified**. Effort sizes are guesses.

## What exists today (repo facts)

- Ops `convert`, `resize` (`scale` percent or `max_side`), `rotate`, `flip`, `grayscale`, `compress`, `strip`, `to-pdf`, with values baked into `actions/images.toml` (`quality = 85`, `max_side = 1920`, `page = "fit"`).
- `ActionDef.params` is an untyped `toml::Table` (`crates/core/src/action.rs`); `Request::Run` carries only an action id, so no user, script or agent can change one value on one run.
- Hive 393 by 362 px, 41 to 118 ms through the daemon, 28 MB idle with the software renderer (ADR 0001). `crates/ui/src/preview.rs` already decodes a cached PNG one row at a time and box filters it into the inscribed square of the centre hexagon: the seed of the preview.
- No crop, no straighten, no metadata view, no size readout, no per-run options.

## 1. What other tools expose

| Tool | Options it exposes | Take for Kawaro |
| --- | --- | --- |
| ImageMagick 7 | resize with `%`, `>` shrink only, `^` fill area, `@` pixel count, thumbnail, adaptive and interpolative resize; `-strip`, `-quality`, `-colorspace` | the geometry vocabulary for option help text |
| sharp / libvips | `fit`: cover, contain, fill, inside, outside; `position`; `withoutEnlargement`; kernel; shrink on load | `fit` names plus `withoutEnlargement` are a proven model |
| Squoosh | resize width, height, `fitMethod` stretch or contain, method (triangle, catrom, mitchell, lanczos3, hqx), premultiply, linearRGB, encoder quality; runs fully local; live size readout | best precedent: two results side by side, live bytes |
| XnConvert | 80 plus actions, saved presets, output naming, metadata editing, batch profiles | presets plus "remember last" is enough |
| GIMP | scale by pixel, percent or physical unit, chain link locks the ratio, interpolation choice, warns above 128 MiB | chain link for aspect lock, memory warning |
| img2pdf | lossless images to PDF, no re-encode, 500 to 700 bytes of container overhead | Kawaro already does the DCTDecode trick; add page size and margin |
| TinyPNG, iLoveIMG | one button compression, quality heuristics, some resize, no target size on the free page | one click must stay the primary path |
| macOS Preview, Windows Photos, Photopea, Canva, Figma export, ImageOptim | export sheets and scale presets | **unverified** here: the Apple guide returned an unsupported content type, the Figma page loaded but I could not confirm its preset text |

Nobody wins with more controls. They win with a good default, one obvious "smaller, same look" action, and a second layer for people who want it.

## 2. Capability plan

Route: `image` in process (pure Rust), `child` like ffmpeg and gs, `model` opt-in per ADR 0010. Licences and versions from the crates.io API on 2026-11-04 unless stated.

| Capability | Route | P | Effort | Licence / risk |
| --- | --- | --- | --- | --- |
| Resize: percent, longest side, exact, fit, fill with crop | image, core math | P0 | M | none, `image` 0.25.10 MIT OR Apache-2.0 |
| Crop with aspect presets and freeform | image | P0 | M | none |
| Images to PDF: page size (fit, A4, Letter), margin, dpi | pdf-writer 0.15.0 MIT OR Apache-2.0 | P0 | M | keeps the DCTDecode fast path |
| Straighten by angle; rotate and flip exist | image | P1 | S | resampling already available |
| Quality target ("under 400 KB") | image, bounded quality search | P1 | M | 3 to 5 encodes per file, must cancel |
| Metadata read, keep or strip | kamadak-exif 0.6.1 BSD-2-Clause, img-parts 0.4.2 MIT OR Apache-2.0 | P1 | M | today every re-encode strips |
| Pad, border, background fill, flatten alpha | image | P1 | S | none |
| PDF to images: dpi, page range | ghostscript child | P1 | S | exists at one dpi |
| PDF page size, N-up, booklet, watermark, sampled compress | ghostscript, qpdf class | P1 | L | separate research needed |
| PNG lossless optimization | oxipng 10.2.1 MIT | P1 | S | pure Rust, slow, run as a job |
| SVG input and render | resvg 0.49.0 Apache-2.0 OR MIT, tiny-skia 0.12.0 BSD-3-Clause | P1 | S | pure Rust, renders at target size |
| Image info panel: dimensions, ratio, dpi, colour, metadata | core, header only | P1 | S | instant, reuses the readout |
| ICC conversion | moxcms 0.9.1 BSD-3-Clause OR Apache-2.0 (in tree) | P2 | M | correctness work |
| Better JPEG encode | mozjpeg 0.10.13, licence IJG | P2 | M | C build, cmake and nasm |
| JXL input and output | jxl-oxide 0.12.6 MIT OR Apache-2.0 decode, libjxl 0.12 BSD-3-Clause through `jpegxl-rs` 0.16.0 which is GPL-3.0-or-later and a C build | P2 | L | cheapest new format to decode, expensive to encode |
| HEIC and HEIF input | libheif LGPL-3.0, C, `libheif-rs` MIT wrapper | P2 | L | opt-in component, bundled codec licences |
| Animated GIF and WebP | gif 0.14.2 MIT OR Apache-2.0, or gifski 1.34.0 AGPL-3.0-or-later | P2 | L | do not link gifski: child process or skip |
| Favicon multi size, sprite sheet, batch rename | image, core | P2 | M | none |
| Upscaling, background removal | model, Real-ESRGAN BSD-3-Clause | P2 | L | ADR 0010 path, weights check pending |
| OCR | Tesseract 5 Apache-2.0, child | P2 | M | already the plan |
| Watermark, compare two images | image, embedded font | P2 | S to M | cheap win |

`image` 0.25.10 cannot decode AVIF without dav1d and cannot decode HEIC or JXL at all (`docs/engines.md`), so JXL input is the cheapest genuinely new format and HEIC the most expensive.

## 3. Options must be declarative data

Keep `[action.params]` as the defaults, add an optional `[action.options]` block that describes what may change. One source of truth for hive, CLI and MCP; no option logic in the UI.

```toml
[action.params]            # unchanged: what a plain click runs
scale = 50

[action.options]
kind = "form"              # "none" (default) or "form"
readout = ["dimensions", "ratio", "percent", "output_name"]
remember = true            # per action values live in config, not here
suffix_template = "-{percent}"

[[action.options.field]]
id = "mode"                # UI only, never reaches the engine
type = "enum"              # percent, px, px_pair, enum, bool, int, float
label = { ar = "الطريقة", en = "Method" }
default = "percent"
values = [{ id = "percent", label = { ar = "بالنسبة", en = "By percent" } },
          { id = "longest_side", label = { ar = "بالجانب الأطول", en = "Longest side" } }]

[[action.options.field]]
id = "percent"
type = "int"
unit = "percent"
min = 1
max = 1000
step = 5
default = 50
label = { ar = "النسبة", en = "Scale" }
visible_when = { field = "mode", equals = "percent" }
maps_to = "scale"          # merged over [action.params] before the engine runs

[[action.options.field]]
id = "longest_side"
type = "px"
min = 16
max = 20000
step = 16
default = 1920
label = { ar = "الجانب الأطول", en = "Longest side" }
visible_when = { field = "mode", equals = "longest_side" }
maps_to = "max_side"
no_enlarge = true          # the readout must say so when the source is smaller
```

Rules that make it safe to share:

1. Closed type set: the UI renders one control per type, the CLI parses by type, MCP emits the same type as JSON Schema. An unknown type is a startup error, never a free text field.
2. Validation lives in `core`: range, step, enum membership, cross field rules, and merge order (defaults, remembered values, the explicit request). Engines keep their own validation.
3. Wire change is additive: `Request::Run { action, files, params: Option<toml::Table> }` with `#[serde(default)]`. The daemon merges and rejects unknown keys as `BadParam`.
4. CLI: `kawaro run image.resize --set percent=25 photo.jpg`; `kawaro actions --id image.resize --schema` prints JSON Schema.
5. MCP: one tool per action family, an `options` object generated from the same schema, and the result echoes resolved values and resulting dimensions so the agent can state them before writing.
6. Naming: `suffix_template` renders from resolved values and goes through the existing conflict free naming in `crates/core/src/output.rs`.
7. Presets are data: `[[action.options.preset]] id = "web" values = { percent = 25 }` gives two or three one tap chips (Web, Share, Print) above the fields.
8. `remember` stays in config keyed by action id, never in the TOML, so an action file update is not shadowed by a stale value.

## 4. Preview math, shared with the engine

Let `(W, H)` be the source size after EXIF orientation, `P` the well size in device pixels, targets integers.

| Mode | Target | Note |
| --- | --- | --- |
| percent `p` | `(max(1, round(W p / 100)), max(1, round(H p / 100)))` | rounding half up |
| longest side `L` | `f = min(1, L / max(W, H))`, then the same two lines | never enlarges |
| exact `(w, h)` with fit | `k = min(w / W, h / H)` | as a resize the canvas follows, so fit equals exact; show the real target, not the typed numbers |
| fill with crop | `k = max(w / W, h / H)`, crop offsets `(W - w / k) / 2`, `(H - h / k) / 2` | the crop rectangle is the preview overlay |
| crop rectangle | output is the rectangle in source pixels | ratio from the rectangle |
| dpi target | `px = round(inches dpi)` | show px and the physical size |

Readout rules: `w x h px`; ratio as `gcd(w, h)` reduced, decimal with two places only when the reduced form is ugly ("1.78:1"); percent change against the source; megabytes labelled "about", estimates only. Arabic writes the unit after the value, digits stay Latin, and the number sits in a bidi isolate so `1920 × 1280` is not reordered.

Hard requirement: preview and engine call one function. Move the math into core, for example `image_geom::target(source, mode, value) -> (u32, u32, Option<Crop>)`, integer rounding, unit tested, and let `crates/engines/src/image.rs` call it instead of building its own `resize_exact` inputs. A preview one pixel off the file is a bug report waiting to happen.

In the well: fit the target rectangle into a `P x P` square with the same `fit` as thumbnails (`crates/ui/src/preview.rs`), never enlarge, centre it. For crop or fill, draw the crop rectangle as a 1 px `border` line and dim the outside with `glass-2` only where a blur exists, otherwise a solid `surface-2`. No gradient, no shadow, no one sided border.

## 5. Preview pipeline and budgets

All off the UI thread: (1) probe the header only, under 1 ms, publish dimensions before any pixel exists; (2) decode a draft at reduced resolution, row by row where the format allows; (3) cache the draft and never decode twice for one option change; (4) on change debounce 80 ms, resample the draft to the well, mask the hexagon, hand the buffer to Slint, cancel older work by generation counter (single flight, newest wins); (5) on drag release, one high quality pass with the engine kernel so pixels match the file.

| Format | Scaled decode | Plan |
| --- | --- | --- |
| JPEG | yes: `jpeg_decoder::Decoder::scale` gives 1/8, 1/4, 1/2, 1 | add `jpeg-decoder` 0.3.2 MIT OR Apache-2.0 as a direct dependency; `image` 0.25 exposes no such API that I could verify |
| PNG | row streaming plus box filter, as `preview.rs` does | interlaced needs a full decode, cap by pixel count |
| WebP | none found | full decode then resize, cap 50 MP, first frame (matches the engine) |
| TIFF, GIF | row or strip reads; first frame | box filter rows, decode first frame only |
| SVG | render at target size, no raster source | resvg 0.49.0, cheapest case |
| PDF | page 1 at a dpi computed from the page box, child process | reuse the ghostscript `to-images` path |
| AVIF, HEIC input | not available today | kind icon and dimensions, no picture |

Budgets, targets and estimates rather than measurements, each testable with fixtures of 1, 12, 24 and 100 MP JPEG plus 8 MP PNG, animated GIF, SVG and a 20 page PDF:

| Stage | Target | How |
| --- | --- | --- |
| Hive first frame | under 150 ms, unchanged | no preview work before paint |
| Stage 2 shell after the customize click | 120 ms | shell only, same 120 ms growth as the hive |
| First preview pixel, 24 MP JPEG (6000 by 4000) | under 100 ms, 150 ms worst | 1/8 decode gives 750 by 500 (0.375 MP), one resample to the well |
| Re-render on an option change | 30 to 50 ms | resample from the cached draft, no decode |
| Slider drag | 30 fps draft, full quality on release | smaller intermediate buffer, skip frames |
| Idle frame paint | under 8 ms | dirty region only, 16.7 ms at 60 fps leaves headroom |
| Preview memory | 4 MB total | draft capped at 1000 px long side (1000 x 667 x 4 = 2.7 MB) plus one 228 px well buffer |
| 100 MP JPEG | draft in 300 to 600 ms, dimensions instant | estimate; entropy decoding cannot be skipped, so 1/8 and never 1/1 |

Cancellation and failure: one worker thread (the tree has no async runtime), `mpsc` plus an atomic generation, a byte cap checked before allocation, a pixel cap that refuses above 1 MP instead of allocating, and a plain "preview not available" line with the dimensions when a decoder fails. A failed preview must never fail the job.

## 6. What the Slint software renderer can and cannot do

Slint is pinned at 1.18.1 (`crates/ui/Cargo.toml`), software renderer, no GPU.

Can, from the docs: render without a GPU, partial rendering with a dirty region cache, line by line rendering if memory is tight, paths, text, images. The dirty region matters: when only the picture property changes, only that rectangle is rasterised again, which is exactly the preview case.

Cannot, per the renderer table: rotations and scaling of elements, `drop-shadow-*`, `border-radius` with `clip: true`, text stroking, and the docs still say "text rendering currently limited to western scripts". Consequences: never rotate or scale the preview with an element transform, rotate and scale in the worker and hand Slint a ready buffer; mask the hexagon in the worker (alpha 0 outside, 1 px antialiased edge) because clipping to a hexagon path is unavailable. The "western scripts" line conflicts with the shipped Arabic hive, so it likely describes an older or no_std text path (**unverified**); test it with Arabic labels, mixed Arabic and Latin in one line, at 100 and 150 percent scale.

60 fps reality: the software renderer fills pixels on the CPU. The hive is 393 by 362 (142k px), a stage 2 plate of 660 by 420 is 277k px, and a hexagon well of circumradius 180 has an inscribed square of 228 px (52k px, derived here as `2 sqrt(3) r / (1 + sqrt(3))`). Repainting only the well at 60 fps is 3.1 Mpx/s; the whole plate is 16.6 Mpx/s. Both are small numbers for a desktop CPU, so the cost is layout and text relayout, not the blit. Keep numbers short, update the readout at most 20 times a second, do not animate plate colours, animate only position and scale tokens, and prefer a 30 fps draft over a 60 fps one.

## 7. A customization stage that fits the hive

Stage 1 is unchanged: the hive appears, cells are drawn, one click runs the defaults. The 150 ms and one click promises stay.

Three deliberate doors, and no extra click for everyone else: hovering or focusing a customizable cell shows the dimension readout in the centre cell ("1920 x 1280, 3:2") so the user can decide without opening anything; a second concentric hexagon outline around that cell (full 2 px `text-primary` stroke, no fill, no shadow) marks it customizable, clicking the cell body runs the default while clicking the ring, or Shift+Enter, opens stage 2; a settings switch "always ask before running actions with options" flips the default for people who want that.

Stage 2, as a layout: the same window grows from 393 by 362 to about 660 by 420 around its centre, so it opens where the user is looking and no new window can steal focus; growth uses the hive open motion (scale 0.92 to 1, 120 ms, ease out), zero under reduced motion. The hive plate stays `surface-1`; the stage 2 plate is `surface-3` inset by `space-4` with a full 1 px `border`: that is the entire elevation story. On the left (right in RTL) the picture well: a pointy top hexagon of circumradius 180, full 1 px `border`, `surface-0` interior, the preview masked to the hexagon, the target rectangle inside, and 2 px full square crop handles when the crop tool is active; under it one line with dimensions, ratio, percent change and the output file name. On the right (left in RTL) the option column, one row per field: label, control, value with unit, using the vendored Material 3 slider, segmented buttons, checkbox and text field; slider track `surface-2` with a `border`, active part `accent`, handle `surface-3` with a 2 px `border`, since the library shadows are already removed. Preset chips (Web, Share, Print) sit above the fields as hexagon chips. Footer: "Run" (filled `accent`, `accent-ink` text) and "Reset" (surface with `border`), plus a "Run with defaults" text button. Escape returns to stage 1 with nothing changed, click outside cancels.

Keyboard and RTL: Tab and Shift+Tab walk the fields, arrows change the focused value, Enter runs, Escape goes back; focus is a 2 px `text-primary` outline, never colour alone; every control's accessible label includes its value and unit, because the readout is the point. Mirror the layout and the slider direction for Arabic, keep digits, SVG and format codes Latin; Slint shapes Arabic but does not mirror layouts, so reverse the order by hand as the settings window already does. With several files selected the well previews the first and the readout adds "and 4 more, same settings"; per file options are out of scope, they turn the hive into a form editor.

Rejected on purpose: a separate editor window (focus stealing, a second design system, the 150 ms budget), a sidebar that pushes the hive off centre, tabs, stacked plus and minus buttons, and any control that needs a shadow to read as raised.

## 8. Patterns worth copying, and what to skip

| Pattern | Source | Use |
| --- | --- | --- |
| Defaults first, advanced behind a disclosure | Squoosh, XnConvert | one click runs, the ring opens the form |
| Live size readout next to the picture | Squoosh | dimensions, ratio, percent in the well footer |
| Aspect chain link | GIMP | one lock control on crop and exact size rows |
| Fit vocabulary cover, contain, fill, inside, outside | sharp | option names in the schema |
| Presets that survive a batch | XnConvert | chips plus remembered values |
| Warn before a huge allocation | GIMP 128 MiB | the pixel cap message |
| Target size instead of quality | no verified free tool exposes it | bounded quality search, result labelled an estimate |
| Skip node graphs, layer stacks, undo trees | XnConvert, Photopea | the hive is a one step tool |

## 9. How to measure, before building

1. Bench test (a `spikes/preview` crate is cheaper for a first look) printing decode, resample and mask times for the fixture set, median of 7 runs, with the same 6G and nice rules as the other heavy scripts.
2. A Slint frame test with `i-slint-backend-testing` (already a dev dependency) that sets the picture property N times and reads back the rendered region, proving the dirty rectangle claim instead of assuming it.
3. A budget script asserting section 5, failing CI on regression, in the style of the existing drop budget script.
4. One real Wayland session and one X11 session, because the software renderer fill rate is the only thing that differs.

## 10. Conflicts with the current plan, and risks

- `docs/engines.md` says users cannot change parameters and `Request::Run` cannot carry them: both need a story before UI work.
- `image.to-pdf` is fixed at 1 pt per pixel with no page size or margin, fine for screenshots and wrong for print. Page size and dpi are the first PDF options users will ask for.
- The preview needs a JPEG scaled decode that `image` 0.25 does not expose, so a new direct dependency (`jpeg-decoder`) is likely: permissive, small, but a dependency change that belongs in `docs/engines.md`.
- gifski is AGPL-3.0-or-later, the only clear licence hazard here. Do not link it; a child process like ffmpeg keeps the boundary, and that legal reading is **unverified**.
- libheif is LGPL-3.0 and its build may pull codecs under other licences, and shipping it would break the light install rule, so it stays opt-in.
- Long jobs stay in the tray, so the preview must be optional at every step: a user who never opens the form must never pay for a decode.
- Risk: a 100 MP file with the form open holds a draft, so the cap and the "preview not available" path are guard rails, not nice to have.
- Risk: schemas drift if UI, CLI and MCP each parse TOML. Only `core` reads the schema; the rest consumes the parsed form.

## 11. Recommended next actions

1. S-117, options as data: typed `[action.options]` in `core`, merge and validation, additive `Run.params`, JSON Schema export, tests. No UI. Unblocks MCP and the CLI at once.
2. S-118, preview worker and bench: probe, scaled decode, draft cache, generation cancel, hexagon mask, the shared dimension function, and a headless harness printing the numbers.
3. S-119, stage 2 for resize only, with readout and preset chips, then re-measure the 150 ms hive promise and the frame times. Crop comes only after that passes.
4. A small PDF page geometry story (page size, margin, dpi) after S-117, because it reuses the same schema and has the highest value per line of code.

## Sources

Every page fetched or checked on 2026-11-04.

- Slint backends and renderers, software renderer features and limitations: https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/ (2026-11-04)
- Slint Rust API, SoftwareRenderer, partial rendering and dirty regions: https://docs.slint.dev/latest/docs/rust/slint/platform/software_renderer/struct.SoftwareRenderer (2026-11-04)
- jpeg-decoder 0.3.2, Decoder::scale with 1/8, 1/4, 1/2, 1: https://docs.rs/jpeg-decoder/latest/jpeg_decoder/struct.Decoder.html (2026-11-04)
- sharp resize, cover, contain, fill, inside, outside, position, withoutEnlargement: https://sharp.pixelplumbing.com/api-resize (2026-11-04)
- sharp performance benchmark, libvips 8.18.2, JPEG resize task: https://sharp.pixelplumbing.com/performance (2026-11-04)
- ImageMagick examples, resizing vocabulary: https://usage.imagemagick.org/resize/ (2026-11-04)
- ImageMagick command line options: https://imagemagick.org/command-line-options/ (2026-11-04)
- Squoosh resize options model, width, height, fitMethod, methods, premultiply, linearRGB: https://raw.githubusercontent.com/GoogleChromeLabs/squoosh/dev/src/features/processors/resize/shared/meta.ts (2026-11-04)
- Squoosh README, compression runs locally: https://raw.githubusercontent.com/GoogleChromeLabs/squoosh/dev/README.md (2026-11-04)
- XnConvert, 80 plus actions and presets: https://www.xnview.com/en/xnconvert/ (2026-11-04)
- GIMP 3.0 Scale Image, aspect chain, 128 MiB ceiling: https://docs.gimp.org/3.0/en/gimp-image-scale.html (2026-11-04)
- img2pdf README, lossless direct embed, 500 to 700 byte overhead: https://raw.githubusercontent.com/josch/img2pdf/main/README.md (2026-11-04)
- TinyPNG: https://tinypng.com/ (2026-11-04)
- iLoveIMG compress image: https://www.iloveimg.com/compress-image (2026-11-04)
- crates.io API metadata for every crate licence and version above, for example gifski: https://crates.io/api/v1/crates/gifski (2026-11-04)
- libheif COPYING, LGPL-3.0, wrappers MIT: https://raw.githubusercontent.com/strukturag/libheif/master/COPYING (2026-11-04)
- libjxl LICENSE, BSD-3-Clause: https://raw.githubusercontent.com/libjxl/libjxl/main/LICENSE (2026-11-04)
- Tesseract LICENSE, Apache-2.0: https://raw.githubusercontent.com/tesseract-ocr/tesseract/main/LICENSE (2026-11-04)
- Real-ESRGAN LICENSE, BSD-3-Clause: https://raw.githubusercontent.com/xinntao/Real-ESRGAN/master/LICENSE (2026-11-04)
- Figma Learn, export from Figma, page loaded, scale presets not confirmed: https://help.figma.com/hc/en-us/articles/360040028114-Export-from-Figma (2026-11-04)
- Apple Preview guide, resize, rotate or flip an image, fetch failed with an unsupported content type: https://support.apple.com/guide/preview/resize-rotate-or-flip-an-image-prvw1262/mac (2026-11-04)
