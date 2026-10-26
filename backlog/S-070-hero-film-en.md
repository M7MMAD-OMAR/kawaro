# S-070 English hero film: the English home page plays an English cut of the product film
Status: done

Not a Rust change. Files: `site/build.py`, `site/media/hero-en.*`, tests under `tests/site/`. Follows S-068. The film sources are untracked and live in `video/intro-hero-en` (22.4 s hero cut) and `video/intro-en` (30 s full film), copied from the Arabic projects.

## Why
S-068 D1 kept the CSS demo on the English page because the film carried Arabic text. The English page should get the same real-looking film, in English, laid out left to right.

## Decisions
- D1: the English film is a translation and a mirror of the Arabic one, not a new edit: same scenes, same timings, same music and UI sounds. Every on screen string is English; the Arabic name is not shown (the large name is the Latin "Kawaro", so the small separate Latin wordmark of the Arabic film is merged into it).
- D2: layout is mirrored for left to right: text uses `left:`, the file manager sidebar is on the left, the hive, drag path, cursor, mark and the trust cells swap sides (x becomes 1920 minus x), the hex wipe sweeps from left to right, `dir` is `ltr`.
- D3: font: IBM Plex Sans (OFL) Regular and SemiBold, bundled in `assets/fonts` and declared by `@font-face`, so Latin text does not depend on a system font.
- D4: the site serves the same formats as S-068 D3: `hero-en.webm` (VP9) first, `hero-en.mp4` (H.264) second, `hero-en-poster.webp` from the demo scene, 1280x720, silent, each under 2 MiB. `hero_film(lang)` picks the files and the aria label by language.
- D5 (manager decisions on pinned tests): the English home no longer keeps the demo, so these tests change.
  - D5a: `test_s068_static.py::test_AC1_english_home_keeps_the_demo` becomes `test_AC1_english_home_has_its_own_film` (one video, no demo figure).
  - D5b: `test_s064_static.py::test_AC2_demo_markup` asserts that neither home page renders `figure.demo` and both render `figure.film`.
  - D5c: `test_s064_browser.py::test_AC2_reduced_motion_shows_the_end_state` loops over no languages (the demo is on no page); reduced motion for the film is `test_s068_browser.py` and `test_s070_browser.py`.
  - D5d: `test_s063_browser.py::test_AC8_no_blank_viewport...` checks `.film` on the English page instead of `.demo`.
  - D5e: `run_all.sh` runs the two new files.
- D6: `hero_demo` and `css/demo.css` stay (unused code removal is a follow up).

## Acceptance criteria
- AC1: `site/en/index.html` has one `video.film-video` with WebM then MP4 sources, `media/hero-en-poster.webp`, autoplay muted loop playsinline, and no demo figure.
- AC2: the English video has an English `aria-label` that says the video is silent.
- AC3: the Arabic page is unchanged and still uses `hero.*`.
- AC4: the three English media files exist, each under 2 MiB, the videos have no audio stream, 1280x720.
- AC5: in Chromium at 1440 and 390 px the English film plays from `hero-en.webm` with no console error and no overflow; with reduced motion it is paused and shows controls.
- AC6: the film shows no Arabic text, no long dash, Western digits, text is not clipped or overlapping in frames at 1.5, 5, 9, 12.9, 15.5, 19.5 and 22 s.
- AC7: `scripts/site/check-deploy.sh --build` and design lint pass.

## Test cases
- `tests/site/test_s070_static.py`: AC1 to AC4.
- `tests/site/test_s070_browser.py`: AC5.
- Manual: frame review of AC6 from a draft render and the final render.

## Review log
- 2026-10-26, builder: film translated and mirrored, rendered (hero cut 1080p at 30 fps, full film 1080p at 60 fps), encoded (mp4 984 KB, webm 1.1 MB, poster 51 KB). hyperframes check: 0 errors on both projects; reading time warnings only, same timings as the Arabic film. Frames reviewed at 1.5, 2.3, 5, 9, 12.9, 15.5, 19.5 and 21.8 s: no overlap or clipping; the strike line sits slightly below the middle of the x height. Chromium at 1440 and 390 px: plays from hero-en.webm, no console error, no overflow. Site tests: all pass except `test_s063_browser.py::test_AC1_forced_dark_renders_same_pixels`, which also fails on main (the playing film changes the pixels between runs); not changed here.

## Manager decision (2026-10-26, after review)

D6. `test_AC1_forced_dark_renders_same_pixels` compared screenshots that include the playing hero film, and the decoded film frame differs between two page loads, so it failed intermittently on both languages. The test now pauses and hides the film before the screenshots (`FREEZE_FILM` in `tests/site/test_s063_browser.py`). The assertion is unchanged: light and dark colour schemes must render the same pixels. Five consecutive runs pass.

### Reviewer (2026-10-26, independent, fresh context)

PASS on 824bb44. Frames compared with the Arabic film, layout mirrored, files within budget, pinned test changes legitimate. The one flaky test is fixed by D6. Full site suite after the rebase onto main: all pass. Report: ~/.hermes/checkpoint-runs/kawaro-1026b/review-s070.md
