# S-067 Hero film: the product film replaces the demo window on the Arabic home page
Status: in review

Not a Rust change. Files: `site/build.py`, `site/css/film.css`, `site/app.js`, `site/media/`, `scripts/site-headers.py`, `scripts/site/check-deploy.sh`, tests under `tests/site/`. Follows S-065.

## Why (owner, 2026-10-26)
The CSS demo window in the hero feels artificial. The product film made for the first LinkedIn post (a 30 s Arabic film, rebuilt with the Kawaro name and mark) feels real, so the hero shows it instead.

## Decisions
- D1: the film is the Arabic page only. It carries Arabic text, so the English page keeps the CSS demo until an English cut exists.
- D2: the hero uses the first 22.4 s (hook, drag and drop demo, the name, formats, trust). The closing "coming soon" scene is cut, and the small badge says only the name.
- D3: silent, `autoplay muted loop playsinline`, 1280x720, WebM (VP9) first and MP4 (H.264) second, plus a WebP poster taken from the demo scene. About 1 MB per file.
- D4: reduced motion: the script pauses the film and shows controls; it also pauses while off screen.
- D5: security headers change on purpose. `media-src 'self'` joins the CSP, `Permissions-Policy` allows `autoplay=(self)` (with `autoplay=()` even a muted film is blocked), `/media/*` is cached 30 days, and `check-deploy.sh` accepts `mp4` and `webm`. The pinned lists in `tests/site/test_s059_deploy.py` and `tests/site/test_static.py` are updated by the manager, and ADR 0007 and `docs/site-security*.md` follow.

## Acceptance criteria
- AC1: `site/index.html` has one `video.film-video` with WebM and MP4 sources and a poster; `site/en/index.html` still has the demo and no video.
- AC2: the film loads and plays in Chromium with no console error and no CSP violation, at 1440 and 390 px wide, with no overflow.
- AC3: `scripts/site/check-deploy.sh --build` passes with the media files in the upload set.
- AC4: design lint is clean (tokens only, no shadow, no radius, no gradient).

## Review log
- 2026-10-26, manager: built and checked in Chromium (desktop and 390 px), video plays from `hero.webm`.
