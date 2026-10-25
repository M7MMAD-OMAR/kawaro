# Kawaro brand

The mark is one hive cell with a traditional domed beehive (كوّارة, kawwara)
cut out of it, the word the name comes from. The cell is the same pointy top
hexagon the app draws (width to height is √3 to 2). Honey amber is the hive's
own accent token.

![app icon](png/kawaro-128.png)

## Files

| File | Use |
| --- | --- |
| `kawaro.svg` | App icon, full colour, square 512 canvas. `accent` hexagon, `accent-ink` beehive (light tokens) |
| `kawaro-symbolic.svg` | Tray and symbolic icon, 16 px. One path, hexagon minus beehive, `#bebebe` through `currentColor` |
| `wordmark.svg` | Mark plus Kawaro and كوارو, for light backgrounds. Text converted to paths |
| `wordmark-dark.svg` | Same, dark tokens, for dark backgrounds |
| `png/kawaro-{16,32,48,64,128,256,512}.png` | Raster exports of the app icon |
| `build.py` | Draws every file above from geometry and `design/tokens.json`, and copies the favicons into `site/` |

Rebuild after any token or geometry change:

```bash
python3 design/brand/build.py
```

Needs `inkscape` (1.4 tested; rsvg-convert was not available): it shapes the
Arabic through HarfBuzz when converting text to paths, merges the symbolic
icon into a single path, and exports the PNGs.

## Geometry

The mark is drawn by hand in `build.py`:

- Hexagon circumradius 248 on a 512 grid, corners rounded at 28.
- A dome of radius 140 centred at (256, 262): the upper half follows the
  circle, the lower part has straight sides.
- Three 18 unit gaps cut the dome into four tiers, so the tiers stay separate
  at 32 px and read as one striped dome at 16 px.
- The fill is `accent-ink`, the ink the app uses on the hovered (accent) cell,
  at about 5.6:1 against `accent`.

## Symbolic icon

`kawaro-symbolic.svg` follows both Linux conventions: GTK recolours
`#bebebe` in a `-symbolic` file, and KDE replaces the `current-color-scheme`
stylesheet and its `ColorScheme-Text` class. Elsewhere it renders mid grey,
which reads on dark and light panels.

## Rules

- No gradients, shadows, outlines or effects on the mark.
- Keep clear space of at least a quarter of the mark's height on every side.
- Do not recolour the mark outside the tokens. On a dark surface use the dark
  `accent` (`wordmark-dark.svg`); in monochrome contexts use the symbolic icon.
- Do not set the wordmark in another font. Latin and Arabic are IBM Plex Sans
  Arabic SemiBold (OFL 1.1), already converted to paths.

## Site assumption

The landing page's canonical and Open Graph URLs assume GitHub Pages at
`https://kawaro.sbarah.com/`. Change them in `site/index.html` and
`site/en/index.html` if the site is hosted elsewhere.
