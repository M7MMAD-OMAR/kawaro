# S-056 Build the public website
الحالة: مراجعة

## Story
As the maintainer, I want a bilingual public website with a honeycomb theme, so that visitors understand SbarHive at a glance, download the beta safely and are told honestly what is and is not verified.

## Acceptance criteria
- AC1: `python3 site/build.py` writes eight pages (home, download, windows, privacy in Arabic at the root and English under `site/en/`), each with a working language switch to its twin and `dir="rtl"` for Arabic.
- AC2: The home hero is an interactive hive: five file kinds, cells that match `actions/*.toml`, the full action name in the centre cell on hover or focus, a click that shows tray progress and the result file next to the untouched original; it works by keyboard.
- AC3: Sections are joined by hex edges and side cells; every animation stops or is instant under `prefers-reduced-motion`, and the content is visible without JavaScript.
- AC4: Every colour, length and duration comes from `design/tokens.json` through `site/tokens.css`; no shadow, gradient, one sided border, em dash or en dash in `site/`.
- AC5: Content is honest: beta `0.4.0-beta.1`, Windows unsigned with the SmartScreen steps, not run on a live Windows machine, no promise of any store or package manager, no telemetry, `SHA256SUMS` steps for Linux and Windows, download links point to GitHub releases.
- AC6: Screenshots checked in a private browser pane at desktop and phone width, in light and dark, Arabic and English, with no horizontal scroll.

## Out of scope
- Publishing, deploying, tagging a release, a custom domain.
- Real screen recordings of the app (the clips on the site are illustrations and say so).

## Review log
- Not a Rust change, so the agent chain is not used; an Arabic copy review and an accessibility review were run by separate agents.
