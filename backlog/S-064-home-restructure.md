# S-064 Home page restructure: one idea per section, a live hero demo, motion from the scroll
Status: done

Not a Rust change. Files: `site/build.py`, `site/style.css`, `site/css/` (new `demo.css`, `marquee.css`, `bento.css`, `formats.css`, `closing.css`; rewritten `hero.css`, `header.css`; restyled `sim.css`; removed `scene.css`, `art.css`), `site/js/scene.js` removed, `site/assets/art-*.webp` removed, `site/fonts/IBMPlexSansArabic-Bold.woff2` added, `design/tokens.json` (`site` block only), `scripts/tokens-to-css.py`, tests under `tests/site/`. Supersedes S-063 AC6 (the pinned scroll scene) and AC9 (generated art); every other S-063 rule stays (light only, radius 0, tokens only, no shadows, gradients or one sided borders, content visible by default).

## Why (owner, 2026-10-26)
The owner judged the S-063 home page "very bad, complicated and trivial" and asked for a full restructure of design, content and sections: modern, clear, with motion. Root causes found in the S-063 page:
- The pinned scene took about 5 screens of scroll and left most of them empty (a blank cream area in every full page capture).
- Nine stacked sections each repeated the same shape: a hexagon mark, a heading, one small row of items.
- The hero art was a static cluster; nothing showed the product working until the visitor scrolled far.
- Type stopped at 96 px semibold, so nothing on the page had the weight of the references the owner sent.

## Story
As a visitor, I want to see the product working in the first screen and then read one clear idea per section, so that I understand what Kawaro does and how to get it without scrolling through empty space.

## Acceptance criteria
- AC1 (structure): the home page has, in order: hero, conversion band, facts grid (`#why`), try it (`#sim`), formats (`#formats`), download (`#download`), open source and support (`#open`). Each section has exactly one `h2`, except the hero (one `h1`). No `#scene`, no `scene.js`, no `scene.css`.
- AC2 (hero demo): the hero holds a CSS only demo (`.demo`) of one conversion: a folder window with photo.png, a cursor, a hive of seven cells opening by the file (no cell for the file's own format), and photo.webp landing in a dashed slot beside the original, plus three step captions. It loops on `--duration-loop`; x is mirrored for right to left; with reduced motion, or before any animation, it shows the end state (both files, no hive, no cursor).
- AC3 (display type): the hero `h1` uses the bold weight (700, `IBMPlexSansArabic-Bold.woff2`, OFL like the other weights) at `--type-display` (176 px) on wide screens and at least `--type-display-sm` (52 px) at 375 px, and fits the viewport without horizontal overflow in both languages.
- AC4 (formats from the source): family names, action counts and codes in `#formats` come from `actions/*.toml` at build time; the heading total equals the number of `[[action]]` entries and each family shows one code per action, so the badge equals the chips; Arabic counts follow number agreement (3 to 10 plural).
- AC5 (motion): one entrance curve (`--ease-out`) and one steady curve (`--ease-steady`, loops only); every duration from tokens. Entrances are scroll driven (`animation-timeline: view()`), move blocks but never fade them, and apply only under `@supports` and `prefers-reduced-motion: no-preference`. Loops (band, hero demo, headline hive) stop under reduced motion. Hover effects are gated by `(hover: hover)`.
- AC6 (header and footer): the header is sticky and holds a primary Download button on every page; the footer is an ink slab ending in the product name in amber.
- AC7 (kept): the try it demo (`#sim`, `js/sim.js`) keeps its markup contract and behaviour, so the S-061 sim tests still apply; one support link in the header; light only; computed radius 0; no shadows, gradients or one sided borders; no hand written colour, length or duration in site CSS; no long dashes.
- AC8 (review): a design reviewer and an Arabic copy reviewer, each in a fresh context, review the page from screenshots and strings; blocking findings are fixed and recorded here.
- AC9 (finish): both languages at 1440 and 375, `scripts/design-lint.sh`, reproducible build, one `CHANGELOG.md` line in English and Arabic.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Parse built home pages: section ids in order, one h1, one h2 per section, no scene files | unit |
| AC2 | Demo markup present (7 cells, result tile, 3 steps); demo.css has rtl keyframes and the reduced motion gate | unit |
| AC3 | Font file exists, `@font-face` 700, tokens `display` and `display-sm`; browser: no horizontal overflow at 375 | unit, UI |
| AC4 | Count `[[action]]` in actions/*.toml and compare to the heading and the per family badges; Arabic plural forms | unit |
| AC5 | CSS scan: curves limited to the two tokens, `animation-timeline` only inside `@supports`, no opacity 0 at rest | unit |
| AC6 | Header CTA on every built page; footer mark present | unit |
| AC7 | Existing S-061 and S-063 suites, minus the superseded scene and art tests | unit, UI |
| AC9 | design-lint, build twice and compare, CHANGELOG line | unit |

## Out of scope
- The inner pages keep their S-063 layout; they only pick up the new header, footer and type.

## Review log

### Round 1 (2026-10-26), fresh context reviewers
- Design reviewer: 6.5/10, FIX on most sections. Applied: one release truth (the eyebrow says coming soon until `RELEASE_PUBLISHED=1`), hive opens beside photo.png and offers no PNG cell, dashed placeholder for the result slot, stray amber hexagon behind the demo removed, chips outlined, crossing ribbons at 4 degrees, section headings at `--fs-h1` without the repeated hexagon mark, family words at display size, the formats badge now equals the chips (presets shown), support tile clipped like the open tile, Support in the header as a plain link, no monospace in the demos, body text 20 px everywhere on desktop. Not applied, with reason: a stat tile (the formats heading already carries the one real number), preopening the try-it hive (would change the S-061 sim contract), full words on the cells (the codes are the app's own cell codes from `actions/*.toml`).
- Arabic reviewer: 20 lines changed (headline now بالزر الأيمن, subject added to the facts heading, لا ترى إلا, one verb for saving, حدّد for files and اختر for actions, نسخة ويندوز غير موقّعة, ZIP بلا تثبيت, readable aria labels). Not applied: رمادي, it is the app's own cell code.
- Found by the new browser test: in Arabic the demo hive sat on the wrong side because `direction: ltr` on a cell flips its own `inset-inline-start`; fixed with `unicode-bidi: isolate`.

### Result
`tests/site/run_heavy.sh`: every suite OK (superseded S-061 and S-063 checks skipped with a reason), `scripts/design-lint.sh` clean.
