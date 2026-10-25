[English](site-security.md) | [العربية](site-security.ar.md)

# Website security

How `https://kawaro.sbarah.com` is hosted and kept safe. The decisions are in `docs/adr/0007-site-hosting-cloudflare.md`; this page is the operating summary (S-059). Values that identify the account are placeholders here: `<ACCOUNT_ID>`, `<ZONE_ID>`, `<EXPIRY_DATE>`. The real values stay outside the repository.

## Hosting

The site is a Cloudflare Worker named `kawaro-site` with Workers Static Assets and no Worker script (no `main` in `wrangler.jsonc`), so there is no server code to attack. `scripts/prepare-site-deploy.sh` builds the site and stages the upload set in `.deploy-out/`, the assets directory of `wrangler.jsonc`, without `build.py` or caches. `scripts/site/check-deploy.sh` then checks that set with its own matcher: an allowlist of extensions, no forbidden directory, no file over 25 MiB, no private key, at most 20,000 files. `site/.assetsignore` is a second layer that Wrangler reads.

## No secret in GitHub

No workflow can deploy. `.github/workflows/site-check.yml` builds and checks the site on pull requests and on `main` with `permissions: {}` at the top, `contents: read` for its job, `persist-credentials: false` and every action pinned by a full commit SHA. No workflow uses `pull_request_target`, a Cloudflare credential or `wrangler-action`. The only secrets in the repository's workflows are the update signing secrets in the `update-manifest` job of `packages.yml`, inside the protected environment `release-signing`.

## Deploy token

Deploys run either in Cloudflare Workers Builds or by hand with `scripts/deploy-site.sh` after `wrangler login`. A Workers Builds token is a custom token for the one account `<ACCOUNT_ID>`, with only the Workers permissions a deploy needs, and an expiry date `<EXPIRY_DATE>` at most 12 months away. It is never stored anywhere else, and its rotation is planned.

Rotation: 30 days before `<EXPIRY_DATE>`, create the new token, put it in Workers Builds, run one build, then delete the old token and write the new date outside the repository. Rotate at once if a token may have leaked.

## Headers

`scripts/site-headers.py` writes `_headers` into the staged folder. Every path gets `Content-Security-Policy` (`default-src 'none'`, scripts from the same origin plus the one inline script by its SHA-256, no `unsafe-eval`, no foreign origin, `frame-ancestors 'none'`, `base-uri 'none'`, `object-src 'none'`), `Strict-Transport-Security` (`max-age=86400; includeSubDomains`, raised to one year after a quiet week, never `preload`), `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, a `Permissions-Policy` that turns every sensor off, `Cross-Origin-Opener-Policy` and `Cross-Origin-Resource-Policy` set to `same-origin`, and `X-Frame-Options: DENY`. Pagefind index files are cached as immutable and fonts for 30 days. The same script writes `.well-known/security.txt`, which points to private vulnerability reports on GitHub and expires 180 days after the build.

## DNS and TLS

The custom domain is a proxied record in the zone `<ZONE_ID>` that Cloudflare creates for the Worker. TLS is Full (strict) with Always Use HTTPS and a minimum of TLS 1.2; DNSSEC is on; no wildcard records.

## Why routes stays in wrangler.jsonc

Today the domain is attached through `routes` with `custom_domain: true` in `wrangler.jsonc`, because deploys are manual and removing it could detach the domain. That entry needs zone permissions at every deploy. When the owner moves deploys to Workers Builds with a token without zone permissions and attaches the domain once in the dashboard, `routes` is removed from `wrangler.jsonc` and from the check in `check-deploy.sh`.

## Privacy

The site has no analytics, no cookies, no third party requests and no forms that send data. Fonts, scripts and the search index are served from the same origin. Cloudflare sees ordinary request logs as the host.

## Rollback

Workers keeps earlier versions: `npx wrangler rollback` or the dashboard (Deployments, Rollback) brings the last good version back in seconds. For a lasting fix, revert the commit and deploy again. A build that fails `check-deploy.sh` never replaces the live version.

## Owner steps, not automated
Only the owner can do these; the repository work above does not need them to pass, but the site is not live until they are done. Use placeholders in anything written to the repository.
1. Confirm the `sbarah.com` zone is on your Cloudflare account and active, and that no DNS record named `hive` exists yet.
2. Turn on 2FA (a hardware key or passkey if possible) on the Cloudflare and GitHub accounts, store the recovery codes offline, and review active sessions.
3. In GitHub: enable secret scanning and push protection, private vulnerability reporting, Dependabot alerts; set "Approval for all outside collaborators" for workflows from forks; default `GITHUB_TOKEN` permission to read only; on `main` block force pushes and deletion and require linear history; disable GitHub Pages for the repository.
4. In Cloudflare, Workers Builds: connect the GitHub App to this single repository; production branch `main`; build command `scripts/site/check-deploy.sh --build`; deploy command `npx wrangler@<pinned version> deploy`; build variables `PYTHON_VERSION` and `NODE_VERSION` (not secrets); non production branches build as preview versions.
5. Replace the default build token with a custom token for one account (`<ACCOUNT_ID>`) with only the Workers permissions a deploy needs (confirm the names in the dashboard), an expiry date (`<EXPIRY_DATE>`, at most 12 months) and a calendar reminder 30 days before it. Do not store it anywhere else; use `wrangler login` for manual work.
6. First deploy: on a branch build or manually, with `workers_dev` temporarily on, open the `workers.dev` address; check headers (step 9), search in both languages and `/.well-known/security.txt`; then turn `workers_dev` off.
7. Attach the custom domain `kawaro.sbarah.com` in the dashboard (Workers, Settings, Domains and routes). Check the other services on `sbarah.com` first, then set SSL Full (strict), Always Use HTTPS, minimum TLS 1.2, DNSSEC (DS record at the registrar) and the proxy on; no wildcard records; CAA only after reading Cloudflare's current guidance.
8. After one week with no problem, change HSTS to `max-age=31536000; includeSubDomains` in `site/_headers` (no `preload`).
9. Verify with an online checker (for example Mozilla Observatory and securityheaders.com) that the headers match AC2 and the grade is at least A, and with an SSL test that TLS 1.0 and 1.1 are refused.
