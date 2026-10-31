# S-072 Launch assets from the real product and the rename announcement text
Status: draft

## Story
As the maintainer, I want a small launch kit built only from the real product (logo, real screenshots, README and site hero, one announcement text in English and Arabic), so that I can post it myself with nothing in it that the app, the repo or a measurement does not back. Non-blocking for every release (`docs/PLAN.md`). The site already has a hero, an Open Graph image and a hero video; this story audits and completes them, it does not redesign the site.

## Acceptance criteria
Agent part (no accounts, no posting):
- AC1 (manifest): `docs/launch/assets.md` lists every launch asset in a table with columns file, source (a path in the repo or a test script that produced it), language (en, ar, none) and theme. 100% of listed files exist, and 100% of rows name a source that exists. Allowed sources: `design/brand/*`, `docs/screenshots/**`, `docs/diagrams/**`, `site/og.png`, `site/media/hero-*`. 0 stock images, 0 mock-ups drawn by hand.
- AC2 (logo): the kit uses only `design/brand/kawaro-cells.svg`, `kawaro-cells-dark.svg`, `wordmark.svg`, `wordmark-dark.svg` and `png/kawaro-*.png`. Their SHA-256 in the manifest equals the file in `design/brand/`. The README and `README.ar.md` name state the same mark as `design/brand/README.md` and ADR 0008 (saguaro cactus from hive cells; the single cell is the app icon). Today the README "The name" section describes a "domed beehive"; that text is corrected, and the string "domed" is absent from both READMEs.
- AC3 (screenshots): the kit has at least 4 real screenshots, 2 English and 2 Arabic, 1 light and 1 dark per language, each copied unchanged (same SHA-256) from `docs/screenshots/**` and each at least 880 px wide. The manifest names the test script and story that produced each. Arabic ones show RTL layout. 0 files edited after capture, 0 text added over them.
- AC4 (README and hero parity): `README.md` and `README.ar.md` each carry the language switch line, the same number of H2 headings (diff 0), the same list of download rows, and the same platform table rows with the same tested or not state. The README current release line equals the version in `Cargo.toml` (today it says 0.4.7, the release is 0.5.0). The site hero `<h1>` and lead in `site/index.html` and `site/en/index.html` say the same 3 facts: convert, locally, result next to the original. Each hero image or video has a non-empty `alt` or `aria-label`, and the `og:image` file is 1200 x 630.
- AC5 (truthful claims): every sentence of the announcement and hero that states a platform, a format or a number maps to a row in a claims table inside `docs/launch/assets.md` (claim, evidence path). Claims limited to what `README.md` Platform support says: X11 reads "tested in a virtual X server", Windows reads "tested in a virtual machine", macOS reads "not supported yet", WinGet reads "waiting for review". 0 numbers without an evidence path. 0 of these words: testimonial, "trusted by", "users love", star or download counts, ratings, "fastest", "best". The only timing figure allowed is the 150 ms budget from `CLAUDE.md`, stated as a budget, not as a result, unless a measurement file is cited.
- AC6 (announcement text): `docs/launch/announcement.md` and `announcement.ar.md` each hold 3 parts: release note (5 to 8 lines, from `docs/release-notes-0.5.0.md` facts only), the rename note (one paragraph: the old name from ADR 0008, the new name, the mark did not change, the meaning of «كوّارة»), and a short post of at most 280 characters per language. Both start with the language switch line. Each includes the known gaps line (unsigned Windows build with SmartScreen warning, X11 checked only in a private session). Arabic uses Western digits, no guillemets.
- AC7 (no personal info and no dashes): across `docs/launch/`, the READMEs and the kit files, 0 matches for a home path (`/home/`), an email address, a machine name, a phone number, or a personal name other than the public repo owner handle already in the README links. `scripts/design-lint.sh` reports 0 long dashes. Every screenshot is checked by eye once for a visible username or path, recorded in the review log.
- AC8 (owner part, listed, not done by the agent): `docs/launch/owner-steps.md` lists the steps only the maintainer can do, each with the text file to paste: posting the announcement on chosen channels, creating accounts, the GitHub social preview upload, and the AUR and COPR accounts. The file contains 0 claims that a post was made. This story is done when AC1 to AC7 pass; AC8 items stay open and are not a gate.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Script parses the manifest table; asserts each file path and source path exists, count of rows equals count of files in `docs/launch/`, no source outside the allowed list | static |
| AC2 | Script compares the SHA-256 of each listed brand file with the manifest; greps both READMEs for "saguaro" present (name section) and "domed" absent | static |
| AC3 | Script asserts at least 2 en and 2 ar rows, both themes, hashes equal the source in `docs/screenshots/`, width from the PNG header at least 880 | static |
| AC4 | Script counts H2 in both READMEs, compares table rows, reads the version from `Cargo.toml` and finds it in the README release line; reads PNG header of `site/og.png` for 1200 x 630; checks `alt` or `aria-label` on hero media in both HTML files | static |
| AC5 | Script extracts numbers and platform names from the announcement and hero text and asserts each appears in the claims table with an existing evidence path; greps the banned word list (0 hits) and for any digit followed by `ms` outside the 150 ms budget wording | static |
| AC6 | Script asserts both files exist, 3 headings each, release note line count 5 to 8, post length at most 280 characters (counted by code points), switch line first, known gaps text present, no Arabic-Indic digits and no `«` or `»` | static |
| AC7 | Grep for `/home/`, `@`, long dashes (U+2013, U+2014) over the touched files; run `scripts/design-lint.sh`; the review log records the by eye screenshot check with the date | static |
| AC8 | Check `owner-steps.md` exists, names at least the 4 steps above, and contains no sentence in the past tense about posting | static |

Test file: `tests/docs/test_launch_assets.sh` (bash and python3 -I, no network, runs in the fast loop).

## Out of scope
- Any posting, account creation, paid promotion, or contact with press or communities (owner, AC8).
- New videos, new screenshots of states no test produces, new site sections, or a site redesign (site track S-056 to S-071).
- Code signing, store submissions, WinGet moderation, AUR and COPR (see `docs/distribution-status.md`).
- Any statistic, review, quote, comparison with other converters, or benchmark.

## Review log
- 2026-10-31, planner: replaced the placeholder with criteria. Read `CLAUDE.md`, `docs/PLAN.md`, `README.md`, `docs/release-notes-0.5.0.md`, `design/brand/README.md`, `site/en/index.html` and the screenshot folders. Found 2 truthfulness gaps to fix in AC2 and AC4: the README name section says "domed beehive" while the logo is the saguaro of cells, and the README release line says 0.4.7 while the release is 0.5.0. Size: about one working day (text, a manifest, one script). Needs the manager's approval; no code written.
