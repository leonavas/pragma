# Performance

Pragma optimizes builds by default: prebuilt HTML, `load` fetch started in `<head>`, per-route CSS with only the theme rules the page can match (inlined in production when <= 14 kB gzip, so small pages have no render-blocking stylesheet), per-page font preload, per-route JS chunks, link prefetch and Speculation Rules, 103 Early Hints, precompressed `.br`/`.gz` assets, hashed immutable assets, and unused CSS stripped. `pragma build` prints the JS gzip size of each page; it does not enforce limits.

## JS chunks

`pragma build` bundles the client JS (`pragma dev` keeps one file per module):

- The core runtime is one file (`/assets/core/boot-<hash>.js`).
- Each route gets one chunk with its page, components, stores and the code only it uses (`/assets/routes/<route>-<hash>.js`). Asset names are URL-safe: `[id]` becomes `_id_` (`/assets/css/app/_id_-<hash>.css`), so no asset URL gets a redirect.
- Code used by 2+ routes goes to `/assets/chunks/shared<n>-<hash>.js`, one chunk per set of routes. A set under 4 kB joins a chunk that a wider set of routes uses.
- `await import()` stays a separate chunk. npm packages stay in `/assets/npm/` (`packages.md`).
- The page `modulepreload`s every chunk it runs. The router prefetches the HTML and chunks of a link on hover, focus, and when the link enters the viewport, also for links rendered after the page loads (`/app/:id` links included). Viewport prefetch starts after the `load` event, when the browser is idle, and is off when the user turns on Save-Data. The router requests the clean page URL (`/app`, not `/app/index.html`).

## Rules

- Navigate with `<a href>`. A `<button>` that calls `navigate()` gets no prefetch.
- Match skeleton shapes to the final content.
- Set `width` and `height` on every `<img>`. Add `loading="lazy"` below the fold. Store AVIF or WebP images in `public/`.
- Set `font-display: swap` on custom fonts and declare them with `@font-face` in `lib/theme.css`. The build preloads the fonts each page uses (`css.md`). Do not write `<link rel="preload" as="font">` in `_layout.html`.
- Give `<for>` a stable `key`.
- Drive per-frame animations (scroll, drag, progress) with `frame` from `'pragma'`: read layout first, then assign state (`state.md`, "Per-frame updates").
- Use per-frame `:--name` values only in `transform` and `opacity`, never in `top`/`left`/`width`/`height`/`margin`/`padding`/`inset` (`css.md`, "Values that change every frame").
- Omit `.js` on pages without interaction.
- Keep heavy single-page code on that page. Load interaction-only code with `await import()` inside the action.
- Return only displayed fields from `load`. Run independent requests with `Promise.all`.
- Keep page JS small. Read the size table of `pragma build` after you add a package or a large module.
- Do not import large modules in `lib/state/`. Do not fetch data in `mount`. Do not enable `ssr` by default.
