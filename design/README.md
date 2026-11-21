# design

Brand sources for pragma. Nothing here ships; the site uses the generated files.

## Decision (2026-11-21)

The logo is direction "Reto": a letter p with no curves. The word "pragma" in the same shapes is the
logotype. The first letter is lime (`#c8f560`), the other letters use the ink color. The old `>_` mark
is not used.

## Contents

- `logo/index.html`: the proposal page with the five directions (Pipe, Reto, Prompt, Marcador, Tag).
  `logo/reto.svg` is the favicon. `logo/reto-logotipo.svg` is the logotype.
- `font/build_reto.py`: source of the Pragma Reto font. Each glyph is a set of rectangles on a grid
  (stroke 3, x-height 16, cap height 22, descender 8). `font/specimen.html` shows all glyphs.
- `og/og.html`: source of `site/public/og.png`.

## Commands

```sh
# font -> site/public/fonts/pragma-reto.woff2 and design/font/pragma-reto.ttf
uv run --no-project --with fonttools --with brotli design/font/build_reto.py

# Open Graph image
playwright screenshot --viewport-size=1200,630 "file://$PWD/design/og/og.html" site/public/og.png
```

## Pending

- `core/starter/public/favicon.svg` has the new p in the starter colors. Build the CLI again to put it
  in `pragma new`.
- To add a glyph, add its rectangles to `glyphs()` in `build_reto.py` and run the font command.
