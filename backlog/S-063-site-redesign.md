# S-063 Website redesign: light only, sharp hex language, big type, scroll driven demo
Status: done

Not a Rust change. Files: `site/build.py`, `site/style.css` (split into `site/css/`), `site/app.js` (split into `site/js/`), `design/tokens.json` (new `site` block only), `scripts/tokens-to-css.py`, tests under `tests/site/`. Supersedes the parts of S-061 and S-062 that fix dark mode, rounded surfaces and the boxed drop demo (S-061 AC7 and AC8 are replaced by AC6 and AC7 here). Sections ship one at a time; nothing half redesigned is pushed to `main`, because the site deploys from `main`.

## Why (root causes found on 2026-10-25)
- The page follows `prefers-color-scheme` and three sections (`how`, `download`, footer) are forced dark through `.inv`, so a light page flips to black blocks.
- `tokens.css` is the app's token file. The site inherits its 6, 10 and 14 px radii and its 12, 14 and 16 px type sizes, which are right for a desktop popup and wrong for a landing page.
- Every section is the same stack: heading, text, rounded card. No section has its own component or its own idea.
- Motion reuses the app's 120 ms and 80 ms hive durations. Content is hidden until an observer fires, which leaves blank screens while scrolling.
- The demo is a small dashed box with five tiny file chips.

## Story
As a visitor, I want a bright, bold page whose shapes come from the honeycomb and whose demo plays as I scroll, so that I understand the product from the page itself.

## Acceptance criteria
- AC1 (light only): `<meta name="color-scheme">` is `light`; the built CSS has 0 `prefers-color-scheme`, 0 `.inv`, 0 `[data-scheme="dark"]` and 0 `*.dark.svg` references; with the browser forced to dark the page renders the same pixels as light.
- AC2 (site tokens): `design/tokens.json` gains a top level `site` block (light palette, radius 0, type scale with h1 at least 72 px on desktop, section spacing, motion durations). The app keys are unchanged and the crates build and test unchanged. `scripts/tokens-to-css.py` emits `site/tokens.css` from the `site` block only. No hand written colour, length or duration in site CSS.
- AC3 (sharp shapes): computed `border-radius` is 0 on every element of every page. Cut corners and hexagons are drawn with `clip-path` polygons or SVG, never with shadows. A bordered shape draws its edge as an inset polygon, because `clip-path` clips borders. No shadows, no gradients, no one sided borders.
- AC4 (big and clear): body text is at least 20 px on desktop and 18 px at 375 px; each section has one `h2` of at least 56 px on desktop and 36 px at 375 px; every button and link target is at least 56 px high; one idea per screen.
- AC5 (components): each section is built by a named helper in `build.py` (`hex_button`, `section_head`, `hex_plate`, `step_cell`, `format_cell`, `download_slab`), with its CSS in its own file under `site/css/components/`. No section styles another section's component.
- AC6 (scroll demo): the home page has one pinned scene (`position: sticky`, 100vh stage inside a tall section). Scroll progress drives, in order: a folder with files, one file selected, the hive opening ring by ring, a cell picked, the result file appearing next to the untouched original. It covers the full viewport, uses only `transform` and `opacity`, and runs one `requestAnimationFrame` loop fed by scroll only. x is mirrored for RTL.
- AC7 (demo access): a keyboard operable version of the demo stays reachable (a "try it yourself" control after the scene, using the existing `#sim` logic and its tests). With reduced motion or JS off the scene shows the final state as static numbered steps.
- AC8 (motion and feedback): one entrance curve and one duration scale from the site tokens; hover and focus change shape fill with an immediate, visible response; content is visible by default and animation only enhances it (0 blank screens at any scroll position, checked by screenshots every 100 px). The AC4 rule of S-061 (no pointer listeners outside the demo) stays.
- AC9 (visuals): hero and section art that is generated (a GPT image model through fal) is stored as WebP under `site/assets/`, has no text inside the image, sits on the light palette, and has Arabic and English alt text. Each file is under 200 KB.
- AC10 (finish): both languages (ar, en), desktop and 375 px, reduced motion on and off; S-059 AC3 checks, `scripts/design-lint.sh`, a dash search over the diff, reproducible build, `CHANGELOG.md` line in English and Arabic.

## Sections, in build order
0 foundation (tokens, light only, helpers) then header, hero, scroll demo, how, privacy, formats, download, open source, support, footer, then the inner pages (download, windows, privacy, docs, releases).

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Grep built CSS and HTML; Playwright with `colorScheme: dark`, compare screenshots with light | unit, UI |
| AC2 | Parse tokens; run `cargo test -p ui` unchanged; grep site CSS for raw values | unit |
| AC3 | Playwright walks every element and asserts computed radius 0; grep for shadow, gradient, one sided border | UI |
| AC4 | Computed font sizes and target heights at 1280 and 375 px | UI |
| AC5 | Assert each helper exists and each component file is referenced once | unit |
| AC6 | Playwright scrolls the scene and asserts the state at 0, 25, 50, 75 and 100 percent, RTL mirror, computed properties limited to transform and opacity | UI |
| AC7 | Tab through the control, run the existing sim cases, reduced motion shows static steps | UI |
| AC8 | Screenshot every 100 px of scroll and assert no blank viewport; check durations come from tokens | UI |
| AC9 | File type, size, alt text present in both languages | unit |
| AC10 | Existing S-059 and design-lint checks, build twice and `diff -r` | unit |

## Review log

### Tester run (before implementation)

Command: `bash tests/site/run_s063_heavy.sh` (systemd-run scope, 6 GB, 200% CPU, CARGO_BUILD_JOBS=1). Result: 45 tests, 45 not passing (static 26 failures; browser 14 failures and 5 errors, the errors are timeouts waiting for the missing `#scene`).

Skipped as superseded by S-063: test_contrast.py `test_AC10_all_pairs` and `test_AC10_sbarah_section_pairs` (dark scheme pairs, `.inv`), test_static.py `test_AC5_support_section_structure` (dark `<picture>` source).

```
FAIL: test_AC10_build_reproducible_with_split_sources (__main__.AC10Finish.test_AC10_build_reproducible_with_split_sources)
AssertionError: False is not true : site/css/components missing in the build
FAIL: test_AC10_changelog_line (__main__.AC10Finish.test_AC10_changelog_line)
AssertionError: 0 != 1 : exactly one CHANGELOG line for S-063
FAIL: test_AC10_design_lint_covers_split_css (__main__.AC10Finish.test_AC10_design_lint_covers_split_css)
AssertionError: False is not true : site/css/components missing
FAIL: test_AC10_no_long_dashes (__main__.AC10Finish.test_AC10_no_long_dashes)
AssertionError: False is not true : site/css/ missing
FAIL: test_AC1_meta_color_scheme_light_everywhere (__main__.AC1LightOnly.test_AC1_meta_color_scheme_light_everywhere)
AssertionError: 'light dark' != 'light'
FAIL: test_AC1_no_dark_in_css_html_js (__main__.AC1LightOnly.test_AC1_no_dark_in_css_html_js)
AssertionError: 'prefers-color-scheme' unexpectedly found in '\n\n@font-face {\n  font-family: "IBM Plex Sans Arabic";\n  src: url("fonts/IBMPlexSansArabic-Regular.woff2") format("woff2");\n  font-weight: 400;\n  font-di
FAIL: test_AC1_tokens_css_has_no_dark_block (__main__.AC1LightOnly.test_AC1_tokens_css_has_no_dark_block)
AssertionError: 'prefers-color-scheme' unexpectedly found in '/* Generated by scripts/tokens-to-css.py from design/tokens.json. Do not edit. */\n\n:root {\n  color-scheme: light dark;\n  --space-1: 4px;\n  --space-2: 8px
FAIL: test_AC2_app_keys_unchanged (__main__.AC2SiteTokens.test_AC2_app_keys_unchanged)
AssertionError: 'site' not found in {'$description': 'Single source of truth for the Kawaro design system. No shadows, no gradients, no one-sided borders. Elevation is expressed with surface tones only.', 'color': {'li
FAIL: test_AC2_generator_reads_site_block_only (__main__.AC2SiteTokens.test_AC2_generator_reads_site_block_only)
AssertionError: 'site' not found in {'$description': 'Single source of truth for the Kawaro design system. No shadows, no gradients, no one-sided borders. Elevation is expressed with surface tones only.', 'color': {'li
FAIL: test_AC2_no_raw_values_in_site_css (__main__.AC2SiteTokens.test_AC2_no_raw_values_in_site_css)
AssertionError: False is not true : no site/css/components/*.css
FAIL: test_AC2_site_block_shape (__main__.AC2SiteTokens.test_AC2_site_block_shape)
AssertionError: None is not an instance of <class 'dict'> : design/tokens.json needs a top level 'site' block
FAIL: test_AC3_clip_path_polygons_used (__main__.AC3SharpShapes.test_AC3_clip_path_polygons_used)
AssertionError: Regex didn't match: 'clip-path\\s*:\\s*polygon\\(' not found in '\n\n@font-face {\n  font-family: "IBM Plex Sans Arabic";\n  src: url("fonts/IBMPlexSansArabic-Regular.woff2") format("woff2");\n  font-weig
FAIL: test_AC3_no_radius_shadow_gradient_in_css (__main__.AC3SharpShapes.test_AC3_no_radius_shadow_gradient_in_css)
AssertionError: Regex didn't match: '^0(px)?$' not found in 'var(--radius-md)' : .skip border-radius
FAIL: test_AC3_svg_has_no_rounding_or_filters (__main__.AC3SharpShapes.test_AC3_svg_has_no_rounding_or_filters)
AssertionError: False is not true : site/css/components missing
FAIL: test_AC5_component_css_files_exist_and_are_own (__main__.AC5Components.test_AC5_component_css_files_exist_and_are_own)
AssertionError: False is not true : site/css/components/hex-button.css
FAIL: test_AC5_each_file_linked_once_and_classes_used_on_home (__main__.AC5Components.test_AC5_each_file_linked_once_and_classes_used_on_home)
AssertionError: 0 != 1 : ar: hex-button.css linked 0 times
FAIL: test_AC5_helpers_exist (__main__.AC5Components.test_AC5_helpers_exist)
AssertionError: Regex didn't match: '^def hex_button\\(' not found in '#!/usr/bin/env python3\n"""Build the Kawaro site: every page in Arabic (site root) and English (site/en).\n\nStandard library only. Colours, length
FAIL: test_AC5_other_css_does_not_style_components (__main__.AC5Components.test_AC5_other_css_does_not_style_components)
AssertionError: False is not true : site/css/components missing
FAIL: test_AC6_one_raf_loop_scroll_only (__main__.AC6SceneStatic.test_AC6_one_raf_loop_scroll_only)
AssertionError: [] is not true : site/js/*scene*.js missing
FAIL: test_AC6_scene_css_animates_transform_and_opacity_only (__main__.AC6SceneStatic.test_AC6_scene_css_animates_transform_and_opacity_only)
AssertionError: 0 not greater than 3 : no scene rules in site CSS
FAIL: test_AC6_scene_markup (__main__.AC6SceneStatic.test_AC6_scene_markup)
AssertionError: 0 != 1 : ar
FAIL: test_AC8_durations_and_curve_from_tokens (__main__.AC8MotionStatic.test_AC8_durations_and_curve_from_tokens)
AssertionError: Regex matched: 'linear' matches 'cubic-bezier|\\bease\\b|ease-in|ease-out|\\blinear\\b|\\d+m?s\\b' in 'marquee calc( * 200) linear infinite' : .marquee .track animation: marquee calc(var(--unit) * 200) li
FAIL: test_AC8_no_pointer_listeners_outside_sim_in_all_js (__main__.AC8MotionStatic.test_AC8_no_pointer_listeners_outside_sim_in_all_js)
AssertionError: False is not true : site/js missing
FAIL: test_AC8_no_reveal_hiding_rules (__main__.AC8MotionStatic.test_AC8_no_reveal_hiding_rules)
AssertionError: False is not true : content hidden by default: from
FAIL: test_AC9_alt_text_both_languages (__main__.AC9Assets.test_AC9_alt_text_both_languages)
AssertionError: 0 not greater than or equal to 2 : site/assets needs the generated hero and section art as .webp
FAIL: test_AC9_webp_format_and_size (__main__.AC9Assets.test_AC9_webp_format_and_size)
AssertionError: 0 not greater than or equal to 2 : site/assets needs the generated hero and section art as .webp
ERROR: test_AC3_scene_and_hover_states_also_sharp (__main__.AC3Sharp.test_AC3_scene_and_hover_states_also_sharp)
ERROR: test_AC6_only_transform_and_opacity_change (__main__.AC6Scene.test_AC6_only_transform_and_opacity_change)
ERROR: test_AC6_rtl_mirror (__main__.AC6Scene.test_AC6_rtl_mirror)
ERROR: test_AC6_scene_content_per_state (__main__.AC6Scene.test_AC6_scene_content_per_state)
ERROR: test_AC6_states_at_0_25_50_75_100 (__main__.AC6Scene.test_AC6_states_at_0_25_50_75_100)
FAIL: test_AC1_forced_dark_computed_colours_equal (__main__.AC1Pixels.test_AC1_forced_dark_computed_colours_equal)
AssertionError: Lists differ: ['rgb(31, 26, 18)|rgba(0, 0, 0, 0)', 'rgb(31, 2[14535 chars] 0)'] != ['rgb(242, 236, 224)|rgba(0, 0, 0, 0)', 'rgb(24[14685 chars] 0)']
FAIL: test_AC1_forced_dark_renders_same_pixels (__main__.AC1Pixels.test_AC1_forced_dark_renders_same_pixels)
AssertionError: Lists differ: [b'\x[126 chars]dd\x07xSe\x1b\x06\xe0\xd3\xa4;\xabI\x9atO(S\x9[459347 chars]x82'] != [b'\x[126 chars]dd\x05\x80T\xe5\x1a\x06\xe03\xdd\xbd\xbdK\x83t[452853 chars]x82']
FAIL: test_AC3_every_element_every_page (__main__.AC3Sharp.test_AC3_every_element_every_page)
AssertionError: Lists differ: ['A.skip radius 10px', 'A.skip radius 10px[177 chars]0px'] != []
FAIL: test_AC4_targets_at_least_56_high (__main__.AC4Big.test_AC4_targets_at_least_56_high)
AssertionError: Lists differ: [[47, 'A.skip انتقل إلى المحت'], [44, 'A.b[629 chars]كة']] != []
FAIL: test_AC4_type_sizes (__main__.AC4Big.test_AC4_type_sizes)
AssertionError: 18.0 not greater than or equal to 20
FAIL: test_AC6_stage_is_pinned_and_covers_viewport (__main__.AC6Scene.test_AC6_stage_is_pinned_and_covers_viewport)
AssertionError: 0 != 1
FAIL: test_AC7_existing_sim_still_works_by_keyboard (__main__.AC7Access.test_AC7_existing_sim_still_works_by_keyboard)
AssertionError: 0 != 1 : sim stays behind the try-it control
FAIL: test_AC7_js_off_static_numbered_steps (__main__.AC7Access.test_AC7_js_off_static_numbered_steps)
AssertionError: 0 != 5
FAIL: test_AC7_reduced_motion_static_numbered_steps (__main__.AC7Access.test_AC7_reduced_motion_static_numbered_steps)
AssertionError: 0 != 5 : ar
FAIL: test_AC7_try_it_control_after_scene_keyboard_reachable (__main__.AC7Access.test_AC7_try_it_control_after_scene_keyboard_reachable)
AssertionError: 0 != 1 : ar
FAIL: test_AC8_content_visible_with_js_off (__main__.AC8Feedback.test_AC8_content_visible_with_js_off)
AssertionError: 0 != 1 : scene missing
FAIL: test_AC8_hover_and_focus_change_fill_at_once (__main__.AC8Feedback.test_AC8_hover_and_focus_change_fill_at_once)
AssertionError: 0 != 1 : hero uses the hex_button component
FAIL: test_AC8_no_blank_viewport_every_100px (__main__.AC8Feedback.test_AC8_no_blank_viewport_every_100px)
AssertionError: 0 != 1 : scene missing
FAIL: test_AC8_reduced_motion_zero_durations_in_scene (__main__.AC8Feedback.test_AC8_reduced_motion_zero_durations_in_scene)
AssertionError: 0 not greater than 5 : scene missing
```

### Implementer, phase B (2026-10-25)

- Art (AC9): `openai/gpt-image-2.5/flare/text-to-image` through fal, 3 generations (budget was 8), cost not reported by the tool. Hero art, privacy art and formats art are stored as `site/assets/art-*.webp` (10 KB, 9 KB, 9 KB), alt text per language, width and height set. Below the fold images load eagerly because the old S-061 test `test_AC7_zero_network_requests_after_load` fails on a lazy fetch that fires during the demo.
- The hero hive SVG was replaced by the hero art; the scene shows the hive.
- Fixes: section head h2 inherited a big top margin (the mark was its first sibling), privacy map labels moved out of the shapes, format cells no longer share edges, hover fills on plates, step cells, format cells, cards and slabs, inner pages use hex plates, step cells and cut corner cards.
- Results: `run_s063_heavy.sh` 26 static and 19 browser tests green; old suites green (test_static, test_links, test_browser; contrast skipped as superseded); `cargo test -p kawaro-ui` green; `cargo fmt --check` clean; `design-lint.sh` clean. One browser test errored once on a page load timeout under machine load and passed on rerun.

### Reviewer, fresh context (2026-10-25): REWORK

Gates: fmt clean, design-lint clean, `cargo test -p kawaro-ui` green, static suites green (S-063 26, S-061 35 + links 10), build reproducible (`diff -r` after two `build-site.sh --no-search` runs, tree clean), no long dashes in the diff (only vendored pagefind), no inline handlers, style attrs only `--name:number`. Browser suites: 1 test errored in each run with `Page.goto` 3000 ms timeout on `/en/` while the machine load was about 6; in the second run 5 old `test_browser.py` AC7Sim tests timed out the same way. Page load measured by hand is 0.3 to 0.6 s, so this is load flakiness, but the 3 s timeout in `tests/site/lib.py` is too tight and should be raised. No blank viewport at any 100 px step (1280 and 375, ar and en).

Visual defects, ranked:
1. Scene (site/css/scene.css, `.scene-stage`): at 1280 the folder fills only the left 45 percent, the hive is tiny (cell labels about 10 px) and the lower third and right half of the pinned 100vh stage are empty. At 375 the hive overlaps the folder and labels break mid word (RESIZ/E, SHRIN/K, ROTAT/E). Not "big components". The scene is one pinned section, not a simulation across the whole page.
2. Hero art (site/assets/art-hero.webp): a flat, fuzzy raster of hexagons in a cut corner box, no relation to the file-conversion idea, weak against the 72 px heading. Hero en 1280: the amber highlight on "right" overlaps the descenders of "file with a". The hero subtitle wraps to three short lines.
3. Privacy art (art-privacy.webp) has rounded corners (monitor, files, lock), against "always sharp corners". The old `support-sbarah-light.svg` card has rounded corners and a pink circle; `support-sbarah-dark.svg` is still tracked and unused.
4. "Try it here" (`#sim`) is still the small dotted box with tiny chips and tiny text, the weakest block on the page.
5. Formats section: hexagon rows overlap the heading area at some scroll positions and the lower hex row is clipped in the screenshots; the art strip above it repeats the same message.
6. ar releases: English changelog lines inside RTL lists show misordered punctuation (content, not new).

### Implementer, rework after the first REWORK (2026-10-25)

- Scene: full viewport stage. At 1280 by 800 the folder (two by two tiles, names 20 px and up) sits on the inline start, the 19 cell hive (cell labels 21 px, about 66 percent of the viewport height) beside it, a huge caption per state at the bottom and five progress hexagons next to the heading. Cells fly out of the original file ring by ring, the picked cell inverts to ink, the result slides out of the cell and lands next to the original on an amber tile with a halo. Under 960 the layout stacks (folder on top, hive below); under 600 only ring one shows so every label is a whole word. Only transform and opacity change; one rAF loop unchanged.
- Hero: the raster art is gone (art-hero.webp deleted). The hero picture is inline SVG drawn in build.py (`hero_art`): a honeycomb where an ink PNG cell turns into an amber WebP cell across a chevron, beside other format cells. The highlight is now inline-block with its own line height, so it no longer covers the line above; the subtitle wraps to two lines.
- Art: art-privacy.webp and art-formats.webp regenerated with fal (two generations, sharp angular geometry only, checked by eye: no rounded corner). The formats art now shows a page shape turning into a hexagon, so it no longer repeats the heading. Alt text updated in both languages.
- Try it: large block, 2 column sample tiles (text 20 px), big drop zone placed above the samples (so a drag stays inside one viewport), hexagon idle state, hive cells 115 px at 1280, filled result rows.
- Formats: art limited in width, more room above and below the hexagon rows, the PDF cell no longer repeats "PDF".
- ar releases: an English only changelog line is its own `dir="ltr"` item.
- Left as is: `support-sbarah-dark.svg` (an old test needs the file; unused on the page) and the rounded owner badge `support-sbarah-light.svg` (the owner's own artwork).
- Test change: `tests/site/lib.py` page timeout 3000 to 15000 ms (load flakiness only).

### Reviewer 2, fresh context (2026-10-25): REWORK

Gates: fmt clean, design-lint clean, S-063 static 26 and browser 19 green, old static suites green (contrast skipped as superseded), full run_all browser suites green (no timeouts with the 15 s goto), two `build-site.sh --no-search` builds identical (`diff -r`). The modified `tests/site/__pycache__/lib.cpython-314.pyc` is pre-existing (tracked since S-061, rewritten by every test run), not part of this story; it should be restored or untracked, not committed with S-063.

First REWORK defects: scene size and mobile overlap fixed (full stage at 1280, whole word labels at 375); hero art replaced by a sharp drawn SVG and the highlight no longer overlaps; privacy art regenerated; formats rows no longer crowd the heading; ar releases English lines are separate LTR items (partly, see 5).

Open defects:
1. Blocker, #sim at 375 (en and ar): the hive after a pick has tiny cells; SHRINK, RESIZE, ROTATE labels are clipped by their hexagons and the centre label breaks mid word ("imag/e", "Conv/ert/to"). Same family as the first REWORK defect 1, now in the demo. Open #sim, pick photo.png, scroll to the hive at 375.
2. Blocker, reduced motion (and by the same path JS off) at 1280: the picked WEBP cell is ink filled with ink text, so the label is an unreadable outline. Emulate prefers-reduced-motion and look at #scene.
3. Polish, reduced motion at 1280: step 5 caption wraps to four lines and touches the viewport edge.
4. Polish, formats art (art-formats.webp): hand drawn wobbly strokes and a faded, clipped panel edge; reads as a raster sketch next to the crisp hexagons. Idle #sim at 1280 has a large empty gap under the intro and a hive placeholder larger than the drop zone.
5. Polish, ar releases at 375: the footer line of links (verify file, Windows unsigned, summary) still breaks punctuation oddly.
6. Polish: stated weak spots confirmed: mobile scene hive labels are small (about 13 px), flying cells in state 3 stop between folder and hive rather than starting from the file; ghost hive is visible but faint (acceptable).

### Implementer, rework after Reviewer 2 (2026-10-25)

- Try it at 375: ring one only (first six cells, whole word codes), cell pitch about 106 px, cell and centre labels 18 px, centre label wraps at spaces only (`overflow-wrap: normal`). Idle block: the hive placeholder is the drop zone height, the stage no longer reserves hive height until a file is picked.
- Scene static end state (reduced motion and JS off): the picked WEBP cell is amber with ink text. The 3 by 2 caption grid keeps step 5 to two lines at 1280. Mobile name labels are 18 px.
- Flying cells: `js/scene.js` measures the original tile, the ghost cells and the result on load, fonts ready and resize (never on scroll) and writes `--sx`, `--sy` per cell and `--rx`, `--ry` on the result; cells now start at the original file tile and the result starts at the picked cell. Only transform and opacity change.
- ar releases: the link line is a list of separate items; the footer GPLv3 fragment is isolated with bdi.
- art-formats.webp: the fal output was wobbly, so it is now drawn by us (straight polygons, rendered to a lossless transparent WebP, 5 KB) instead of spending more generation calls.
- Gates: S-063 static 26 and browser 19 green, run_heavy green, design-lint clean, fmt clean, two builds identical.

### Reviewer 3, fresh context (2026-10-25): PASS

Gates: S-063 heavy run green (19 browser tests), `run_heavy.sh` green on rerun (one `Page.goto` timeout on `/en/` in the first run while the machine was loaded, not repeated), design-lint clean, fmt clean, two `build-site.sh --no-search` builds identical, no long dashes in the diff outside vendored pagefind, no .pyc in the new commits (the tracked `lib.cpython-314.pyc` from S-061 is only rewritten by test runs; restored).
Blockers verified with own screenshots: (1) #sim at 375 in en and ar after picking photo.png: ring one cells PDF, AVIF, GIF, JPG, WEBP, PNG whole, centre label breaks at the space only; (2) #scene with reduced motion and with JS off at 1280 (en, ar): picked WEBP cell amber with ink text, readable.
Polish: scene at 375 step 5, the "photo.webp" name is clipped by the tile edge and the tile touches the MOV tile; step 4 picked cell overlaps ring cells slightly; scene hive labels at 375 are small in step 3 flight. Formats art is crisp. ar releases footer at 375 reads fine. CHANGELOG already carries the S-063 line.
