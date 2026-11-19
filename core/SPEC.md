# pragma v0: implementation contract

Single source for implementers. Decisions and rationale: `DECISIONS.md`.
Changing this contract requires notifying the coordinator (`@claude-16`).

## 0. Principles

- Plain JavaScript (ES modules). Never TypeScript, never JSDoc types.
- Never HTML in JS: no `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`,
  `createElement`, or template literals with markup. Markup lives in `.html`.
  Exception: the runtime (`core/`) clones `<template>` and applies directives.
- Pages open fast: prebuilt HTML with skeleton, data fetched in parallel.
  Each page downloads only the JS of the modules it uses.
- Deterministic and closed: one way to do each thing; anything not listed here is an error.
- No comments anywhere (app and framework). Code is the documentation: self-explaining names.
  In the app, `pragma check` rejects comments and vague names.
- English everywhere: messages, docs, identifiers.

## 1. pragma repository

```
core/
  DECISIONS.md  SPEC.md  TEAM.md  TESTING.md  README.md
  docs/              LLM docs (copied to <app>/core/docs/)
  runtime/           framework JS (copied to <app>/core/)
    server/          fetch handler, SSR, Node adapter, server logs
  starter/           app skeleton used by `pragma new`
  cli/               Rust workspace
    crates/pragma-diag      shared Diagnostic type
    crates/pragma-css       Tailwind subset + base CSS
    crates/pragma-jslint    JS rules (oxc)
    crates/pragma-compiler  build, analysis, directives
    crates/pragma-cli       `pragma` binary: new, dev, build, check, serve
  example/           example app (generated with `pragma new`)
```

## 2. App structure (fixed)

```
<app>/
  AGENTS.md            agent rules (CLAUDE.md points to it)
  pragma.json          config: { "name", "lang", "budgets": { "initialJsKb", "routeJsKb", "cssKb" } }
  routes/
    _layout.html       root layout; contains <main data-page><slot></slot></main>
    _404.html
    index.html         → /
    index.js           page behavior (optional)
    index.server.js    page data (optional)
    shop/
      index.html       → /shop
      [id].html        → /shop/:id
      [id].js  [id].server.js
      _components/<name>/<name>.html|.js|.css   components for this route only
  lib/
    components/<name>/<name>.html|.js|.css      app components
      modal/  alert/  dropdown/                 from the starter
    state/<name>.js    shared state (store)
    theme.css          global tokens and custom classes
  utils/               pure JS functions
  public/              files copied as-is
  core/                framework (generated; do not edit)
```

Route rules:
- Every `.html` in `routes/` is a route, except names and folders starting with `_`.
- `index.html` serves its folder. `x.html` plus `x/index.html` = build error.
- `[name]` (file or folder) = path param. Static beats param.
- A page file is a fragment: content placed in the `<slot>` of `_layout.html`.
  A `<title>` at the top of the fragment becomes the page title.
- Component use: `<c-name>` = `lib/components/<name>/`; `<p-name>` = `_components/<name>/` in
  the page's folder or any ancestor up to `routes/`. Not found = error.

## 3. HTML syntax (closed list)

`:` binds a value, `.` binds a class, `@` binds an event, `$` is owner state or prop. Condition
and repetition are tags (`<if>`, `<else-if>`, `<else>`, `<for>`, `<empty>`). The build
translates everything to the internal `data-*` the runtime reads (`:text` → `data-text`,
`@click` → `data-on-click`, `:href` → `data-attr-href`, `.x` → `data-class-x`, `$x` →
`state.x` or prop). Writing directive `data-*`, `<template>`, `state.` or `props.` in source is an error.

Paths: only `root.a.b` (no expressions, calls, indexes). Roots: `$x` (owner state or prop),
`data` (`load` result), `shared` (stores), `params` `query` (strings), `page`
(`page.loading`, `page.error`), `<for>` variable.

| Syntax | Where | Effect |
|---|---|---|
| `:text="p"` | element | `textContent` (escaped) |
| `:<attribute>="p"` | element | attribute; `false`/`null`/`undefined` removes; `true` = empty |
| `.<class>="[!]p"` | element | toggles the class |
| `:show="[!]p"` | element | `hidden` when false |
| `:model="$p"` | input/select/textarea | two-way; only `$` or `shared` |
| `@<event>="action"` | element | calls the owner's `actions.action` |
| `@open="id"` `@close` | button | opens/closes overlay (section 6) |

Blocks (path unquoted; the build emits `<template data-if>` etc.):

| Syntax | Effect |
|---|---|
| `<if [!]p>…</if>` | conditional block |
| `<else-if [!]p>…</else-if>` | right after `</if>` or `</else-if>`; any number |
| `<else>…</else>` | right after `</if>` or `</else-if>` |
| `<for x in p key=x.field>…</for>` | repeats; `key` required |
| `<empty>…</empty>` | right after `</for>`: empty list |

Rules: each block has exactly 1 root element. Missing path in dev = error. `!` only in `<if>`,
`<else-if>`, `:show` and `.class`. Inline `style` and `on*` are forbidden. `:checked` and
`:selected` also set the property.

**Lexical owner.** The owner of `@`, `:model` and `$` is whoever wrote the element: in a
component's `.html` with a module, the component; in a page, the page. Slot content belongs to
its author. The build marks each slot-content root that has directives with `data-slot="n"`
(n = how many components up the owner is); the runtime evaluates that element in the scope
outside that component. Directives on the root of a component with a module belong to the component.

### Components

`<c-x>` = `lib/components/x/x.html`; `<p-x>` = `_components/x/x.html` in the page's folder or
any ancestor up to `routes/`. 1 root element. Expanded at build (macro).
- Static prop `title="X"`: build-time literal. Dynamic prop `:total="path"`: evaluated in the
  user's scope and updated each render. Inside: `$title`, `$total` (HTML) and `ctx.$` (JS).
- Props are read-only; missing prop = empty/false; prop named like a state key = error.
- A component with a module does not receive the user's `$` via `:x` (inside, `$` is its own).
- `<slot></slot>` receives children; `<slot name="actions">` receives children with `slot="actions"`.
- The build marks the root with `data-c="x"`. `x.js` loads only on pages using the component.

### Rendered block DOM (client and SSR identical)

The generated `<template>` stays in the DOM. Each instance is inserted as a sibling right after
the template of the branch that produced it, with `data-of="<tid>"` (the `data-tid` of the
`if`/`for` opening the chain) and, in `for`, `data-key="<value>"`. Updates diff by key; existing
nodes are not recreated.

### Compiled format (for SSR)

The build emits `templates/<route>.json`: a list of ops.
```
["html", "<literal>"]
["text", "path"]
["attr", "name", "path"]                    inside the opening tag
["class", "static", [["name","path",neg]]]
["show", "path", neg]
["if", tid, "path", neg, [ops]]
["elif", tid, "path", neg, [ops]]           only if no earlier branch in the chain rendered
["else", tid, [ops]]
["for", tid, "var", "path", "keyPath", [ops]]
["empty", tid, [ops]]                       only if the preceding for had no items
```
Each block op follows its template's html. A chain with a `state`/`shared` path is left entirely
to the client. Instances carry `data-of` and `data-key` as above. A block newly inserted on the
client with `[autofocus]` receives focus.

## 4. Page and component modules (client)

```js
export const state = ctx => ({
  expanded: false,
  get total() { return ctx.shared.cart.items.length },
})
export const actions = {
  toggle(ctx) { ctx.$.expanded = !ctx.$.expanded },
  async save(ctx) { await ctx.call('save', { id: ctx.params.id }) },
}
export function mount(ctx) {
  const timer = setInterval(() => ctx.update(state => { state.now = Date.now() }), 1000)
  return () => clearInterval(timer)
}
```
`ctx`: `$ data shared params query page event el scope log open close confirm navigate call update`
(`$` = owner state + props).
- `state(ctx)` gets the owner's ctx (no event `event`/`el`/`scope`); `ctx.data` exists only
  after load. Derived value = getter on the state object or store, recomputed each render.
- After each action (sync or async) the runtime reapplies directives.
  Outside actions, change state only via `ctx.update(fn)`.
- `mount` is only for external effects (timer, global listener, observer). Returns a cleanup,
  called when leaving the page/removing the component.
- `lib/state/cart.js`: `export default store({ items: [] }, { persist: 'cart' })`
  (`store` from `core/state.js`). Used in templates as `shared.cart.*` and in JS as
  `ctx.shared.cart`. Store getters are derived (not saved to `localStorage`); store methods
  hold mutations shared by several pages.

## 5. Data, loaders and SSR

`routes/shop/[id].server.js`:
```js
export async function load({ params, query, request, env, log }) { return { product } }
export const actions = { async save({ input, params, env, log }) { return { ok: true } } }
export const ssr = false
```
- Default, no SSR: prebuilt HTML arrives with skeleton (`<if page.loading>`).
  The client fetches `GET /_data/<path>?<query>` → `{ "data": ... }` or `{ "error": { "message" } }`.
- In the `<head>` of pages with a loader, the build injects a minimal inline script that starts
  the fetch before modules: `self.__pd = fetch('/_data' + location.pathname + location.search)`.
  The runtime uses `self.__pd` if present.
- `ctx.call('name', input)` → `POST /_action/<path>/<name>` with JSON → action result.
- `ssr = true`: the server runs `load`, renders the compiled template and embeds data in
  `<script type="application/json" id="pragma-data">`. The client binds directives without
  rebuilding the DOM.
- Error in `load` → `page.error` on the client; 500 with ERROR log on the server.

## 6. Overlays: modal, alert, dropdown, popover

Always HTML + JS. Primitive in `core/overlay.js`; components in `lib/components/`.
- Modal and alert: `<dialog>` with `showModal()`. Alert uses `role="alertdialog"`.
- Dropdown and popover: native `popover` attribute.
- Open: `<button @open="about">`. Close: `@close` inside the overlay, Esc, or backdrop click
  (modal). In JS: `ctx.open(id)`, `ctx.close(id)`, `await ctx.confirm(id)` → `value` of the
  clicked `@close` button (`'cancel'` if closed otherwise).
- Back (browser or Android) closes the top overlay, never navigates: opening does `pushState`;
  `popstate` with an open overlay closes it and stops; closing via UI consumes the entry with `history.back()`.
- Modal and alert lock page scroll (`html[data-scroll-lock]` with `overflow:hidden`). If the page
  had a scrollbar, the value is `gutter` and CSS reserves its space (`scrollbar-gutter: stable`);
  without one, nothing is reserved. No layout shift either way.
  Several open = stack; unlocks when the stack empties.
- `core/overlay.js` is imported only on pages with `<dialog>` or `[popover]` (the build
  modulepreloads it only there).
- Focus returns to the opener.

Usage:
```html
<button @open="about">About</button>
<c-modal id="about" title="About the app">
  <p>Content.</p>
  <button slot="actions" @close value="ok">Close</button>
</c-modal>
```

## 7. Router (client)

- Intercepts same-origin `<a>` clicks (without `target`, `download` or `data-reload`).
- Prefetch on `pointerenter`/`focus` and when the link enters the viewport (idle): fetches the
  route's prebuilt HTML and `modulepreload`s its modules (manifest).
- Navigate: fetch HTML, swap the content of `<main data-page>` and `<title>`, unmount the
  previous page (cleanups), import modules and start the loader in parallel.
  The layout is not recreated; its directives are reapplied.
- `popstate`: the overlay manager intercepts first.
- Scroll: top on new navigation; restored on back/forward. Focus moves to `<main>`.
- Links `href="/path"` (no query or hash) to the current route get `aria-current="page"`:
  the build marks prebuilt HTML and the router keeps it on each navigation.
- `document.startViewTransition` when available.
- Manifest: `/_pragma/manifest.json` → `[{ "pattern": "/shop/:id", "html": "/shop/[id].html", "modules": [...], "loader": true, "ssr": false }]`.

## 8. CSS

- `core/base.css`: preflight (Tailwind v4, MIT) + `@layer reset, tokens, base, components, utilities`.
- `lib/theme.css`: tokens (`--color-*`, `--font-*`, `--radius-*`, `--shadow-*`) and custom
  classes in `@layer components`.
- Utilities: closed subset with Tailwind v4 names, generated by `pragma-css` with only used
  classes. No arbitrary values (`w-[37px]` = error). List in `docs/classes.md` (generated by the crate).
- A class in HTML must be a supported utility or defined in project CSS (`lib/theme.css`,
  component `*.css`). Otherwise, check error.
- Custom class: when a combination repeats in 3+ places or needs something outside the subset.
  Component CSS goes in `<name>.css`, scoped with `[data-c="name"]`.
- v0 output: one global minified CSS (lightningcss) with hash.

## 9. Build (`pragma build`)

```
dist/
  client/                 HTML per route, hashed assets, /_pragma/manifest.json
  server/worker.js        Cloudflare Workers entry (if any *.server.js)
  server/node.mjs         Node adapter (node:http) with static files
  server/wrangler.jsonc   assets dist/client, not_found_handling SPA, run_worker_first
  templates/<route>.json  ssr routes only
```
- JS: ES modules, no bundler in v0. Each module minified (oxc) with hashed name; imports
  rewritten. Each page gets `<link rel="modulepreload">` only for what first paint uses:
  the static-import closure (never `import()`), plus `core/overlay.js` if the page has an
  overlay. The manifest uses the same list.
- Client npm packages: prebundled with `rolldown` (tree-shaking, only names the app imports),
  one `/assets/npm/<package>-<hash>.js` per package and shared chunks in `/assets/npm/_*`;
  modulepreloaded like app modules (`docs/packages.md`).
- Dev build adds `<script>self.__pragmaDev=true</script>` to `<head>` (path and action errors in dev).
- `index.html` is not cached; hashed assets are `immutable`.
- `pragma.json` budgets checked at the end of the build.

## 10. Check (`pragma check`)

Errors with file:line:column, rule and message.
- JS (oxc): HTML in JS (section 0); `console.*`; TS or JSDoc types (`@type`, `@param {`,
  `@returns {`, `@typedef`, `@ts-`); import outside the project and `core/`; comment
  (`js/comment`); vague name (`js/vague-name`).
- HTML: unknown or legacy syntax; invalid path root; `<for>` without `key`; malformed block or
  `<template>` in source; misplaced `<else-if>`/`<else>`/`<empty>`; block without single root;
  action missing from the owner module; component not found; inline `style`/`on*`;
  `<script>` in `routes/` and `lib/`.
- CSS: unknown class; arbitrary value.
- Budgets (builds in a temp dir).
- Accessibility warnings (`a11y/*`, non-blocking; `pragma check --strict` makes them errors), on
  the expanded page, pointing at the source file: `img-alt`, `accessible-name` (button or a
  without text, `:text`, `aria-label`/`labelledby` or `title`), `label` (input, select,
  textarea; `type=hidden|submit|reset|button|image` exempt), `dialog-name`,
  `click-non-interactive` (`@click` on a non-interactive element without `role` and
  `tabindex`), `heading` (skipped level), `single-h1` (if/else branches count separately),
  `tabindex` (> 0), `duplicate-id`, `iframe-title`, `a-no-href`.

## 11. Server and dev

- Server code is JS in `fetch(request, env, ctx)` form. Cloudflare Workers is the preferred
  target; Node ≥ 22 via `dist/server/node.mjs`.
- `pragma dev` (Rust, hyper + tokio, port 5180): incremental build in `.pragma/dev/`, watch,
  live reload via SSE at `/_pragma/reload` (CSS swaps without reload). If any `*.server.js`
  exists, starts `node .pragma/dev/server/node.mjs` on an internal port and proxies `/_data`,
  `/_action`, `/_otel` and ssr pages.
- `pragma serve`: same server without watch, over `dist/`. Ctrl+C or SIGTERM also stop the
  child node.
- A path ending in `.html` is never an SSR page (it is the prebuilt HTML the router fetches),
  in `pragma dev/serve` and in the handler.
- `pragma new <dir>`: copies `starter/` + `runtime/` → `<dir>/core/` + `docs/` → `<dir>/core/docs/`
  (embedded in the binary).

## 12. Logs (OpenTelemetry)

- `core/log.js` (client) and `core/server/log.js`: `log.debug|info|warn|error(msg, attrs)`.
  Records follow the OTel Logs Data Model; attributes use semantic conventions.
- Client: captures `error`, `unhandledrejection`, navigation and fetch failures; batches to
  `POST /_otel` (`keepalive`/`sendBeacon` on `visibilitychange`). Every runtime fetch sends
  `traceparent`.
- Server: span per request; `/_otel` forwards to OTLP/HTTP if `OTEL_EXPORTER_OTLP_ENDPOINT`
  is set, else prints OTel JSON to stdout.
- `pragma dev` prints readable logs in the terminal.

## 13. Out of v0

SSG with data (`entries()`), per-route CSS, nested layouts, `[...rest]`, bundler,
Speculation Rules, i18n.

## 14. Build ↔ runtime contract

### Generated HTML per page
```html
<!doctype html>
<html lang="en">                                         from pragma.json "lang" (default "en")
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>…</title>
  <script>try{document.documentElement.dataset.theme=…}catch{}</script>   only if the app uses core/theme.js
  <link rel="stylesheet" href="/assets/app-<hash>.css">
  <script>self.__pd=fetch('/_data'+location.pathname+location.search,{headers:{accept:'application/json'}})</script>
  <link rel="modulepreload" href="/assets/…">            one per page module
  <script type="application/json" id="pragma-route">{…}</script>
  <script type="module" src="/assets/core/boot-<hash>.js"></script>
</head>
<body>…_layout.html content with <main data-page>…page…</main>…</body>
</html>
```
- `__pd` only on pages with a loader. Extra `<head>` tags may come from a `<head>` block at the
  start of `_layout.html`.
- `#pragma-route`: `{ "pattern", "page": "<page module url or null>",
  "components": { "<name>": "<url>" }, "loader": bool, "ssr": bool }`. Only components with `.js`.
- `core/` modules (boot, router, …) are also hashed; imports between them rewritten.

### JS interfaces between runtime files
- `core/boot.js`: entry. Reads `#pragma-route`, starts log and router, mounts the page.
- `core/page.js` imports `core/overlay.js` on demand (`overlays()`) and exports it as `overlay`
  for the router (`overlay?.handlePopState`, `overlay?.closeAll`).
- `core/overlay.js` (owner: overlays): `initOverlays({ log })`, `open(id)`, `close(id)`,
  `confirm(id) → Promise<string>`, `handlePopState(event) → boolean` (true = consumed; the
  router does not navigate).
- `core/log.js` (owner: server/logs): `log` with `debug|info|warn|error(msg, attrs)`,
  `initClientLog({ endpoint: '/_otel', service })`, `traceHeaders() → { traceparent }`.
- `core/state.js` (owner: client runtime): `store(initial, { persist })`.
- Component module owner: lexical (§3). The runtime stores the owner in each element's scope;
  `data-slot="n"` goes up n components. With no component owner, the owner is the page. The
  layout has no module in v0: its directives use `shared`, `params`, `query`.
- `/_pragma/reload` (SSE) and `/_pragma/dev.js` (live reload client) are served by
  `pragma dev`, injected only in dev.

### CSS the runtime expects from `base.css`
`html[data-scroll-lock]{overflow:hidden}`, `html[data-scroll-lock=gutter]{scrollbar-gutter:stable}`, `[hidden]{display:none!important}`,
default `dialog::backdrop` and `.skeleton` (pulsing block honoring `prefers-reduced-motion`).

## 15. Server ↔ build contract

```js
// core/server/handler.js
export function createHandler({ routes }) → { fetch(request, env, ctx) }
// routes: [{ pattern: '/shop/:id', server: <.server.js namespace>, ssr: bool,
//            template: <compiled ops> | null, shell: '<prebuilt page html>' | null }]
```
- Handles `/_data/*`, `/_action/*`, `/_otel` and `ssr` pages. Everything else:
  `env.ASSETS.fetch(request)` (Workers) or the Node adapter's static server.
- The build emits `dist/server/worker.js`: imports `./core/server/handler.js` and each
  `.server.js` (copied to `dist/server/` with relative imports, paths preserved), embeds
  templates and shells as JS objects and does `export default createHandler(...)`.
- The build emits `dist/server/node.mjs`:
  `import h from './worker.js'; import { serve } from './core/server/node.mjs'; serve(h, { root: new URL('../client/', import.meta.url), port: process.env.PORT ?? 3000 })`.
- The Node adapter serves `client/` statics with correct MIME and cache, resolves param routes
  to their HTML via `client/_pragma/manifest.json`, and answers `_404.html` with 404.

## 16. Default palette

Colors defined by `base.css` and generated by `pragma-css` (`text-`, `bg-`, `border-`, `ring-`…):
`white`, `black`, `transparent`, `current` and `{neutral,primary,danger,success,warning}-{50,100,200,300,400,500,600,700,800,900,950}`.
`lib/theme.css` may redefine values and add colors with `--color-<name>` or
`--color-<name>-<shade>`; the generator recognizes them. Dark: `[data-theme="dark"]` on `<html>`
and the `dark:` variant. Switch theme: `setTheme('dark'|'light')` and `theme()` from
`core/theme.js` (stores `localStorage['pragma:theme']`). If any app module imports
`core/theme.js`, the build adds to every page, before the CSS, an inline script restoring the
theme (or following `prefers-color-scheme`), without flash.
