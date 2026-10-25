# S-065 Home page: one hero message, no try-it demo, scroll choreography
Status: done

Not a Rust change. Files: `site/build.py`, `site/css/hero.css`, new `site/css/motion.css`, `site/css/bento.css`, `site/style.css`, `site/app.js` (comment), removed `site/js/sim.js` and `site/css/sim.css`, tests under `tests/site/`. Follows S-064.

## Why (owner, 2026-10-26)
- The try-it section adds nothing: remove it completely.
- The page needs real scroll motion: how each block enters, leaves and is composed.
- The hero headline ("convert with the right click") is not what a visitor looks for. The two things that matter are converting and staying safe on the machine. Remove the "coming soon" badge above the headline and open up the line spacing: the two lines read crowded.
- Is the hero noisy? Where does the eye go?

## Hero analysis
The S-064 hero had nine things competing: the status pill, a headline with a small animated hive inside it, an amber highlight, the lead, the amber Download button, a "try it" link, three outlined chips, the note, and the looping demo with three captions. Four of them were amber or animated (pill dot, headline hive, highlight, button), so the eye had no single first stop, and the chips repeated what the headline should say. The headline answered "how" (right click) instead of "what and why" (convert, safely, on my machine).

Decision: one message, one action, one proof. The headline carries the promise (حوّل ملفاتك بأمان على جهازك / Convert files safely, locally) with the highlight on the place the file stays; the lead says how, in two sentences; one Download button; the demo shows it. Removed: the pill, the headline hive, the chips and the try-it link. Reading order: headline, then the button, then the demo.

## Acceptance criteria
- AC1: no `#sim`, no `sim.js`, no `sim.css`, no link to `#sim`.
- AC2: the hero holds only the `h1`, `.hero-side` (lead, one primary button, the note) and the demo; no eyebrow, no chips. The Arabic headline lines have a visible ink gap (line height 1.55, no negative margin).
- AC3: scroll choreography in `css/motion.css`: a reading meter, ribbons that swing into their X, headings unmasked upward, bento tiles that arrive one after another and sink back on exit, family words from the reading start with their codes from the other side, download slabs from opposite sides, closing tiles that rise and grow, the footer name rising. The hero headline drifts up and the demo settles back on exit. Scroll and view timelines only, gated by `@supports` and `prefers-reduced-motion: no-preference`; nothing fades.
- AC4: S-064 rules hold (tokens only, radius 0, no shadows, gradients or one sided borders, targets 56 px, text 20 px on desktop).

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | `test_s064_static.py` `test_AC7_no_try_it_demo` | unit |
| AC2, AC4 | `test_s064_browser.py` (no overflow or overlap, CTA in the first screen at 375), S-063 type and target checks | UI |
| AC3 | `test_s064_static.py` `test_AC5_view_timeline_gated_and_never_fades` | unit |

## Review log
- Frames at ten scroll positions (1440, Arabic) checked by eye: no blank or broken state.
- Superseded: the S-061 sim browser tests (`AC7Sim`, `AC8SimAccess`).

## Round 2: full responsive audit (owner: "there are problems, check everything")
Audited 8 pages x 8 widths (320 to 1920) x 2 languages for page overflow, boxes leaving the viewport, clipped text, small targets and overlapping blocks, plus full page shots read by eye. Fixed:
- Demo: the hive opened over the file being converted and over the result slot, with half covered letters peeking out. It now opens in the free end of the window; the other files' labels recede while it is open.
- Formats: long family words (Documents, مستندات) ran into their codes from 1024 px up. From 1024 px the list is two shared subgrid columns; below that the codes sit under the word.
- Docs on phones: the menu was a pile of eleven boxes and pushed the search box off screen. It is one swipeable row that brings the current page into view.
- Releases and download pages: footer links and GitHub links were 27 to 32 px tall; now 56 px targets.
- Troubleshooting (Arabic): the quoted English warning broke its line direction; reworded without quote marks.
- Hero: demo step captions shortened to one line; the page title follows the new message.
Remaining by design: inline links inside prose sentences keep text height (WCAG inline exception).
