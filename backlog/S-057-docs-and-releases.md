# S-057 Docs and releases on the website
الحالة: منتهية (منشورة على الموقع، تُبنى بـ scripts/build-site.sh وتنشر عبر S-059؛ الاختبارات في tests/site)

## Story
As a visitor or an AI agent, I want short guides, a formats list and a releases page on the website, so that I can find how Kawaro works without reading the repository.

## Acceptance criteria
- AC1: `scripts/build-site.sh` writes docs pages (seven guides plus formats), a releases page and an Atom feed, in Arabic at the root and English under `/en/`.
- AC2: Guides come from `docs/guide/NAME.md` and `NAME.ar.md`; the formats page comes from `actions/*.toml`; releases come from `CHANGELOG.md`. Nothing is written twice.
- AC3: Search works in Arabic and English through Pagefind and loads its index only when the box is used.
- AC4: `sitemap.xml` (with hreflang), `robots.txt`, `llms.txt`, `llms-full.txt` and a Markdown twin of each doc page exist; doc pages carry BreadcrumbList and TechArticle data and edit and issue links to GitHub.
- AC5: The Pages workflow runs only by hand.

## Out of scope
- Publishing, a custom domain, a CLI reference page.

## Review log
- Not a Rust change, so the agent chain is not used.
