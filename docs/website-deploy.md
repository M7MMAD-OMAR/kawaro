# Website deploy

The site is static, built into `site/` and served by a Cloudflare Worker with static assets (worker `kawaro-site`) at <https://kawaro.sbarah.com/>.

Redeploy after any change to `site/build.py`, `docs/guide/`, `actions/` or `CHANGELOG.md`:

```
scripts/deploy-site.sh
```

It rebuilds the site (including the Pagefind index, which is not committed), leaves out `build.py`, writes `_headers` (strict CSP, HSTS, nosniff and more) and `.well-known/security.txt` with `scripts/site-headers.py`, and uploads with `wrangler deploy` (config in `wrangler.jsonc`). It needs `wrangler login` once on the machine; no token is stored in the repo.

The domain `kawaro.sbarah.com` is a Worker custom domain declared in `wrangler.jsonc`; Cloudflare creates the DNS record and the certificate. HSTS starts at one day and is raised after a week of clean traffic (ADR 0007). The canonical address comes from `BASE` in `site/build.py`.
