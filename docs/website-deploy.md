# Website deploy

The site is static, built into `site/` and served by Cloudflare Pages (project `sbarhive`) at <https://hive.sbarah.com/>.

Redeploy after any change to `site/build.py`, `docs/guide/`, `actions/` or `CHANGELOG.md`:

```
scripts/deploy-site.sh
```

It rebuilds the site (including the Pagefind index, which is not committed), leaves out `build.py`, writes `_headers` (strict CSP, HSTS, nosniff and more) and `.well-known/security.txt` with `scripts/site-headers.py`, and uploads with `wrangler pages deploy`. It needs `wrangler login` once on the machine; no token is stored in the repo.

The domain `hive.sbarah.com` is a CNAME to `sbarhive.pages.dev` in the `sbarah.com` zone, attached to the Pages project as a custom domain. The canonical address comes from `BASE` in `site/build.py`.
