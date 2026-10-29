# S-094c Introduction film: faithful, sharp, bounded
Status: draft (2026-10-29)

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

## Out of scope
- Sound, video files, new languages.

## Review log
-
