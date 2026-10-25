# S-063 Website redesign: light only, sharp hex language, big type, scroll driven demo
Status: planned

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
