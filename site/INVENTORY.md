# pragma feature inventory (for the "every example of everything" page)

Source of truth: `core/SPEC.md`, `core/docs/*.md`, compiler (`core/cli/crates/pragma-compiler/src/*`),
runtime (`core/runtime/**`). Every example marked **[verified]** passed `pragma check --strict`
(and `pragma build`, plus a headless render where noted) in a throwaway app made with `pragma new`,
using `~/.cargo/bin/pragma` (built 2026-11-18 22:38; only npm `bundle.rs`/`deps.rs` changed after it).
**UNDOCUMENTED** = supported by code, not mentioned in `core/docs/`. **GOTCHA** = works, but in a way
that will surprise you. Bugs are in `BUGS.md` (ids `B1`…).

Global writing rules the compiler enforces (any example you write must obey them):
- No comments anywhere (`<!-- -->`, `//`, `/* */`) → `html/comment`, `js/comment`, `css/comment`.
- Strict HTML: close every non-void tag. Void: `area base br col embed hr img input link meta source track wbr`.
- No `<script>`, `<style>`, `style="..."`, `on*=` attributes, `<template>`, `<html>`, `<body>` in source.
- JS names: no `tmp temp aux foo bar obj arr val res ret fn func cb data info stuff thing handle handler helper util utils misc process test …`,
  no names ending in a digit (except `base64 sha1 sha256 md5 utf8 i18n`), no names < 3 chars except
  `$ i j k id ok el ctx url log` and parameters of expression-body arrows (`list.map(n => n * 2)` is fine; `catch (e)` is not).
- No `console.*` (use `log`), no TypeScript/JSDoc types, no HTML markup in JS strings.

---

## 1. Project structure & CLI

### 1.1 `pragma new <dir>`
Creates an app (dir must be empty or missing). Copies starter + runtime (`core/`) + docs (`core/docs/`).
```sh
pragma new my-site && cd my-site && pragma dev
```
Output: `app created in my-site (43 files)`. Starter ships `routes/{_layout,_404,index}.html`, `routes/index.js`,
`lib/components/{modal,alert,dropdown}`, `lib/theme.css`, `public/favicon.svg`, `AGENTS.md`, `CLAUDE.md`, `.gitignore`.

### 1.2 Fixed layout
```
pragma.json  package.json(optional)  AGENTS.md  CLAUDE.md
routes/  lib/components/<name>/  lib/state/<name>.js  lib/theme.css  utils/  public/  core/
```
- Only `routes/ lib/ utils/ core/ public/` are scanned. Extra top-level *files* (README, this file) are ignored.
- `routes/` may contain only `.html`, `.js`, `.server.js` for pages; anything else must be private (`_name.json`, `_name.server.js`) or it is `app/structure` "unsupported file in routes/".
- A `.js`/`.server.js` without a same-name `.html` → `app/structure` "module without a page".
- Component folder names: `[a-z][a-z0-9-]*`. Store file names: JS identifier. Loose files directly in `lib/components/` → error.

### 1.3 `pragma.json` [verified]
```json
{ "name": "my-site", "lang": "en", "budgets": { "initialJsKb": 15, "routeJsKb": 10, "cssKb": 15 } }
```
- `name` required (used as fallback `<title>`, OTel service name, wrangler worker name). `lang` → `<html lang>`, default `"en"`, must match `[A-Za-z0-9-]+`.
- Budgets are optional; each omitted key = not checked. Unknown keys → `app/pragma-json` error.

### 1.4 Commands
| Command | Notes |
|---|---|
| `pragma dev [--port 5180] [--host 127.0.0.1]` | incremental build into `.pragma/dev/`, watch, live reload (SSE `/_pragma/reload`), CSS hot-swap without reload, starts `node` (≥22) when any `*.server.js` exists and proxies `/_data`, `/_action`, `/_otel`, SSR pages. Budgets are warnings in dev. |
| `pragma check [--strict]` | full prod build into a temp dir; prints `file:line:col error|warning [rule] message` + a code frame; `no problems found` on success. Exit 1 on errors (or on warnings with `--strict`). |
| `pragma build` | writes `dist/`; prints a per-page JS gzip table, CSS gzip, `N page(s), with server`. |
| `pragma serve [--port 3000] [--host 127.0.0.1]` | serves `dist/` as production (brotli/gzip precompressed in memory, ETag, `immutable` for `/assets/`, `no-cache` for HTML) and spawns `node dist/server/node.mjs` if present. |
| `pragma -V` / `pragma --help` | version / help. `--opt=value` and `--opt value` both work. |

- UNDOCUMENTED: in `pragma dev` a failed rebuild shows a full-screen **error overlay in the browser** (file:line:col + rule), dismissable; fixed save hides it.
- UNDOCUMENTED: `pragma dev` prints client `log.*` records (sent to `/_otel`) readably in the terminal.

---

## 2. Routes & layouts

### 2.1 File → URL [verified]
| File | URL |
|---|---|
| `routes/index.html` | `/` |
| `routes/about.html` | `/about` |
| `routes/blog/index.html` | `/blog` |
| `routes/blog/[slug].html` | `/blog/:slug` |
| `routes/[lang]/index.html` | `/:lang` (folder params work) |
| `routes/blog/[slug]/edit.html` | `/blog/:slug/edit` |
- Static segment beats param. `x.html` + `x/index.html` → `app/route-conflict`. Two params in the same position (`[id].html` and `[slug].html` in the same folder) → `app/route-conflict`.
- Param name: `[A-Za-z_][A-Za-z0-9_]*`. No `[...rest]`, no optional params, no nested layouts.
- Files/folders starting with `_` are private (not routes). Only `routes/_layout.html` and `routes/_404.html` are special; any other `_x.html` → error.

### 2.2 Page fragment + `<title>` [verified]
`routes/about.html`
```html
<title>About · My site</title>
<h1>About</h1>
<p>Static content is in the prebuilt HTML.</p>
```
Renders `/about` with `<title>About · My site</title>`. A page has no `<html>/<head>/<body>`. `<title>` must be at the top level of the page (elsewhere → `html/title`), next to the page's `<meta>`/`<link>` head tags. Binding it to `data` needs `ssr = true` (see SEO §5.3).

### 2.3 Layout [verified]
`routes/_layout.html`
```html
<head>
  <title>My site</title>
  <meta name="description" content="Site-wide description">
  <link rel="icon" href="/favicon.svg" type="image/svg+xml">
</head>
<header>
  <nav aria-label="Main">
    <a href="/" class="nav-link">Home</a>
    <a href="/about" class="nav-link">About</a>
  </nav>
</header>
<main data-page><slot></slot></main>
<footer><p>Footer</p></footer>
```
- `<head>` optional, must be first; its children are copied into every page's `<head>` (after `<title>`, before CSS).
- UNDOCUMENTED: a `<title>` inside the layout `<head>` is the **fallback title** for pages without their own `<title>`; final fallback is `pragma.json` `name`.
- Exactly one `<main data-page>` containing only `<slot></slot>` (`html/layout`).
- Layout has no `.js` and no `.server.js`: it can read only `shared`, `params`, `query` (using `data`/`$`/`@action` there → `html/invalid-root` / `html/unknown-action`). For behaviour put a component with a `.js` in the layout.

### 2.4 Active link [verified]
Any routed `<a href="/about">` (no query/hash) to the current page gets `aria-current="page"` in the prebuilt HTML (static routes) and is kept up to date by the router on every navigation (all routes). Style it:
```css
@layer components {
  .nav-link[aria-current="page"] { font-weight: 600; }
}
```
Query-based "active" (tabs/filters): bind `:aria-current` to a getter returning `'true'` or `false`:
```html
<a href="?filter=pending" :aria-current="$filterLink.pending">Pending</a>
```
```js
export const state = ctx => ({
  get filterLink() { return { all: !ctx.query.filter && 'true', pending: ctx.query.filter === 'pending' && 'true' } },
})
```

### 2.5 404 page
`routes/_404.html` (fragment, can have `<title>`). Served with status 404 by `pragma serve`/Node adapter for unknown URLs. UNDOCUMENTED [verified]: `routes/_404.js` is supported (state/actions on the 404 page). `routes/_404.server.js` → error.

### 2.6 Client router (automatic)
- Same-origin `<a href>` clicks are intercepted (not with `target`, `download`, `data-reload`, or modifier keys/middle click).
- Prefetch on hover/focus and when the link enters the viewport (idle): prebuilt HTML + modulepreload of the route modules.
- Swaps only `<main data-page>` and `document.title`; layout stays (its directives re-applied). Uses `document.startViewTransition` when available. Scroll to top on push, restored on back/forward; `#hash` scrolls to the element; focus moves to `<main>`.
- Every navigation (also query-only) recreates page state.
- Full reload: `<a href="/logout" data-reload>Log out</a>`.
- From JS: `ctx.navigate('/blog?tag=rust')`.
- `<head>` is NOT swapped on navigation: only `<title>` changes (fine, since head extras are global anyway).

---

## 3. Dynamic routes

### 3.1 Param route, data from server [verified, SSR rendered via node]
`routes/blog/[slug].html`
```html
<title>Post</title>
<a href="/blog">← Blog</a>
<if page.loading>
  <p class="skeleton h-8 w-64"></p>
</if>
<else-if page.error>
  <p role="alert" :text="page.error.message"></p>
</else-if>
<else-if data.post>
  <article>
    <h1 :text="data.post.title"></h1>
    <p :text="data.post.summary"></p>
  </article>
</else-if>
<else>
  <h1>Post not found</h1>
</else>
```
`routes/blog/[slug].server.js`
```js
import posts from './_posts.json' with { type: 'json' }

export async function load({ params }) {
  return { post: posts.find(post => post.slug === params.slug) ?? null }
}
```
- `params.slug` (HTML) / `ctx.params.slug` (JS) / `params.slug` in `load` — always strings (URL-decoded).
- `params.x` that the route does not have → `html/invalid-root` with the list of valid params.
- "Not found" is data (`{ post: null }`); the response status stays 200 (soft 404 — see SEO §5.6).
- Static hosting: param routes need a server that maps `/blog/abc` → `/blog/[slug].html` (`pragma serve` and the Node adapter do it via the manifest). Workers: see B1.

### 3.2 Query string
`query.tag` (HTML), `ctx.query.tag` (JS), `query` in `load`/actions — plain object of strings; repeated keys keep the last value. Missing `query.x` never throws in dev (other missing paths do).

---

## 4. Server data, server actions, SSR

### 4.1 `load` (client-fetched, default) [verified]
`routes/shop/index.server.js`
```js
export async function load({ params, query, request, env, log }) {
  const [products, featured] = await Promise.all([listProducts(query.category), listFeatured()])
  log.info('catalog loaded', { 'product.total': products.length })
  return { products, featured }
}
```
- Return value → `data` (HTML) / `ctx.data` (JS, only after load; read via getters inside the `<else>` of `page.loading`).
- Build injects in `<head>`: `<script>self.__pd=fetch('/_data'+location.pathname+location.search,...)</script>` so the fetch starts before modules.
- Endpoint: `GET /_data/<path>?<query>` → `{"data": ...}` or `{"error":{"message"}}` (`cache-control: no-store`).
- Throw in `load` → client `page.error.message`, HTTP 500, ERROR log.
- `data.*` in HTML without a `load` export → `html/invalid-root` "data requires a load in routes/x.server.js".
- `request` is the Fetch `Request` (headers, cookies). `env`: Workers bindings; on Node it is `process.env` (+`ASSETS`) — UNDOCUMENTED.
- GOTCHA: prebuilt HTML contains the `<template>` of the `page.loading` chain, not the rendered skeleton; the skeleton appears once boot JS runs (see B9). Content outside blocks is in the HTML immediately.

### 4.2 Private server helpers & JSON
```
routes/shop/_catalog.server.js     helper, import only from .server.js files
routes/shop/_products.json         import products from './_products.json' with { type: 'json' }
```
Client module importing any `*.server.js` → `js/import` "client module cannot import ... (server code)".
`node:*` imports allowed only in `*.server.js` (and `core/server/`).

### 4.3 Server actions [verified on node]
`routes/blog/[slug].server.js`
```js
const likes = new Map()

export const actions = {
  async like({ input, params, query, request, env, log }) {
    const total = (likes.get(params.slug) ?? 0) + (input?.step ?? 1)
    likes.set(params.slug, total)
    log.info('post liked', { 'post.slug': params.slug })
    return { likes: total }
  },
}
```
`routes/blog/[slug].js`
```js
export const state = () => ({ sending: false, error: '' })

export const actions = {
  async like(ctx) {
    ctx.$.sending = true
    try {
      ctx.data.post.likes = (await ctx.call('like', { step: 1 })).likes
    } catch (failure) {
      ctx.log.error('post like failed', { 'post.slug': ctx.params.slug, 'exception.message': failure.message })
      ctx.$.error = 'Could not save right now.'
    } finally {
      ctx.$.sending = false
    }
  },
}
```
- `ctx.call(name, input)` → `POST /_action/<current path>/<name>` with JSON body; resolves with the JSON result; non-2xx → throws `Error(message)`.
- Unknown action → 404 `{"error":{"message":"Unknown action: x"}}`; non-JSON body → 400; non-POST → 405.
- Documented (`data.md`): actions also receive `query`, `request`, `url`, `cookies`, `locals`, `setHeaders`, `waitUntil`; an action may `return new Response(...)` (status, headers, streaming body pass through).
- UNDOCUMENTED: `ctx.call` is also available in component modules; it always targets the **current page's** `.server.js`.
- A `.server.js` without `.html` is an endpoint (`GET`/`POST`/... returning a `Response`): files, streams, RSS, sitemap.xml, webhooks, WebSockets (Workers only). `routes/_middleware.server.js` runs before every request.

### 4.4 SSR opt-in [verified: node returns rendered HTML + `#pragma-data`]
Add to the page's `.server.js`:
```js
export const ssr = true
```
- Must be literally `export const ssr = true`.
- The server runs `load`, renders the page's compiled ops into `<main data-page>` of the prebuilt shell, removes the `__pd` script and appends `<script type="application/json" id="pragma-data">{"data":...}</script>` before `</body>`. Client binds without re-rendering. Skeleton is never shown (`page.loading` is false on the server).
- Only `data`, `params`, `query`, `page` and `<for>` vars render on the server; any `if/for` chain whose path is `$…`/`shared…` is left to the client.
- Build emits `dist/templates/<route>.json` (ops) and embeds template + shell into `dist/server/worker.js`.
- Headers: `content-type: text/html`, `cache-control: private, no-cache`. HEAD supported.
- Error in `load` → status 500, shell without rendered body, client shows `page.error`.
- The server renders the `<main>` content and the page's bound head tags (`<title :text>`, `<meta :content>`, `<link :href>`); the rest of `<head>` is the static shell — see SEO §5.3.
- Client navigation to an SSR route fetches the SSR HTML from the server (not the prebuilt file).

---

## 5. SEO / Open Graph [verified; `core/SPEC.md` §5.1, `core/docs/routes.md` "SEO and link previews"]

**Summary: per-page head works. A page writes `<title>`, `<meta>` and `<link>` at its top level; the build moves them into `<head>` and replaces the layout tag with the same key. With `ssr = true` they can be bound to `data`, `params` or `query`, so crawlers and link-preview bots (no JS) get the real values in the first response. Still impossible: JSON-LD (no `<script>`) and setting the HTTP status from `load`. (B2, B3, B4 and BUG-S-2 are fixed.)**

### 5.1 Site-wide defaults in the layout `<head>` [verified]
`routes/_layout.html`
```html
<head>
  <meta name="description" content="Native HTML, CSS and JS apps.">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="My site">
  <meta property="og:title" content="My site">
  <meta property="og:description" content="Native HTML, CSS and JS apps.">
  <meta property="og:image" content="https://example.com/og.png">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="theme-color" content="#111111">
  <link rel="icon" href="/favicon.svg" type="image/svg+xml">
</head>
<main data-page><slot></slot></main>
```
- The layout `<head>` is static: any directive there → `html/head`.
- Never put `canonical` or `og:url` in the layout: every page would claim the same URL. Set them per page.
- `<script type="application/ld+json">` cannot be written (`html/script`).

### 5.2 Per-page static head [verified]
`routes/pricing.html`
```html
<title>Pricing · My site</title>
<meta name="description" content="Plans and prices.">
<link rel="canonical" href="https://example.com/pricing">
<meta property="og:url" content="https://example.com/pricing">
<meta property="og:title" content="Pricing">
<h1>Pricing</h1>
```
Output: charset, viewport, `<title>`, the layout head tags with the page's tags replacing theirs in place (`meta` keyed by `name`, `property` or `http-equiv`; `link` by `rel` + `hreflang`), then the page's new tags (here canonical and `og:url`) appended. No title at all → `pragma.json` `name`.
- `<meta>`/`<link>` anywhere else in a page, or in a component → `html/head`.
- This site: every section page sets description, canonical, `og:url`, `og:title` and `og:description`; every iframe demo route under `*/demo/*` sets `<meta name="robots" content="noindex">` (except the seo demo, which models indexable articles).

### 5.3 Per-page head from server data (needs `ssr = true`) [verified]
`routes/blog/[slug].html` + `[slug].server.js` with `export const ssr = true`
```html
<title :text="data.post.title">Blog · Shop</title>
<meta name="description" :content="data.post.summary">
<meta name="robots" :content="data.robots">
<link rel="canonical" :href="data.url">
<meta property="og:title" :content="data.post.title">
```
- Only `:text` on `<title>` and `:<attr>` on `<meta>`/`<link>`, rooted in `data`, `params` or `query`; a binding without `ssr = true` → `html/head`.
- The server renders them, escaped, before sending. Null/false keeps the static fallback; a `<meta>` left without `content` or a `<link>` without `href` is removed (so `robots` can appear only for a missing entity; see `routes/examples/seo/demo/[slug]`).
- The router replaces `<title>` and every `<meta>`/`<link>` except charset, viewport, stylesheet and modulepreload with the target document's on each client navigation.
- `load` sets headers (`setHeaders`), cookies, status (`error()` from `'pragma/server'`) and redirects (`redirect()`).

### 5.4 What SSR does buy for SEO
With `ssr = true`, the bound head tags and the `<main>` content (headings, text, links, lists from `data`) are in the first HTML response; without SSR, only content outside `<if>/<for>` blocks is in the HTML (initial text inside `:text` elements is kept, e.g. `<h1 :text="data.x">Fallback</h1>` ships "Fallback").

### 5.5 Other SEO-relevant facts
- `<html lang>` from `pragma.json` `lang`. Viewport + charset meta always emitted.
- `robots.txt`, `sitemap.xml`, `og.png`, `manifest.webmanifest` → put static files in `public/` (copied to site root; `.webmanifest` gets the right MIME). A `public/` file that collides with a build output → `app/structure`. No dynamic sitemap/RSS (no non-HTML routes). This site: `public/robots.txt` with a `Sitemap:` line and a hand-kept `public/sitemap.xml` of every indexable page.
- Missing entity on a dynamic route returns HTTP 200 (soft 404): `load` cannot change the status; only unknown *routes* get 404 (Node, `pragma serve` and Workers). Mitigate with a bound `robots` `noindex` (§5.3).
- HTML is `no-cache`, hashed assets `immutable`; brotli/gzip by `pragma serve`.

---

## 6. HTML syntax & directives (closed list)

Roots usable in any path: `$x` (owner state/prop), `data`, `shared.<store>`, `params`, `query`, `page.loading|page.error`, `<for>` variable. Paths are `root.a.b` only (identifiers; `$items.length` works since it is a property). No expressions, calls, indexes, operators, `{{ }}`. `!` only in `<if>`, `<else-if>`, `:show`, `.class`.

All examples in §6 were [verified] together in one page (`routes/play.html` + `play.js`) and rendered headless.

| Directive | Example | Effect |
|---|---|---|
| `:text` | `<h1 :text="$heading">Fallback</h1>` | textContent (escaped). Fallback text ships in HTML. |
| `:<attr>` | `<a :href="$link" :title="$heading">` | sets attribute; `false/null/undefined` removes; `true` → empty attr |
| `:disabled` | `<button :disabled="$busy">` | boolean attribute |
| `:aria-*` | `<button :aria-pressed="$pressedText">` | return `'true'`/`'false'` strings (boolean `false` removes the attribute) |
| `:checked` `:selected` | `<input type="checkbox" :checked="task.done">` | attribute **and** property |
| `:data-*` | `<article :data-tone="$tone">` | UNDOCUMENTED but valid: dynamic data attribute, a styling hook for component CSS (`[data-c="x"][data-tone="ok"]`) |
| `.class` | `<li .line-through="item.done" .font-semibold="!$bold">` | toggles the class; variants work: `.md:font-bold="$bold"`; class must be a known utility/project class |
| `:show` | `<p :show="!$hidden">` | toggles `hidden` (element stays in DOM) |
| `:model` | `<input :model="$name">` | two-way; only `$x` or `shared.x`; only on `input/select/textarea` |
| `@event` | `<input @keydown="onKey" @input="typed">` | calls owner `actions.name(ctx)`; any lowercase event name (`@dblclick`, `@focusout`, `@change`) |
| `@submit` | `<form @submit="save">` | `preventDefault()` automatic |
| `@open="id"` | `<button type="button" @open="about">` | opens overlay `id` |
| `@close` | `<button @close value="ok">` | closes the enclosing overlay; `value` → `ctx.confirm` result |

`:model` details (runtime `directives.js`): checkbox ↔ boolean; radio ↔ `value` of the checked one (`<input type="radio" name="size" value="s" :model="$size">`); `type=number|range` ↔ number or `null` when empty; select/textarea/text ↔ string.

Forbidden in attributes: `:style`, `:on*` (`html/on-inline`), `style=`, `on*=`. Legacy `data-text`, `data-on-*`, `data-attr-*`, `data-class-*`, `data-with-*`, `state.x`, `props.x` → `html/legacy-syntax` with the fix. Writing `data-c|tid|of|slot` → `html/unknown-directive`. Markers you may write: `data-page` (layout `<main>` only), `data-reload` (`<a>`).

GOTCHAS:
- `.bg-primary-500/50="$x"` is rejected (`/` ends the attribute name; error `html/invalid-path empty path`). Use a custom class.
- `:class="$x"` passes `check` but replaces the whole class attribute and the classes it produces are NOT generated into the CSS (B5). Use `.class` toggles.
- `{{ x }}` / `${ x }` are not rejected (docs say they are) — rendered as literal text (B6).

---

## 7. Blocks (`<if>`, `<else-if>`, `<else>`, `<for>`, `<empty>`) [verified]

```html
<if $loading>
  <p>Loading</p>
</if>
<else-if !$items.length>
  <p>No items yet</p>
</else-if>
<else>
  <ul>
    <for group in $groups key=group.id>
      <li>
        <h2 :text="group.name"></h2>
        <ul>
          <for item in group.items key=item.id>
            <li .line-through="item.done" @click="pick" role="button" tabindex="0" :text="item.label"></li>
          </for>
          <empty>
            <li>Empty group</li>
          </empty>
        </ul>
      </li>
    </for>
  </ul>
</else>
```
```js
export const actions = {
  pick(ctx) { ctx.scope.item.done = !ctx.scope.item.done },
}
```
Rules (with the error you get):
- Path unquoted: `<if page.loading>`, `<if !$open>`; `<if test="x">` → `html/block`.
- `key` required (`html/for-without-key`); `key=item.id` and `key="item.id"` both accepted; key must start with the loop variable; keys must be unique (dev throws on duplicates; SSR throws).
- Exactly 1 root element per block, and it cannot be another block (`html/block-single-root`). Put directives on that inner element, not on the block tag.
- `<else-if>`/`<else>` right after `</if>`/`</else-if>` (whitespace ok, an element in between → `html/misplaced-else`); `<empty>` right after `</for>` (`html/misplaced-empty`).
- Loop variable cannot be a root name (`data`, `state`, `shared`, `params`, `query`, `page`, `props`, `$`).
- In actions, the item is `ctx.scope.<var>` (`ctx.scope.item`); nested loops expose both vars.
- An element with `autofocus` inside a newly inserted block gets focus.
- Rendered DOM: `<template data-tid>` stays; instances follow it with `data-of` (+ `data-key` in `for`). Updates diff by key.
- UNDOCUMENTED: inside a component, `<if $prop>` on a **static** prop is decided at build time (no template emitted): `<c-badge label="New" icon>` with `<if $icon>…</if>` ships the branch inline.

---

## 8. Components

### 8.1 Stateless component, static + boolean props, default slot fallback [verified]
`lib/components/badge/badge.html`
```html
<span class="rounded-full px-2 text-xs" .bg-primary-100="$active" :title="$label">
  <span :text="$label"></span>
  <if $icon>
    <span aria-hidden="true">*</span>
  </if>
  <slot>Default badge text</slot>
</span>
```
Usage:
```html
<c-badge label="New" active icon class="ml-2"></c-badge>
<c-badge label="Plain" active="false">Custom text</c-badge>
```
Output (prebuilt):
```html
<span data-c="badge" class="rounded-full px-2 text-xs bg-primary-100 ml-2" title="New"><span>New</span> <span aria-hidden="true">*</span> Default badge text</span>
<span data-c="badge" class="rounded-full px-2 text-xs" title="Plain"><span>Plain</span> Custom text</span>
```
- Static props are build-time literals; a bare attribute (`active`) is truthy; the string `"false"` is falsy (UNDOCUMENTED).
- UNDOCUMENTED: `<slot>` children are fallback content when the caller passes nothing.
- `class` on the tag is appended to the root's classes. `id` is just a prop: apply it with `:id="$id"`.
- Exactly 1 root element; root cannot be `<template>`, `<slot>` or a component (`html/component`).

### 8.2 Component with `.js` (state per instance) [verified]
`lib/components/toggle/toggle.html`
```html
<button type="button" class="btn" :id="$id" @click="flip" :aria-pressed="$pressed" :text="$label"></button>
```
`lib/components/toggle/toggle.js`
```js
export const state = ctx => ({
  on: ctx.$.initial === 'on',
  get pressed() { return String(this.on) },
})

export const actions = {
  flip(ctx) { ctx.$.on = !ctx.$.on },
}
```
Usage: `<c-toggle id="dark-toggle" label="Dark" initial="on"></c-toggle>`
- In JS every prop is a string from the root's `data-*` (`ctx.$.initial === 'on'`). GOTCHA: a bare boolean prop (`<c-toggle locked>`) is `''` (falsy) in JS but truthy in HTML (B7).
- The module loads only on pages using the component; component CSS is only included if a page uses it.
- Prop named like a state key → the state key hides it; a getter reading `ctx.$.<same name>` recurses forever.

### 8.3 Dynamic props (`:prop`)
```html
<for p in data.products key=p.id>
  <p-product-card :product="p"></p-product-card>
</for>
```
Inside a component **without** `.js`: `$product.name` resolves to the caller's path (any depth). Inside a component **with** `.js`: `$product` works in HTML; `ctx.$.product` is the attribute string (`"[object Object]"` for objects) → pass ids/scalars (`:id="p.id"`). A component with `.js` cannot receive the caller's `$x` (`html/component`): pass `data.`, `shared.` or a `<for>` item.

### 8.4 `:model` pass-through (UNDOCUMENTED pattern used by the example app)
`lib/components/search/search.html` (no `.js`):
```html
<input type="search" class="field w-full sm:w-64" autocomplete="off" :placeholder="$placeholder" :aria-label="$label" :model="$value">
```
Usage on a page with `state = () => ({ search: '' })`:
```html
<c-search label="Search products" placeholder="Search…" :value="$search"></c-search>
```
Typing updates the page's `$search` (the dynamic prop is an alias to the caller path).

### 8.5 Named slots, slot ownership [verified]
`routes/_components/site-note/site-note.html`
```html
<aside class="card"><slot></slot><footer><slot name="foot"></slot></footer></aside>
```
```html
<p-site-note>
  <p>Body</p>
  <button slot="foot" type="button" @click="hello">Hello</button>
</p-site-note>
```
- `hello` is the **page's** action (slot content belongs to its author; build adds `data-slot="1"`).
- Filling a slot the component lacks → `html/component` "does not accept slot \"x\"". `slot=` only on direct children of `<c-…>/<p-…>`.
- In a component without `.js`, `@x` calls the caller's action.

### 8.6 Route-local components (`p-`) [verified]
`routes/blog/_components/post-meta/post-meta.html` → `<p-post-meta author="Ana" date="2026-11-18"></p-post-meta>` in any page under `routes/blog/`. Lookup walks up from the page folder to `routes/` (`routes/_components/x/` is visible to every route). `<p-x>` outside `routes/` → error. Promote = move to `lib/components/` and rename `p-`→`c-`. Two components with the same folder name on one page → `html/component`. Components cannot have `.server.js`. Self-use (cycle) → error.

---

## 9. State, actions, mount, stores

### 9.1 Page module [verified]
```js
export const state = ctx => ({
  draft: '',
  get visible() { return ctx.shared.tasks.items.filter(task => ctx.query.filter !== 'pending' || !task.done) },
})

export const actions = {
  add(ctx) { ctx.shared.tasks.add(ctx.$.draft.trim()); ctx.$.draft = '' },
}

export function mount(ctx) {
  const timer = setInterval(() => ctx.update(current => { current.now = Date.now() }), 1000)
  return () => clearInterval(timer)
}
```
- After each action (sync or async; also after the promise settles) the runtime re-applies all directives. Outside actions use `ctx.update(fn)`.
- `@x` naming an action not exported → `html/unknown-action` (checked per owner).
- `$x` on a page without `.js` → `html/invalid-root`.

### 9.2 `ctx`
`$ data shared params query page event el scope log open close confirm navigate call update`. `ctx.el` in actions = the element with the `@event`; in `state`/`mount` = owner root (page: `<main data-page>`). `ctx.state`/`ctx.props` → `js/ctx` error.

### 9.3 Store [verified]
`lib/state/prefs.js`
```js
import { store } from '../../core/state.js'

export default store({
  name: '',
  get greeting() { return this.name ? `Hello, ${this.name}` : 'Hello' },
  rename(next) { this.name = next },
}, { persist: 'prefs' })
```
HTML: `<p :text="shared.prefs.greeting"></p>`, `<textarea aria-label="Name" :model="shared.prefs.name"></textarea>`. JS: `ctx.shared.prefs.rename('Ana')`.
- `persist` saves to `localStorage['pragma:<key>']` after each render, getters excluded.
- UNDOCUMENTED: persisted stores sync across tabs (`storage` event).
- `shared.x` with no `lib/state/x.js` → `html/invalid-root`.
- GOTCHA/BUG: a store used **only from JS** (`ctx.shared.prefs` in a getter) and never referenced as `shared.prefs` in that page's HTML (nor imported by its modules) is not loaded on that page → `TypeError` at runtime, while `check` passes (B8). Workaround: `import prefs from '../lib/state/prefs.js'` in the page module, or reference `shared.prefs.*` in the HTML.

---

## 10. Overlays (modal, alert, dropdown) [starter, verified]

```html
<button type="button" class="btn" @open="about">About</button>
<c-modal id="about" title="About the app">
  <p>Content.</p>
  <button slot="actions" type="button" class="btn btn-primary" @close value="ok">Close</button>
</c-modal>

<button type="button" class="btn" @click="remove">Delete</button>
<c-alert id="remove" title="Delete project?">
  <p>This cannot be undone.</p>
  <button slot="actions" type="button" class="btn" @close value="cancel" autofocus>Cancel</button>
  <button slot="actions" type="button" class="btn btn-danger" @close value="ok">Delete</button>
</c-alert>

<c-dropdown id="account" label="Account">
  <a href="/profile">Profile</a>
  <hr>
  <button type="button" @close @click="logout">Log out</button>
</c-dropdown>
```
```js
export const actions = {
  async remove(ctx) {
    if (await ctx.confirm('remove') !== 'ok') return
    await ctx.call('remove', { id: ctx.params.id })
  },
  logout(ctx) { ctx.navigate('/') },
}
```
| | Props | Slots | Closes on | Scroll lock |
|---|---|---|---|---|
| `c-modal` (`<dialog>`) | `id`, `title` | default, `actions` | Esc, backdrop, ×, `@close`, Back | yes |
| `c-alert` (`<dialog role="alertdialog">`) | `id`, `title` | default, `actions` | Esc, `@close`, Back (not backdrop) | yes |
| `c-dropdown` (`[popover]`) | `id`, `label` | default | outside click, Esc, `@close`, Back | no |
- `ctx.open(id)`, `ctx.close(id)`, `await ctx.confirm(id)` → clicked `@close` `value` or `'cancel'`.
- Back closes the top overlay (overlays push a history entry), overlays stack, focus returns to the opener, scrollbar gutter reserved only if there was a scrollbar. Dropdown is positioned under (or above) its button.
- `core/overlay.js` loads only on pages containing `<dialog>`/`[popover]`.
- These are ordinary editable components in `lib/components/` (you can restyle them).

---

## 11. Theme & CSS

### 11.1 Theme toggle [verified, also with `/core/theme.js` import]
`lib/components/theme-switch/theme-switch.js`
```js
import { theme, setTheme } from '../../../core/theme.js'

export const state = () => ({
  dark: theme() === 'dark',
  get label() { return this.dark ? 'Light theme' : 'Dark theme' },
  get pressed() { return String(this.dark) },
})

export const actions = {
  toggle(ctx) {
    ctx.$.dark = !ctx.$.dark
    setTheme(ctx.$.dark ? 'dark' : 'light')
  },
}
```
`theme-switch.html`: `<button type="button" class="btn" @click="toggle" :aria-pressed="$pressed" :text="$label">Dark theme</button>`
- If any module imports `core/theme.js`, every page gets an inline no-flash script in `<head>` restoring `localStorage['pragma:theme']` or following `prefers-color-scheme`. Without that import, dark mode never activates (no media-query fallback).
- UNDOCUMENTED: imports may use the absolute form `'/core/theme.js'` (lint allows `/core/…`).

### 11.2 Tokens & custom classes (`lib/theme.css`) [verified]
```css
@layer tokens {
  :root {
    --color-brand: oklch(0.7 0.15 150);
    --color-accent-500: oklch(0.6 0.2 30);
  }
  [data-theme="dark"] {
    --color-brand: oklch(0.6 0.12 150);
  }
}

@layer components {
  .nav-link[aria-current="page"] { font-weight: 600; }
}
```
- `--color-brand` → `bg-brand`, `text-brand`…; `--color-accent-500` → `text-accent-500`… [verified].
- Redefinable: `--color-*`, `--font-{sans,serif,mono}`, `--radius-*`, `--shadow-*`, `--spacing`. A new font name (`--font-display`) does NOT create `font-display` (`css/unknown-class`) [verified].
- UNDOCUMENTED tokens in `base.css`: `--page-bg`, `--page-fg` (html background/text, switch in dark), `--skeleton-bg`, `--container-*`.
- Starter tokens/classes: `bg-surface`, `text-ink`, `text-muted`, `border-line` (theme-aware), `.card`, `.btn`, `.btn-primary`, `.btn-danger`, `.dialog`. (Example app adds `.btn-icon`, `.field`, `.nav-link`.)
- Component CSS: `lib/components/x/x.css`, scope with `[data-c="x"]`; wrapped in `@layer components` automatically.
- Forbidden: `style=`, arbitrary values `w-[37px]` (`css/arbitrary-value`), negatives `-mt-2`, unknown classes, `!important`, ID selectors, external `@import` (the last three are doc rules).
- One global hashed, minified CSS (lightningcss) with only used utilities.

### 11.3 Utility subset
Full list in `core/docs/classes.md` (generated). Variants: `hover: focus: focus-visible: active: disabled: group-hover: dark: sm: md: lg: xl: 2xl:`, combinable (`md:hover:bg-primary-600`), opacity `bg-primary-500/50` [verified]. Palette: `white black transparent current` + `{neutral,primary,danger,success,warning}-{50…950}`. Not supported e.g. `shadow-inner` [verified]. `skeleton` and `group` are recognized classes.

---

## 12. npm packages & tree shaking
(Examples from `core/cli/xtask/packages/app`, covered by `packages.test.mjs`.)
```sh
npm i date-fns --ignore-scripts
```
`package.json` → `"dependencies": { "date-fns": "4.4.0" }`
```js
import { format } from 'date-fns'

export const state = () => ({ formattedDate: format(new Date(2024, 0, 15), 'yyyy-MM-dd') })
```
- Client packages are prebundled with rolldown: one `/assets/npm/<pkg>-<hash>.js` per package (`@date-fns/tz` → `date-fns__tz`), shared chunks `/assets/npm/_*.js`, modulepreloaded, counted in budgets. Only imported names ship; subpaths share the package entry (`date-fns/format` + `date-fns/locale`).
- CommonJS works but ships whole (`lodash/groupBy`, `dayjs` + `dayjs/plugin/relativeTime`).
- `import * as x` / `import('pkg')` ship the whole package. Template-literal specifier → `js/import`.
- Errors: not in `dependencies` → `js/import`; UI frameworks (React, Vue, Svelte, Solid, lit, jQuery, Alpine…) → `js/import`; TS/JSX/.vue/CSS/assets or Node built-ins on the client → `app/package`; unknown export → `js/import`; remote `https://` imports → `js/import`.
- `process.env.NODE_ENV` → `"production"`/`"development"`.
- Server (`*.server.js`): any dependency incl. `node:*`, not bundled; `dist/server/package.json` has `"type":"module"`.
- Recommended: `Intl.*`, `date-fns`, `@date-fns/tz`, `valibot`, `fuse.js`, `chart.js` (draw into a `<canvas>` from `mount`), `es-toolkit`. Avoid: `moment`, `lodash`, `axios`, `uuid`, `nanoid`, HTML-building libs.

chart.js pattern (canvas in HTML, chart in `mount`):
```html
<canvas id="sales-by-month" width="480" height="240" role="img" aria-label="Sales by month"></canvas>
```
```js
import { BarController, BarElement, CategoryScale, Chart, LinearScale } from 'chart.js'

Chart.register(BarController, BarElement, CategoryScale, LinearScale)

export function mount(ctx) {
  const salesChart = new Chart(ctx.el.querySelector('#sales-by-month'), {
    type: 'bar',
    data: { labels: ['Jan', 'Feb', 'Mar'], datasets: [{ label: 'Sales', data: [12, 19, 7] }] },
  })
  return () => salesChart.destroy()
}
```

---

## 13. Logs (OpenTelemetry)
```js
ctx.log.info('order saved', { 'order.id': ctx.params.id })
```
- Client: `ctx.log` in actions/mount; `import { log } from '../core/log.js'` in `utils/`. Batches to `POST /_otel` every 2 s / 50 records, `sendBeacon` on hide. Auto: `error`, `unhandledrejection`, `navigation` (info, with duration), `loader failed`, `navigation failed`. Every runtime fetch sends `traceparent`. In dev every `log.*` also prints to the browser console. Client minimum level is `info` (debug only reaches the console in dev).
- Server: `log` in `load`/actions; one record per request (`GET /blog/:slug` with method, route, status, duration). `/_otel` forwards to OTLP/HTTP JSON if `OTEL_EXPORTER_OTLP_ENDPOINT` is set, else OTel JSON lines on stdout.
- UNDOCUMENTED env vars: `PRAGMA_LOG_LEVEL` (default `info`; so server `log.debug` is dropped in dev too — B10), `OTEL_EXPORTER_OTLP_LOGS_ENDPOINT`, `OTEL_EXPORTER_OTLP_HEADERS`, `OTEL_SERVICE_NAME`, `OTEL_RESOURCE_ATTRIBUTES`. `/_otel` caps batches at 256 KB.

---

## 14. Performance (built in)
- Prebuilt HTML per route; `load` fetch starts in `<head>`; `modulepreload` for exactly the page's static import closure (+ `core/overlay.js` only with overlays); `import()` modules not preloaded.
- Hashed, minified modules (oxc), no app bundler; per-component modules; CSS only used classes.
- Link prefetch (hover/focus/viewport); view transitions; uncached HTML + immutable assets; brotli/gzip in `pragma serve`.
- Budgets (gzip kB) enforced by `check`/`build` (`app/budget`, error in prod, warning in dev). Measured: a page with no JS of its own ≈ 7.2–7.5 kB gzip total (runtime).
- Heavy code used after an interaction: `const { parse } = await import('./heavy.js')` inside the action.

---

## 15. Checks: lint, a11y, structure
JS: `js/syntax js/html-in-js js/console js/jsdoc-types js/import js/comment js/vague-name js/ctx`.
HTML: `html/syntax html/comment html/legacy-syntax html/unknown-directive html/invalid-path html/invalid-root html/block html/block-single-root html/for-without-key html/misplaced-else html/misplaced-empty html/outside-template html/unknown-action html/component html/unknown-component html/layout html/title html/script html/style-inline html/on-inline`.
CSS: `css/syntax css/comment css/unknown-class css/arbitrary-value`. App: `app/structure app/route-conflict app/package app/pragma-json app/budget`.
A11y warnings (errors with `--strict`), on the expanded page: `a11y/img-alt`, `a11y/accessible-name` (text, `:text`, `aria-label`, `aria-labelledby`, `title`, img alt, labelled svg), `a11y/label` (`<label for>`, wrapping `<label>`, `aria-label(ledby)`; `hidden submit reset button image` exempt), `a11y/dialog-name`, `a11y/click-non-interactive` (`@click` on non-interactive needs static `role` + `tabindex`), `a11y/heading`, `a11y/single-h1` (if/else branches separate), `a11y/tabindex` (>0), `a11y/duplicate-id`, `a11y/iframe-title`, `a11y/a-no-href`. `:<attr>` counts as the attribute.
Example output [verified]:
```
utils/x.js:4:39 error [js/vague-name] name `e` does not say what it is or does; ...
routes/css.html:3:4 error [css/arbitrary-value] arbitrary value "w-[37px]" is not supported; ...
```

---

## 16. Deploy targets
`dist/` layout [verified]:
```
dist/client/            <route>.html per page (blog/[slug].html), _404.html, assets/, _pragma/manifest.json, public/ files
dist/server/worker.js   createHandler({ service, routes: [{ pattern, server, ssr, template, shell }] })
dist/server/node.mjs    node adapter (PORT env, default 3000)
dist/server/wrangler.jsonc  { name, main: "worker.js", compatibility_date: "2025-11-01", assets: { directory: "../client", binding: "ASSETS", not_found_handling: "single-page-application", run_worker_first: true } }
dist/server/package.json    { "type": "module" }
dist/server/core/…, dist/server/routes/**/*.server.js (copied, relative imports kept)
dist/templates/<route>.json  SSR routes only
```
- Static only (no `*.server.js`): deploy `dist/client/`. Host must map `/about` → `about.html` and param routes `/blog/x` → `blog/[slug].html` (generic static hosts cannot do the latter); unknown → `_404.html` with 404. Client logs POST `/_otel` will fail (B11).
- Node: `node dist/server/node.mjs` (`PORT=8080 node dist/server/node.mjs`). Serves statics with MIME/cache, resolves param routes via the manifest, 404 page, SSR, `/_data`, `/_action`, `/_otel`. Node finds `node_modules` walking up from `dist/server`.
- Cloudflare Workers: `npx wrangler deploy --config dist/server/wrangler.jsonc` (wrangler bundles). Known issue: non-SSR param routes and unknown URLs get `index.html` (B1). `node:*` imports would need `compatibility_flags: ["nodejs_compat"]`, which the generated config lacks.
- `pragma serve` = local production preview of the same thing.

---

## 17. Anything else
- Generated `<head>` order: charset, viewport, `<title>`, layout head extras merged with the page's head tags (§5.2), theme script (if theme.js used), CSS, (dev flag), `__pd` loader script, modulepreloads, `#pragma-route` JSON, boot module.
- `#pragma-route`: `{ pattern, page, components, stores, loader, ssr }` (UNDOCUMENTED `stores` key).
- `/_pragma/manifest.json`: `[{ pattern, html, modules, loader, ssr }]`, used by router and Node adapter.
- Dev only: `self.__pragmaDev=true` → unknown path / unknown action / duplicate key / `:model` root errors throw in the browser.
- Not in v0: SSG with data, nested layouts, `[...rest]`, per-route CSS, i18n, Speculation Rules.

---

Counts: about 95 feature entries (sections 1–17), 19 entries marked UNDOCUMENTED (some cover several items, e.g. the env vars), 11 bugs in `BUGS.md`.
