# Website analytics (Umami)

The public website counts visits in aggregate with Umami, without cookies and without identifying visitors. The desktop app has no telemetry and never loads this script.

## One place to change

Everything lives in `scripts/site/analytics.json`:

```json
{
  "host": "https://stats.sbarah.com",
  "website_id": "...",
  "domains": ["kawaro.sbarah.com"]
}
```

- `host`: the Umami server. Change it to move the stats host. The script tag (`site/build.py`), the CSP in `scripts/site-headers.py` (`script-src` and `connect-src`) and the two CSP tests all read it, so nothing else needs editing.
- `website_id`: the Umami website id.
- `domains`: hostnames allowed to report (`data-domains`). If the site domain changes, change it here and in `BASE` of `site/build.py`. Other hosts, including local previews, send nothing.

Then rebuild and deploy (`docs/website-deploy.md`).

## Script attributes

`data-do-not-track="true"`, `data-performance="true"`, `data-errors="true"`. `data-exclude-search` is deliberately not used because it would drop UTM parameters.

## Events

Events are `data-umami-event` attributes on real elements, added by `site/build.py` (`track()` and the button builders). No custom JavaScript. Props are fixed words or a hostname, never personal data or free text.

| Event | Props | Element |
| --- | --- | --- |
| `cta-click` | `where`: `nav-download`, `hero-download`, `open-source-github` | the CTA buttons |
| `download` | `target`: `windows/msi`, `linux/deb`, `linux/rpm` | download buttons |
| `outbound-click` | `to`: hostname only | every link to another site |
| `docs-search` | none | the docs search box (never the query text) |
| `language-switch` | `to`: `ar` or `en` | the language links |

Funnel: home page, `cta-click`, `download`; then docs `/docs/install/` and its verify section.

There is no copy button on the site today, so there is no `copy-command` event. Add one only on a real button element.

## Cloudflare beacon

Cloudflare Web Analytics can inject `static.cloudflareinsights.com/beacon.min.js` into the pages. The CSP blocks it (a console error, harmless but noisy). Turn it off in the dashboard: Workers and Pages, `kawaro-site`, Settings, Domains and routes (or Metrics), disable Web Analytics; or Analytics and Logs, Web Analytics, the `kawaro.sbarah.com` site, remove automatic setup.
