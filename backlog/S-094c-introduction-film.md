# S-094c Introduction film: faithful, sharp, bounded
Status: approved with tests (2026-10-29)

## Story
As a first time user, I want the Introduction film to show the real flow clearly on any window size, so that I understand how to use Kawaro in one viewing.

## Why
Owner complaint (2026-10-29): on a maximized Windows window the film is broken. Measured in `crates/ui/ui/welcome.slint` lines 261 to 275: a Rectangle of width 100% and height width*9/16 holds an Image of a 576x324 PNG (`crates/ui/resources/welcome/frame-*.png`, drawn in code by `welcome-frames.slint`). At about 1800 px the frame is upscaled about 3x (blurry) and the box is about 1000 px tall, overflows the window and is cut by the taskbar, leaving a half drawn scene (a dragged thumbnail, an empty area, the hive already faded). The old film is rejected as a defect, not a style choice. The card size rules are S-094 AC9; this story is the content and sharpness.

## Dependency
After S-094.

## Acceptance criteria
- AC1 (sharp): frames are rendered at scale 3 of the largest displayed size (`film-max-width` 560 px gives 1680x945 frames), or the scene is drawn live as vectors from one shared `DemoScene` component. Rendered pixel size equals displayed size times the window scale factor, +/- 0 px, at scale 1, 2 and 3.
- AC2 (story): the film shows 4 steps in order: select a file in a file manager, the hive appears at the pointer, drop on a cell, the result file appears beside the original. The hive is visible (opacity 1) in every frame of steps 2 and 3; no frame shows a dragged thumbnail without its source, or an empty area where a step says something appears. Each step has a caption from `kawaro_core::welcome` in Arabic and English.
- AC3 (controls): a poster frame (step 4, result visible) shows before play; a Play/Pause button with accessible label toggles the loop; under the system reduced motion setting the film shows the poster as a static image with the 4 captions as a list and never auto plays.
- AC4 (no overflow): at 880x640, 1920x1080 and 3840x2160 the card is inside the window and its height <= the visible content area (shared test with S-094 AC9).
- AC5 (review): the owner reviews the film on real Linux and Windows captures.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Render the card at scale 1, 2, 3 in the testing backend; the image source pixel size >= displayed size x scale | ui, snapshot |
| AC2 | Table over all frames: step index, hive opacity (expect 1 for steps 2 and 3), count of visible scene elements per step equals the expected list | unit (scene model) |
| AC3 | Initial state shows the poster; press Play: frame index advances after 1 tick; press Pause: it stays; reduced motion on: no timer runs after 3000 ms and the 4 captions are present | ui |
| AC4 | Geometry at the three sizes, no overflow | ui |
| AC5 | Capture reviewed by the owner | manual |

## Test hooks (pinned by `crates/ui/tests/s094c_film.rs`)
All `#[doc(hidden)] pub`, on `WelcomeWindow`, geometry in logical px as in S-094.
- `kawaro_ui::film::frames() -> Vec<FilmFrame>` with `FilmFrame { step: usize (0..4), hive_opacity: f32, elements: Vec<String> }`; element names `file-manager`, `source-file`, `pointer`, `hive`, `thumbnail`, `result-file`. Required per step: 0 file-manager, source-file (no hive, thumbnail, result); 1 plus hive, pointer; 2 file-manager, source-file, hive, thumbnail (no result); 3 file-manager, source-file, result-file (no thumbnail). A thumbnail implies its source-file. Hive opacity is exactly 1.0 in steps 1 and 2 (the second and third steps).
- `kawaro_ui::film::poster_index()`; window: `film_frame()`, `film_step()`, `film_is_poster()`, `film_playing()`, `film_timer_running()`, `film_tick()` (what the timer does, only while playing), `press_film_play_pause()`, `film_button_label()` (also in `visible_labels()`), `set_reduced_motion(bool)`, `film_card_rect()`, `set_scale_factor(f32)`, `film_render_pixels() -> (u32, u32)`.
- Raster frames, if any remain in `crates/ui/resources/welcome/frame-*.png`, must be at least 1680x945.
- Not covered by these tests: the rendered look (snapshots, light and dark, 4K), which stays with the reviewer screenshots and AC5.

## Out of scope
- Sound, video files, new languages.

## Review log
- 2026-10-29 tester: `crates/ui/tests/s094c_film.rs`, 7 tests (AC1 x2, AC2 x2, AC3 x2, AC4). Red run: does not compile, `error[E0432]: unresolved import kawaro_ui::film` and 36 `E0599` for the missing window hooks (`film_frame`, `film_step`, `film_is_poster`, `film_playing`, `film_timer_running`, `film_tick`, `press_film_play_pause`, `film_button_label`, `set_reduced_motion`, `film_card_rect`, `set_scale_factor`, `film_render_pixels`). The raster size test would also fail on the current 576x324 frames once it compiles. Runs used `systemd-run --scope` with 6G, 200% CPU, jobs 2, nice 19. AC5 is manual.
