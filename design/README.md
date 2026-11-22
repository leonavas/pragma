# design

Brand sources for pragma. Nothing here ships; the site uses the generated files.

## Decision (2026-11-21)

The mark is direction "Pipe": a bar and a chevron that together make a p. The chevron comes from the
old `>_` mark. The mark is lime (`#c8f560`). Next to it, the word "pragma" is set in Pragma Reto in the
ink color. The other directions (Reto, Prompt, Marcador, Tag) were removed; the proposal page
with all five is `design/logo/index.html` at commit `b4a26e1`.

## Contents

- `logo/pipe.svg` is the mark. `logo/pipe-logotipo.svg` is the mark with the word.
  `logo/readme-banner.svg` is the banner of the root `README.md`.
- `font/build_reto.py`: source of the Pragma Reto font. Each glyph is a set of rectangles on a grid
  (stroke 3, x-height 16, cap height 22, descender 8). `font/specimen.html` shows all glyphs.
  `font/martian-mono.woff2` is used by the specimen and by the OG page.
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
