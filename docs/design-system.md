# Kawaro design system

Short reference. The source of truth is `design/tokens.json`; this file
explains it and never overrides it.

## Tokens

| Group | Tokens | Notes |
| --- | --- | --- |
| Surfaces | `surface-0` to `surface-3` | Elevation is expressed by these tones only. Higher means a later surface number |
| Lines | `border` | Full borders only, never one side |
| Text | `text-primary`, `text-secondary` | Both pass 4.5:1 on every surface in their scheme |
| Accent | `accent`, `accent-ink` | Light `#D9921A`, dark `#F0A93B`. In light mode the accent is too pale for text or focus rings (about 2.4:1 on `surface-0`): use it only as a fill behind `accent-ink` |
| Status | `success`, `warning`, `danger`, `progress` | |
| Space | `space-1` to `space-6` | 4, 8, 12, 16, 24, 32 px |
| Radius | `radius-sm`, `md`, `lg` | 6, 10, 14 px |
| Border width | `default`, `focus` | 1 and 2 px |
|| Glass | `glass-2`, `glass-3` | Tinted translucent overlays |
|| Motion | `ring-delay-ms`, `highlight-move`, `press`, `hive-close` | Animation timings and easings |
| Motion | `hive-open` 120 ms, scale 0.92 to 1, ease-out; `hover` 80 ms | Zero when the system asks for reduced motion |

Generated outputs, never edited by hand:

- Slint globals for the app (`tokens.slint`).
- `site/tokens.css` for the website: `python3 scripts/tokens-to-css.py`.
  Light values on `:root`, dark values under `prefers-color-scheme: dark`, a
  `[data-scheme="light"]` and `[data-scheme="dark"]` pair so one section can pin
  its scheme, and motion set to zero under `prefers-reduced-motion: reduce`.
- The website pages: `python3 site/build.py` (Arabic at the root, English in
  `site/en/`). It reads `design/tokens.json` for the hive geometry.

The landing page needs larger type than the app; its sizes are multiples of
`font-size-title` (body 1.125, h2 1.75, h1 2.375 to 3.25), so the scale still
moves with the token.

## Hex geometry

- Pointy-top hexagons in axial coordinates `(q, r)`.
- `r` is the circumradius; a cell is `√3·r` wide and `2r` tall.
- Spacing `s = r + gap / √3` leaves exactly `gap` (6 px) between flat edges.
- Centre `x = s·(√3·q + √3/2·r)`, `y = 1.5·s·r`. Mirror `x` for right-to-left.
- Order: the centre cell, then each ring starting at direction 4 `(-1, +1)`,
  walking the six directions. Ring 1 has 6 cells, ring 2 has 12.
- Cell sizes `sm` 56, `md` 72, `lg` 88.
- Fills: cells `surface-2`, centre `surface-3`, hovered cell `accent` with an
  `accent-ink` label, all stroked with `border`.
- A cell shows a format code (WEBP, PDF) or a short action word. The full name
  appears in the centre cell on hover, never inside a small cell.
- Above the code sits a monoline icon (`hex.icon-size` 18 px, `hex.icon-gap`
  2 px above the code), coloured like the code: `text-primary`, `accent-ink`
  on hover. A cell without an icon shows the code alone, centred.
- Cell icons live in `crates/ui/icons/`: 24 px viewBox, 1.75 stroke, round
  caps and joins, black only (the app colourises them). Actions pick one by
  name with `icon = "..."` in `actions/*.toml`.

The implementations: `spikes/hive/src/geometry.rs` (app) and `site/build.py` and `site/app.js`
(landing page hero). Keep them in step.

## Icons

| Need | File |
| --- | --- |
| App launcher, `.desktop` file, about dialog | `design/brand/kawaro.svg` or `png/kawaro-*.png` |
| Tray, panel, any monochrome place | `design/brand/kawaro-symbolic.svg` |
| README, site header, social images | `design/brand/wordmark.svg` (light) or `wordmark-dark.svg` |
| Browser tab | `site/favicon.svg`, `site/favicon-32.png` |

## Do

- Take every colour, space, radius and duration from tokens.
- Separate layers with surface tones: a card is `surface-1` on `surface-0`,
  and `surface-2` when it sits on a `surface-1` band.
- Draw focus as a full `outline` in `text-primary`, 2 px.
- Underline links; colour alone does not mark them.
- Reverse the order of layout items for Arabic; Slint does not flip layouts.
- Write Arabic and English as two originals, not one translated from the other.

## Don't

- No `box-shadow`, `drop-shadow`, or shadowed elevation from component libraries.
- No gradients of any kind.
- No border on one side only, no side accent bars.
- No em dash or en dash in any text, comment or document.
- No hard-coded colour or length outside the generated token files.
- No light-mode accent as text, link colour or focus ring.
