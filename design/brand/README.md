# SbarHive brand

The mark is one hive cell with a saguaro cactus (صبار) cut out of it. The cell
is the same pointy top hexagon the app draws (width to height is √3 to 2); the
cactus is the name. Honey amber is the hive's own accent token.

![app icon](png/sbarhive-128.png)

## Files

| File | Use |
| --- | --- |
| `sbarhive.svg` | App icon, full colour, square 512 canvas. `accent` hexagon, `accent-ink` cactus (light tokens) |
| `sbarhive-symbolic.svg` | Tray and symbolic icon, 16 px. One path, hexagon minus cactus, `#bebebe` through `currentColor` |
| `wordmark.svg` | Mark plus SbarHive and صبار هايف, for light backgrounds. Text converted to paths |
| `wordmark-dark.svg` | Same, dark tokens, for dark backgrounds |
| `png/sbarhive-{16,32,48,64,128,256,512}.png` | Raster exports of the app icon |
| `explorations/` | The four AI generated concept sketches the mark was chosen from. Inspiration only, not assets |
| `build.py` | Draws every file above from geometry and `design/tokens.json`, and copies the favicons into `site/` |

Rebuild after any token or geometry change:

```bash
python3 design/brand/build.py
```

Needs `inkscape` (1.4 tested; rsvg-convert was not available): it shapes the
Arabic through HarfBuzz when converting text to paths, merges the symbolic
icon into a single path, and exports the PNGs.

## How the mark was chosen

Four directions were generated with `fal-ai/nano-banana-2` on 19 October 2026
and judged at 16 px:

| Sketch | Idea | Verdict |
| --- | --- | --- |
| `a-cutout-hexagon.webp` | Amber hexagon, cactus as negative space | **Chosen.** Two shapes, survives 16 px, matches the app cell |
| `b-honeycomb-sprout.webp` | Seven cell cluster, sprout on top | Cells too fine at 16 px; the green sprout is off palette |
| `c-hex-cactus.webp` | Cactus built from stacked hexagons | Reads as the letter J |
| `d-monoline-tile.webp` | Monoline cactus on a dark tile | Thin strokes vanish when small |

The final mark is drawn by hand in `build.py`, not traced from the sketch:

- Hexagon circumradius 248 on a 512 grid, corners rounded at 28.
- Trunk 80 wide with a fully round top; arms 56 wide with round tops and a
  round outer elbow; 40 units between each arm and the trunk, so the arms stay
  separate at 32 px and merge into one readable silhouette at 16 px.
- The trunk stops above the bottom vertex, so the hexagon stays one piece and
  the cactus stands on a band of honey.
- The cactus fill is `accent-ink`, the ink the app uses on the hovered
  (accent) cell, at about 5.6:1 against `accent`.

## Symbolic icon

`sbarhive-symbolic.svg` follows both Linux conventions: GTK recolours
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
`https://hive.sbarah.com/`. Change them in `site/index.html` and
`site/en/index.html` if the site is hosted elsewhere.
