# Performance

Pragma optimizes builds by default: prebuilt HTML, `load` fetch started in `<head>`, per-route CSS (inlined in production when <= 14 kB gzip), per-page JS, link prefetch and Speculation Rules, 103 Early Hints, precompressed `.br`/`.gz` assets, hashed immutable assets, unused CSS stripped, and `pragma check` budgets.

## Rules

- Navigate with `<a href>`.
- Match skeleton shapes to the final content.
- Set `width` and `height` on every `<img>`. Add `loading="lazy"` below the fold. Store AVIF or WebP images in `public/`.
- Set `font-display: swap` on custom fonts. Avoid `<link rel="preload" as="font">` unless proven faster on slow 4G.
- Give `<for>` a stable `key`.
- Omit `.js` on pages without interaction.
- Keep heavy single-page code on that page. Load interaction-only code with `await import()` inside the action.
- Return only displayed fields from `load`. Run independent requests with `Promise.all`.
- Do not raise budgets in `pragma.json`; reduce code size instead.
- Do not import large modules in `lib/state/`. Do not fetch data in `mount`. Do not enable `ssr` by default.
