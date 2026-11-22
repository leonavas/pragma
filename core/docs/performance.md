# Performance

Pragma optimizes builds by default: prebuilt HTML, `load` fetch started in `<head>`, per-route CSS (inlined in production when <= 14 kB gzip), per-page JS, link prefetch and Speculation Rules, 103 Early Hints, precompressed `.br`/`.gz` assets, hashed immutable assets, and unused CSS stripped. `pragma build` prints the JS gzip size of each page; it does not enforce limits.

## Rules

- Navigate with `<a href>`.
- Match skeleton shapes to the final content.
- Set `width` and `height` on every `<img>`. Add `loading="lazy"` below the fold. Store AVIF or WebP images in `public/`.
- Set `font-display: swap` on custom fonts. Avoid `<link rel="preload" as="font">` unless proven faster on slow 4G.
- Give `<for>` a stable `key`.
- Drive per-frame animations (scroll, drag, progress) with `frame` from `'pragma'`: read layout first, then assign state (`state.md`, "Per-frame updates").
- Use per-frame `:--name` values only in `transform` and `opacity`, never in `top`/`left`/`width`/`height`/`margin`/`padding`/`inset` (`css.md`, "Values that change every frame").
- Omit `.js` on pages without interaction.
- Keep heavy single-page code on that page. Load interaction-only code with `await import()` inside the action.
- Return only displayed fields from `load`. Run independent requests with `Promise.all`.
- Keep page JS small. Read the size table of `pragma build` after you add a package or a large module.
- Do not import large modules in `lib/state/`. Do not fetch data in `mount`. Do not enable `ssr` by default.
