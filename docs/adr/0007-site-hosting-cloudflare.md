# ADR 0007: Hosting the website and docs on Cloudflare at hive.sbarah.com

Status: accepted
Date: 2026-10-25

## Context

The maintainer wants the public website and the documentation served from `https://hive.sbarah.com`, on Cloudflare, linked to the GitHub repository. The repository is public, so security comes first: anyone can read every file and every workflow, open pull requests from forks, and watch the logs.

Facts read in the repository on 2026-10-25 (verified):

- `site/` holds source and output together. `site/build.py`, `site/app.js`, `site/style.css`, `site/tokens.css`, `site/fonts/` are sources; `index.html`, `en/`, `docs/`, `releases/`, the Markdown twins (`docs/*.md`, `releases.md`), `sitemap.xml`, `robots.txt`, `llms.txt`, `llms-full.txt` are written by `scripts/build-site.sh`. `site/pagefind/` is built by `npx --yes pagefind@1.5.2` and is gitignored. `site/.nojekyll` is a GitHub Pages leftover.
- `BASE` in `site/build.py` is still `https://m7mmad-omar.github.io/sbarhive/`. Canonical links, hreflang, the sitemap, robots, `og:image` and `llms.txt` all use it.
- Arabic is at the root and English under `/en/`. Docs pages exist in both (`/docs/NAME/` and `/en/docs/NAME/`), with a Markdown twin each, `hreflang` in the head and in `sitemap.xml`, and `x-default` pointing to Arabic.
- The markup has one inline executable script (`document.documentElement.classList.add("js")` in every head), data blocks that are not executed (`application/ld+json`, `application/json` for `hive-config`), and `style="--i:N"` attributes on hive cells. `app.js` sets custom properties and transforms through the CSSOM, which a CSP allows. `style.css` has only two `url()` values, both fonts in `site/fonts/`, and no `data:`. Pagefind loads a WebAssembly module and starts a same origin worker (`new Worker(workerUrl)`).
- The GitHub Pages workflow was removed in commit 82a8ffb. Release files, `SHA256SUMS` and the signed update feed (release tag `updates`, ADR 0006) live on GitHub Releases.
- `ci.yml` and `packages.yml` use actions by tag (`@v4`), not by commit SHA. CI is red on `main` until S-058 lands.
- A single maintainer pushes straight to `main`.

## Decisions

### 1. Hosting: Workers with Static Assets and no Worker script

The site is a Cloudflare Worker with Static Assets: `wrangler.jsonc` names the Worker and an assets directory, and has no `main`. There is no server code, so there is nothing to attack except the static files and the configuration. Headers come from a `_headers` file inside the assets directory, ignore rules from `.assetsignore`.

Rejected:

| Option | Why not |
| --- | --- |
| Cloudflare Pages | Works, but Cloudflare points new projects to Workers Static Assets and Pages gets no new features (to be re-checked at S-059 time). Starting on the lasting platform avoids a migration. Pages would be an acceptable fallback. |
| GitHub Pages | No custom response headers: no real CSP (`frame-ancestors` cannot be set by `<meta>`), no HSTS on the custom domain beyond the default, no cache control. It was already removed in 82a8ffb. |
| R2 for downloads | Adds a write credential to CI, a second place that holds the binaries, and splits `SHA256SUMS` and the signed update feed away from the release that the signing job (ADR 0006) already publishes to. GitHub Releases already serves the files free through a CDN. Reconsider only if GitHub limits downloads. |
| A Worker script in front of the assets | Server code, a larger attack surface, and nothing the site needs. Rejected until a feature truly needs it (it would need its own ADR). |

### 2. Downloads stay on GitHub

The site links to GitHub release files. It never proxies a binary through Cloudflare. The download page shows the `SHA256SUMS` steps. The in-app updater reads `https://github.com/M7MMAD-OMAR/sbarhive/releases/download/updates/<channel>.json`; the site is not on that path and an outage or compromise of the site cannot change what the app installs, because the app trusts only the minisign key committed in the repository (`docs/update-signing.md`).

### 3. Build and deploy: Cloudflare Workers Builds (git integration)

Two ways to connect GitHub to Cloudflare were compared.

| | (a) Workers Builds | (b) GitHub Actions with `wrangler-action` |
| --- | --- | --- |
| Cloudflare secret stored in GitHub | none | an API token, as an environment secret |
| Can a workflow, a fork PR or a compromised action leak a deploy credential | no credential exists in GitHub | the token is exposed to the deploy job and to any step in it |
| Waits for GitHub CI to be green | not natively | yes (`needs` or `workflow_run`) |
| Build runs on | a Cloudflare image (Node and Python versions set by build variables) | a GitHub runner |
| Cloudflare side | a GitHub App installed on this one repository, plus a build token that Cloudflare creates | a token the owner creates and rotates |

Decision: **(a)**. The top risk for a public repository is a leaked or abused deploy credential, and (a) leaves none in GitHub. A hostile pull request, a poisoned action or a leaked log cannot deploy, because no workflow can. The weakness of (a) is that it does not wait for GitHub CI. It is accepted for two reasons: the site depends on the site build and not on the Rust crates, so a red `cargo test` must not block a docs fix; and the Workers Builds build command itself is `scripts/site/check-deploy.sh --build`, which builds the site and fails on any forbidden file, bad header file or missing page. A failing build never replaces the live version.

Settings in the Cloudflare dashboard (owner steps in S-059):

- Production branch `main`; root directory empty; build command `scripts/site/check-deploy.sh --build`; deploy command `npx wrangler@<pinned> deploy`.
- Build variables (not secrets): `PYTHON_VERSION` and `NODE_VERSION` set to versions that satisfy `tomllib` (Python 3.11 or newer). No secrets are ever added as build variables.
- Non production branches: builds on, with `wrangler versions upload` (a preview version, not a deployment). The preview URL is public but only serves files the public repository already shows.
- The GitHub App is limited to the single repository, never "all repositories".
- Review the build token Cloudflare creates: if it is broader than "edit this Worker", replace it with a custom token (decision 5).

Fallback (b), not shipped: if CI gating becomes necessary, a separate workflow `deploy-site.yml` triggered by `workflow_run` of `CI`, with: `event == 'push'` and `head_repository.full_name == github.repository` and `head_branch == 'main'` checked (a fork PR from its own `main` also arrives as `head_branch == 'main'`); `actions/checkout` at `head_sha` with `persist-credentials: false`; top level `permissions: {}` and `contents: read` on the job; `environment: site-production` restricted to `main`, holding the token as an environment secret; `concurrency` group; actions pinned by SHA. A new ADR is required to switch.

### 4. Threat model for a public repository

| Threat | Control |
| --- | --- |
| Secret committed or printed in a log | No Cloudflare secret exists in GitHub (decision 3). GitHub secret scanning and push protection on. `check-deploy.sh` fails on files named `.env`, `*.key`, `*.pem` or `*.pub`, and on any private key header, in the upload set. Wrangler config holds no `account_id` and no token. |
| Malicious pull request from a fork runs with secrets | No workflow uses a Cloudflare credential. Repository setting: approval required to run workflows from all outside contributors. Default `GITHUB_TOKEN` permission read only. |
| `pull_request_target` or `workflow_run` misuse | `pull_request_target` is forbidden in this repository (a test greps for it). No workflow checks out fork code with write permissions. |
| Supply chain of actions | Every action in new workflows is pinned by full commit SHA with the version as a comment; Dependabot (`github-actions`, weekly) proposes bumps. Pinning `ci.yml` and `packages.yml` is a separate story (S-058 left it out; see Still open). |
| Supply chain of the site build | `wrangler` is pinned by exact version in the deploy command. Pagefind is pinned to 1.5.2 but fetched by `npx --yes`, which pins a version and not an integrity hash. A `package.json` with a lockfile and `npm ci` is the follow up (S-060). |
| Unreviewed change on `main` | A single maintainer cannot require review of his own commits, so a required review is not listed as a control. What works for one maintainer: block force pushes and branch deletion on `main`; require linear history; no bypass for admins except as an explicit choice; `CODEOWNERS` for `.github/`, `site/`, `wrangler.jsonc` and `docs/adr/` so a second contributor, if one appears, cannot touch them unreviewed; signed commits optional. |
| Account takeover (GitHub or Cloudflare) | 2FA on both accounts, preferably a hardware key or passkey; recovery codes stored offline; a short list of active sessions; no long lived API keys on disk (`wrangler login` uses a short lived OAuth token; do not export `CLOUDFLARE_API_TOKEN` in a shell profile). |
| Overbroad Cloudflare token | See decision 5. |
| Hostile file in the upload | `.assetsignore` and the allowlist in `check-deploy.sh` (decision 6). |
| Content injection (XSS) | No server, no user input stored, strict CSP (decision 7). `innerHTML` in `app.js` receives only Pagefind excerpts built from our own pages. |
| Subdomain takeover | No dangling DNS records. Remove records that point to services no longer used (the old `github.io` target if any). |
| Stale copy of the old site | GitHub Pages is switched off in the repository settings (owner step) so no second, older copy answers. |

### 5. Cloudflare API token (least privilege)

Workers Builds creates its own token. To stay in control, replace it with a custom token (and use the same kind of token for any manual `wrangler deploy`):

- Resources: one account and one zone (`sbarah.com`), never "all accounts" or "all zones".
- Permissions: only those that a Workers deploy needs. Start from the dashboard template "Edit Cloudflare Workers" and remove everything not needed; the exact permission names change over time, so they are confirmed in the dashboard and in Cloudflare's current docs at the time (do not rely on this ADR for them). "Workers Scripts: Edit" is account wide, so any other Worker on the same account is inside its blast radius: if the owner has other Workers, use a separate account or accept the risk knowingly. The custom domain is attached by hand once (owner step), so the token does not need zone DNS or route edit permissions; that is the preferred way. `routes` with `custom_domain` in `wrangler.jsonc` would need those zone permissions on every deploy, so it is not used.
- Expiry date set to at most 12 months. Rotation note: put a calendar reminder 30 days before expiry; create the new token, update Workers Builds, delete the old one; the date is written in `docs/site-security.md` as a placeholder the owner fills outside the repository.
- No token in the repository, in `wrangler.jsonc`, in logs, in shell history or in a profile file. Account ids are placeholders in docs.

### 6. What is uploaded: `.assetsignore` plus an allowlist

`wrangler.jsonc` (no comments, parseable by `jq`):

```
{
  "name": "sbarhive-site",
  "compatibility_date": "2026-10-25",
  "workers_dev": false,
  "preview_urls": true,
  "assets": { "directory": "./site", "html_handling": "auto-trailing-slash", "not_found_handling": "none" }
}
```

No `main`, no `account_id`, no `vars`, no bindings, no `routes`.

`site/` mixes sources and output, so the safe rule is an **allowlist enforced by a script**, with `.assetsignore` as the first layer:

- `site/.assetsignore` ignores: `build.py`, `*.py`, `*.sh`, `*.map`, `*.toml`, `*.lock`, `*.log`, `*.bak`, `*.orig`, `*.swp`, `.env*`, `*.key`, `*.pem`, `.git*`, `.nojekyll`, `node_modules/`, `backlog/`, `__pycache__/`. It does **not** ignore `*.md` (the generated Markdown twins are the targets of `llms.txt`) and does **not** ignore `.*` (that would drop `.well-known/`).
- `scripts/site/check-deploy.sh` computes the upload set (every file under the assets directory minus `.assetsignore` matches, using its own matcher) and fails with exit 1, naming the file, when any file has an extension outside the allowlist (`html css js json xml txt md svg png woff2 webmanifest`, Pagefind `pagefind pf_meta pf_index pf_fragment pf_filter pf_dir`, and the file `OFL.txt`), when a path is under a forbidden directory, when a file is over 25 MiB or the count is over 20,000 (Cloudflare limits), or when a file contains a private key header. The script does not depend on `wrangler deploy --dry-run` printing a file list.
- `_headers` and `_redirects` are Cloudflare control files and are not served.
- `.well-known/security.txt` is generated by `site/build.py` (see below). Whether Wrangler uploads dot directories is checked in the first deploy (owner step); if it does not, the fallback is a plain `/security.txt` plus a redirect.

`/.well-known/security.txt` (RFC 9116): `Contact:` is the GitHub private advisory URL (no email address, no personal data), `Expires:` is the build date plus 180 days (so every deploy renews it), `Preferred-Languages: en, ar`, `Canonical:` and `Policy:` (the `SECURITY.md` page on GitHub).

### 7. Security headers (`site/_headers`)

For `/*`:

| Header | Value |
| --- | --- |
| `Content-Security-Policy` | `default-src 'none'; script-src 'self' 'wasm-unsafe-eval' 'sha256-<hash of the js flag script>'; style-src 'self'; style-src-attr 'unsafe-inline'; img-src 'self'; font-src 'self'; connect-src 'self'; worker-src 'self'; manifest-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'; upgrade-insecure-requests` |
| `Strict-Transport-Security` | `max-age=86400; includeSubDomains` at first deploy, raised to `max-age=31536000; includeSubDomains` after one week with no problem. No `preload` (see below). |
| `X-Content-Type-Options` | `nosniff` |
| `Referrer-Policy` | `no-referrer` |
| `Permissions-Policy` | `accelerometer=(), autoplay=(), browsing-topics=(), camera=(), clipboard-read=(), display-capture=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), midi=(), payment=(), usb=()` |
| `Cross-Origin-Opener-Policy` | `same-origin` |
| `Cross-Origin-Resource-Policy` | `same-origin` |
| `X-Frame-Options` | `DENY` (old browsers; `frame-ancestors` is the real control) |

Notes:

- `script-src` has no `'unsafe-inline'`. The one inline script is allowed by its SHA-256 hash, computed by the build from the real markup, so a change of that script without the hash fails the test. The `ld+json` and `application/json` blocks are data and need no allowance. `'wasm-unsafe-eval'` is there only for Pagefind's WebAssembly. Dynamic `import()` of `pagefind.js` is covered by `'self'`.
- `style-src-attr 'unsafe-inline'` is a deliberate, narrow exception for `style="--i:N"` on hive cells (a custom property carrying a number cannot inject script). A test bounds it: every `style=` attribute in the built HTML must match `^--[a-z-]+:-?[0-9.]+;?$`. `<style>` elements and style sheets stay under `style-src 'self'`.
- A hash is fragile if the inline script changes; moving the script to a file would remove the exception, and may be done later.
- No `report-uri`: a report endpoint would need server code and would collect visitor data.
- HSTS `preload`: the preload list works on the registrable domain, so submitting means `sbarah.com` and every subdomain of it must serve HTTPS with `includeSubDomains`, and removal takes months. That decision belongs to the whole domain, not to this site, so it is not made here. `includeSubDomains` on `hive.sbarah.com` affects only `*.hive.sbarah.com`.
- Do not enable the zone wide Cloudflare HSTS setting. It would apply to every host of `sbarah.com`.

Cache rules (Workers Static Assets sends `ETag` and `Cache-Control: public, max-age=0, must-revalidate` by default, which is right for HTML):

- Immutable for one year only for names with a content hash: `/pagefind/fragment/*`, `/pagefind/index/*`, `/pagefind/*.pf_meta`.
- Not immutable, because their names do not change when the content does: `style.css`, `tokens.css`, `app.js`, `pagefind.js`, `pagefind-entry.json`, `pagefind-worker.js`, `wasm.*.pagefind`. They keep the default revalidation. Fonts get `max-age=2592000` (30 days).
- Fingerprinting `style.css` and `app.js` in the build would allow `immutable` for them; that is a later optimisation.

### 8. DNS and TLS

- The `sbarah.com` zone must be on the owner's Cloudflare account (owner step; nothing here works otherwise).
- The custom domain `hive.sbarah.com` is attached to the Worker in the dashboard (Workers, Settings, Domains and routes). Cloudflare creates the proxied DNS record and the certificate. If a record for `hive` already exists, remove it first.
- Zone settings, which are zone wide and must be checked against everything else served from `sbarah.com` (for example `https://sbarah.com/support`) before they are turned on: SSL mode Full (strict), Always Use HTTPS, minimum TLS 1.2 (TLS 1.3 on), DNSSEC on with the DS record added at the registrar, Automatic HTTPS Rewrites on.
- Proxy on (orange cloud) for the `hive` host. No wildcard DNS records (`*.sbarah.com`).
- CAA records: optional hardening. A wrong CAA record can stop Cloudflare from issuing or renewing the certificate, so the owner reads Cloudflare's current CAA documentation first and tests; do not add `issuewild ";"`, because the Universal certificate covers `*.sbarah.com`.
- If the zone sends no mail, add `v=spf1 -all` on the host and a `_dmarc` record with `p=reject` on the apex, so the name cannot be spoofed.

### 9. Privacy

The site follows `CLAUDE.md`: no analytics, no cookies, no third party request, no CDN font or script, fonts self hosted (IBM Plex Sans Arabic, OFL). Cloudflare Web Analytics and Cloudflare's "Bot Fight" or challenge scripts that inject JavaScript stay off, because the CSP would block them and they break the promise. The CSP and a test that finds no `http(s)` URL in `src`, `href` of link tags or `import` that points to another origin enforce it.

### 10. Environments, rollback and a broken build

- Staging: a non production branch build gives a preview URL on `workers.dev` (for review before a change reaches `main`). The very first deploy happens with the custom domain not yet attached: the owner opens the `workers.dev` address once (`workers_dev` is turned on for that single deploy, then back off, because it is an extra public name), checks headers, then attaches the domain.
- Rollback: Workers keeps previous versions. `npx wrangler rollback` (or the dashboard, Deployments, "Rollback") brings the last good version back in seconds. For a lasting fix use `git revert <commit>` and let Workers Builds redeploy.
- Broken build: the live version stays untouched, because a deploy only runs after a successful build command, and the build command includes `check-deploy.sh`. A broken page that passes the checks is fixed by rollback first, then a revert.

### 11. Docs on the site

Keep the docs inside the same site under `/docs` (Arabic) and `/en/docs` (English). Reasons: one domain, one CSP, one search index, one set of headers, and the files already come from `docs/guide/` so nothing is written twice.

- Language: the switch link exists in the header and footer; both languages keep a separate URL (no automatic redirect by browser language, which needs server code and hides one language from crawlers). RTL comes from `dir="rtl"` on the Arabic pages.
- `hreflang`: each page carries `ar`, `en` and `x-default` (Arabic) in the head and in `sitemap.xml`. After the domain change they must all be absolute `https://hive.sbarah.com/...` URLs; the canonical of a preview URL still points to the production host.
- Search: Pagefind, built at build time, same origin, loads only when the box is used (as today). It indexes both languages.
- Agents: keep `llms.txt`, `llms-full.txt` and the Markdown twins.
- Versioned docs: not now. One "latest" version plus the Releases page is enough while the product is in beta. When a stable release diverges from the previous one, build frozen copies per minor version under `/docs/v0.5/` with `noindex` and a canonical link to the latest page, so search does not index duplicates.

## Stories

| Story | Scope |
| --- | --- |
| S-059 | Wrangler config, `_headers`, `.assetsignore`, `check-deploy.sh`, domain switch in `build.py`, `security.txt`, new CI workflow `site-check.yml`, `docs/site-security.md`, owner steps |
| S-060 (later) | Lockfile for Pagefind and wrangler (`npm ci`), pin actions in `ci.yml` and `packages.yml`, a custom 404 page, fingerprinted CSS and JS |

## Still open

- CI is red on `main` until S-058 lands. With Workers Builds this does not block the site (decision 3), but it blocks the next story by the project rules.
- Pinning `ci.yml` and `packages.yml` by SHA was excluded from S-058 and is left to S-060, because it touches the release signing workflow.
- Whether the Workers Builds image honours the Python version variable and ships Python 3.11 or newer. Verified at the first build; if not, the build command installs it or the site build moves to option (b).
- Whether Wrangler uploads the `.well-known` directory.
- Whether the other services on `sbarah.com` accept SSL Full (strict) and minimum TLS 1.2 before the zone settings change.
- HSTS preload for `sbarah.com`, as a separate decision for the whole domain.
- Who gets the `CODEOWNERS` entries once there is a second maintainer.
