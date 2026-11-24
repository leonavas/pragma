# Performance

Pragma optimizes builds by default: prebuilt HTML with the scripts' initial state rendered (`data.md`, "Prebuilt HTML"), `load` fetch started in `<head>`, per-route CSS with only the theme rules the page can match (inlined in production when <= 14 kB gzip, so small pages have no render-blocking stylesheet), font preload for the faces the prebuilt HTML shows, per-route JS chunks, link prefetch and Speculation Rules, 103 Early Hints, precompressed `.br`/`.gz` assets, hashed immutable assets (JS, CSS, and the `public/` files that CSS names with `url()`, such as fonts), and unused CSS stripped. `pragma build` prints the JS and CSS gzip size of each page and marks a page whose CSS is over 14 kB (a render-blocking stylesheet); it does not enforce limits. When a page is marked, look for theme or component rules it does not need on first paint.

## JS chunks

`pragma build` bundles the client JS (`pragma dev` keeps one file per module):

- The core runtime is one file (`/assets/core/boot-<hash>.js`).
- Each route gets one chunk with its page, components, stores and the code only it uses (`/assets/routes/<route>-<hash>.js`). Asset names are URL-safe: `[id]` becomes `_id_` (`/assets/css/app/_id_-<hash>.css`), so no asset URL gets a redirect.
- Code used by 2+ routes goes to `/assets/chunks/shared<nnn>-<hash>.js` (`shared000`, `shared001`, …), one chunk per set of routes. A set under 4 kB joins a chunk that a wider set of routes uses.
- `await import()` stays a separate chunk. npm packages stay in `/assets/npm/` (`packages.md`).
- The bodies of `<if>`, `<else-if>`, `<else>`, `<for>` and `<empty>` go to the page's template module (`/assets/tpl/<route>-<hash>.js`), and the page `modulepreload`s it. The prebuilt HTML keeps only a short comment where each block renders (`<!--p:if t3 state.open-->`), so a page with many blocks or long lists stays small. The browser parses a body only when a block first renders it.
- The page `modulepreload`s every chunk it runs. The router prefetches the HTML and chunks of a link on hover, focus, and when the link enters the viewport, also for links rendered after the page loads (`/app/:id` links included). Viewport prefetch starts after the `load` event and the page's first `/_data` and `/_layout` responses, when the browser is idle, and is off when the user turns on Save-Data. The router requests the clean page URL (`/app`, not `/app/index.html`).

## Lazy-load heavy code

Code that runs only after a click (export to XLSX or PDF, a parser, a chart in a closed modal) does not need to be in the route chunk. Put it in a `utils/` module and load that module with `await import()` inside the action.

`utils/export-xlsx.js`:

```js
import { utils, writeFile } from 'xlsx'

export function exportXlsx(rows, name) {
  const book = utils.book_new()
  utils.book_append_sheet(book, utils.json_to_sheet(rows), 'Data')
  writeFile(book, name)
}
```

`routes/report.html`:

```html
<script>
  let rows = []

  async function download() {
    const { exportXlsx } = await import('../utils/export-xlsx.js')
    exportXlsx(rows, 'report.xlsx')
  }
</script>

<button type="button" @click="download">Export XLSX</button>
```

- `pragma build` puts the module in `/assets/chunks/<name>-<hash>.js` and the packages it imports in `/assets/npm/`. The page does not `modulepreload` them; the browser fetches them on the first click. `pragma dev` does the same per file.
- Import named exports in the `utils/` module, not `import('pkg')` in the page: the build tree-shakes the package only through named imports.
- Use `import()` only. A static `import` of the same module anywhere in the route (page, component, store) puts it back in the route chunk.
- It works the same in a component `<script>` (for example the action of a modal). The markup of the component stays in the route; only the imported code is lazy.
- The page JS size that `pragma build` prints does not count lazy chunks.
- Show a pending state if the first load can be slow (`let exporting = true` before the `await`).

## Long lists

A render that runs past 12 ms leaves the rest of each outer `<for>` (not one inside another `<for>` item) for a later task, so the browser can paint and answer input in between. Each task creates at least one more item, from where the last one stopped, until the list is complete. `mount` calls share the same 12 ms: the ones left run in the next task, in order. The later tasks run at background priority (`scheduler.postTask`, else `setTimeout`). You add nothing to the markup.

- Items render in list order, except inside a reverse-column scroller ("Scroll that starts at the end" in `css.md`): there the last items render first, so a chat shows its newest messages first.
- `mount` of a component in an item runs in the task that created it or a later one. Do not assume the whole list is on the page in `mount`; read it again on `ResizeObserver` or when you need it.
- Items that a later task adds do not get `data-entering`.

## Page transitions

The router swaps pages inside a View Transition. Only `<main data-page>` fades in (150 ms, from opacity 0.5); the layout around it (header, sidebar) changes at once and does not blink. To animate another element across pages, give it its own `view-transition-name` only while a transition runs (`:global(html[data-view-transition]) .x { view-transition-name: x; }`): an element with a name is a stacking context and a backdrop root, so a `backdrop-filter` inside it stops blurring. With `prefers-reduced-motion`, there is no transition.

## Rules

- Navigate with `<a href>`. A `<button>` that calls `navigate()` gets no prefetch.
- Match skeleton shapes to the final content.
- Set `width` and `height` on every `<img>`. Add `loading="lazy"` below the fold. Store AVIF or WebP images in `public/`.
- Set `font-display: swap` on custom fonts and declare them with `@font-face` in `lib/theme.css`, with the file in `public/` and a root path (`url('/fonts/x.woff2')`). The build serves the font from a hashed, immutable `/assets/` URL and preloads the fonts the first paint of each page uses (`css.md`). Do not write `<link rel="preload" as="font">` in `_layout.html`.
- Give `<for>` a stable `key`.
- Drive per-frame animations (scroll, drag, progress) with `frame` from `'pragma'`: read layout first, then assign state (`state.md`, "Per-frame updates").
- Use per-frame `:--name` values only in `transform` and `opacity`, never in `top`/`left`/`width`/`height`/`margin`/`padding`/`inset` (`css.md`, "Values that change every frame").
- Omit `.js` on pages without interaction.
- Keep heavy single-page code on that page. Load interaction-only code with `await import()` inside the action ("Lazy-load heavy code").
- Return only displayed fields from `load`. Run independent requests with `Promise.all`.
- Keep page JS small. Read the size table of `pragma build` after you add a package or a large module.
- Do not import large modules in `lib/state/`. Do not fetch data in `mount`. Do not enable `ssr` by default.
- Do not set the first scroll position in `mount` (`scrollTo`): it runs after the first paint. Open a chat or log at its end with a reverse column (`css.md`, "Scroll that starts at the end").
