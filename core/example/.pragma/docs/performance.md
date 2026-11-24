# Performance

Pragma optimizes builds by default: prebuilt HTML with the scripts' initial state rendered (`data.md`, "Prebuilt HTML"), `load` fetch started in `<head>`, per-route CSS with only the theme rules the page can match (inlined in production when <= 14 kB gzip, so small pages have no render-blocking stylesheet), font preload for the faces the prebuilt HTML shows, JS composed per route (below), link prefetch and Speculation Rules, 103 Early Hints, precompressed `.br`/`.gz` assets, hashed immutable assets (JS, CSS, and the `public/` files that CSS names with `url()`, such as fonts), and unused CSS stripped.

`pragma build` prints, per route, the first-load JS (the route entry and every module it preloads) as raw, gzip and brotli bytes, and the CSS gzip size. A static route prints `0` JS. It marks a page whose CSS is over 14 kB (a render-blocking stylesheet); it does not enforce limits. When a page is marked, look for theme or component rules it does not need on first paint.

## JS per route

The build reads everything a route renders (layout, nested layouts, page, components, slot content, stores, the bodies of every `<if>`/`<for>` branch, `load`, CSS) and loads only the runtime it needs. You add no flags.

- **Static route**: no directive, no script, no store, no `load`, no `<dialog>` or `[popover]` (layout included). It ships no JS: no runtime, no modulepreload. Links to it and from it are normal page loads with a cross-document view transition. One `@click` or `{}` in the layout makes every route reactive.
- **Reactive route**: one entry module (`/assets/routes/<route>-<hash>.js`) imports the kernel (state, DOM bindings, events) and only the modules the route uses: `<if>`/`<else>`, `<for>` (with long-list slicing), `:model`, enter/leave motion, recursive components, `:--name`, `persist`, `load`. Each block body is in the entry; the browser parses one only when it first renders.
- **Loaded later**, never preloaded: the router (prefetch, SPA navigation, view transitions) and `log` load when the browser is idle after the first mount; `call`/`stream` load on the first call; overlays load on the first `open()`, or with the page when it has a modal, alert or dropdown. A link clicked before the router is in is a normal page load.
- Modules used by 2+ routes go to shared chunks in `/assets/chunks/`. `await import()` stays a separate chunk. npm packages stay in `/assets/npm/` (`packages.md`). Asset names are URL-safe: `[id]` becomes `_id_`.
- The page `modulepreload`s every module its entry imports. The router prefetches the HTML and modules of a link on hover, focus, and when the link enters the viewport, also for links rendered after the page loads (`/app/:id` links included). Viewport prefetch starts after the `load` event and the page's first `/_data` and `/_layout` responses, when the browser is idle, and is off when the user turns on Save-Data. The router requests the clean page URL (`/app`, not `/app/index.html`).
- A navigation in the app to a route that needs modules not loaded yet loads them before the new page enters.
- `pragma dev` keeps one file per module; the set of modules is the same.

## Before the runtime

The page paints from prebuilt HTML before its JS runs. In that window:

- Links and forms with `action` work natively.
- A control that only JS can run is `disabled` until its handler is live: a `<button>`, a button-like or choice `<input>`, or a `<select>` with an `@event`, `@open` or `@close`, and the submit button of a `<form @submit>` without `action`. A click there does nothing; it is not replayed later.
- A `disabled` you write yourself (or `:disabled`) stays yours: the runtime never enables it.
- `persist` fields are restored before the first render, so the page never shows the default first.

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
- The first-load JS that `pragma build` prints does not count lazy chunks.
- Show a pending state if the first load can be slow (`let exporting = true` before the `await`).

## Long lists

A render that runs past 12 ms leaves the rest of each outer `<for>` (not one inside another `<for>` item) for a later task, so the browser can paint and answer input in between. Each task creates at least one more item, from where the last one stopped, until the list is complete. `mount` calls share the same 12 ms: the ones left run in the next task, in order. The later tasks run at background priority (`scheduler.postTask`, else `setTimeout`). You add nothing to the markup.

- Items render in list order, except inside a reverse-column scroller ("Scroll that starts at the end" in `css.md`): there the last items render first, so a chat shows its newest messages first.
- `mount` of a component in an item runs in the task that created it or a later one. Do not assume the whole list is on the page in `mount`; read it again on `ResizeObserver` or when you need it.
- Items that a later task adds do not get `data-entering`.

## Page transitions

A navigation by the router swaps pages inside a View Transition. A full page load (a static route, or a click before the router is in) uses the cross-document View Transition. Both look the same: only `<main data-page>` fades in (150 ms, from opacity 0.5); the layout around it (header, sidebar) changes at once and does not blink. To animate another element across pages, give it its own `view-transition-name` only while a transition runs (`:global(html[data-view-transition]) .x { view-transition-name: x; }`): an element with a name is a stacking context and a backdrop root, so a `backdrop-filter` inside it stops blurring. With `prefers-reduced-motion`, there is no transition.

## Updates

After the first render, a change re-renders only what reads it: a `{}`, `:attr`, `:class`, `:show` or `:--name` binding patches itself, and a change read by `<if>`, `<for>`, `:model` or a component prop re-renders only the element that holds that block or component. A change nobody reads does nothing. An event handler does not re-render the page by itself.

## Rules

- Navigate with `<a href>`. A `<button>` that calls `navigate()` gets no prefetch.
- Match skeleton shapes to the final content.
- Set `width` and `height` on every `<img>`. Add `loading="lazy"` below the fold. Store AVIF or WebP images in `public/`.
- Set `font-display: swap` on custom fonts and declare them with `@font-face` in `lib/theme.css`, with the file in `public/` and a root path (`url('/fonts/x.woff2')`). The build serves the font from a hashed, immutable `/assets/` URL and preloads the fonts the first paint of each page uses (`css.md`). Do not write `<link rel="preload" as="font">` in `_layout.html`.
- Give `<for>` a stable `key`.
- Drive per-frame animations (scroll, drag, progress) with `frame` from `'pragma'`: read layout first, then assign state (`state.md`, "Per-frame updates").
- Use per-frame `:--name` values only in `transform` and `opacity`, never in `top`/`left`/`width`/`height`/`margin`/`padding`/`inset` (`css.md`, "Values that change every frame").
- Keep interaction out of the layout when most pages are content: one `@click` or `{}` in `_layout.html` puts the runtime on every route.
- Omit `.js` on pages without interaction.
- Keep heavy single-page code on that page. Load interaction-only code with `await import()` inside the action ("Lazy-load heavy code").
- Return only displayed fields from `load`. Run independent requests with `Promise.all`.
- Keep page JS small. Read the per-route table of `pragma build` after you add a package, a large module, or a directive to the layout.
- Do not import large modules in `lib/state/`. Do not fetch data in `mount`. Do not enable `ssr` by default.
- Do not set the first scroll position in `mount` (`scrollTo`): it runs after the first paint. Open a chat or log at its end with a reverse column (`css.md`, "Scroll that starts at the end").
