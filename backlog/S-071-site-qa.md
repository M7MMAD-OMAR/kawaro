# S-071 Site audit after the rename and the hero film
Status: in review

Not a Rust change. Files: `site/build.py`, `site/css/docs.css`, `site/css/pages.css`, the generated `site/` output, `tests/site/`, `.gitignore`. Audit run 2026-10-26 against production (https://kawaro.sbarah.com) and the local source on branch `story/s071`.

## Why
The site was renamed to Kawaro and the Arabic home page now plays the hero film (S-068). Nobody had yet checked the live site end to end for leftovers of the old name, headers, dead links, layout at the four breakpoints, the film itself and the brand assets.

## Scope
Read only checks: every URL in the sitemap, `llms.txt`, `llms-full.txt`, `robots.txt`, `security.txt`, both feeds and the Markdown twins; response headers; link crawl; screenshots in Arabic and English at 1440, 1024, 768 and 390 px; console and CSP; OCR of the UI snapshots; the brand assets. Fixes only in site sources. `crates/` is read only here.

## Acceptance criteria
- AC1: no old product name in any production body, title, meta, JSON-LD or alt text; the only hits for `sbar` are the support link to the owner's own brand. Tracked text has no old name or old host.
- AC2: every `/docs/NAME/` page links to its Markdown twin at a URL that resolves (no 404 from a crawl of same origin links).
- AC3: an unknown URL is answered with status 404 and the site's own page (`404.html`, root absolute URLs, `noindex`), not an empty body.
- AC4: no horizontal overflow or clipped card on the four widths, in both languages; the download slabs stay inside the viewport at 768 px.
- AC5: the edit row under each docs page (edit, report an issue, Markdown) has targets of at least 56 px.
- AC6: the eight security headers, `media-src 'self'`, `autoplay=(self)` are served on pages, media and CSS.
- AC7: the film plays, and with reduced motion it pauses and shows controls; focus rings show on light and dark surfaces.
- AC8: no Python bytecode is tracked.

## Decisions
- D1: the prose links that are shorter than 56 px (inside a sentence) stay as they are. They are inline text, not stand alone targets.
- D2: the Cloudflare insights beacon blocked by the CSP is not fixed in code; the owner switches it off in the dashboard.
- D3: the stale UI snapshots that show the old name are in `crates/`, so they are only reported (see findings).

## Findings
| id | severity | where | result |
|---|---|---|---|
| F1 | high | production unknown URL: 404 with a zero byte body, because `wrangler.jsonc` asks for `404-page` and no `404.html` was published | fixed: `site/404.html` built by `not_found()` |
| F2 | high | all 18 docs pages: the Markdown link was relative to the page folder (`/docs/start/start.md`, 404) | fixed: `../NAME.md` |
| F3 | medium | `/en/download/` at 768 px: the Linux slab overflowed the viewport by 4 px, its right edge was cut | fixed: grid tracks `minmax(0, 1fr)` |
| F4 | low | docs edit row links were 27 px high on 390 px | fixed in `docs.css` |
| F5 | low | tracked `__pycache__/*.pyc` (two files) dirtied by every test run | fixed: untracked, ignored |
| F6 | medium | `crates/ui/tests/snapshots/settings-{en,ar}-{light,dark}.png` show the old name as the default folder name | open, `crates/` is out of scope |
| F7 | info | the Cloudflare insights beacon is blocked by the CSP on every page (one console error per page) | open by decision D2 |
| F8 | info | in the film, the last scene puts the full stop of `GPLv3.` next to the bullet; cosmetic, inside the video | open |
| F9 | info | inline prose links shorter than 56 px on 390 px (privacy, releases, windows, docs) | open by decision D1 |

## Review log
- 2026-10-26, builder: audit and fixes as above. `bash scripts/build-site.sh --no-search`, new `tests/site/test_s071_static.py` (AC2, AC3, AC8). Evidence screenshots in `/home/sbarah/.hermes/artifacts/qa/`.
- 2026-10-26, builder: `tests/site/run_all.sh` in the heavy scope: test_static 35 OK (skipped 4), test_links 10 OK, test_contrast 2 (skipped 2), test_s063_static 26 OK, test_s064_static 18 OK, test_s068_static 5 OK, test_s071_static 5 OK, test_browser 20 OK, test_s064_browser 4 OK, test_s068_browser 2 OK. test_s063_browser had one failure in the full run (`forced_dark_renders_same_pixels`, Arabic, the hero film frame differs between two shots) and passed alone: 19 OK (skipped 11). Known flake caused by the playing film (S-068), not changed here.
