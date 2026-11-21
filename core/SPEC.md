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
    crates/pragma-cli       `pragma` binary: new, dev, build, check, serve, upgrade
  example/           example app (generated with `pragma new`)
```

## 2. App structure (fixed)

```
<app>/
  AGENTS.md            agent rules (CLAUDE.md points to it)
  pragma.json          config: { "name", "lang", "webmcp" } ("budgets" from older apps is ignored)
                       "webmcp": true (default when absent) | false (§3.1)
  routes/
    _layout.html       root layout; contains <main data-page><slot></slot></main>
    _404.html
    index.html         → / (its <script> and <style> at the top, §4)
    index.js           the page script as a file instead (optional)
    index.server.js    page data (optional)
    shop/
      index.html       → /shop
      [id].html        → /shop/:id
      [id].server.js
      _components/<name>/<name>.html[|.js|.css]   components for this route only
  lib/
    components/<name>/<name>.html[|.js|.css]      app components (script/style inline or as files)
      modal/  alert/  dropdown/                 from the starter
    state/<name>.js    shared state (store, §4; reached through `shared`)
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
  `<title>`, `<meta>` and `<link>` at the top level of the fragment go to `<head>` (§5.1);
  elsewhere in a page or component they are an error (`html/head`).
- Component use: `<c-name>` = `lib/components/<name>/`; `<p-name>` = `_components/<name>/` in
  the page's folder or any ancestor up to `routes/`. Not found = error.

## 3. HTML syntax (closed list)

`{path}` puts a value in text, `:` binds a value, `.` binds a class, `@` binds an event, `$` is a
name of the owner's script (state, prop, constant or function). Condition and repetition are tags
(`<if>`, `<else-if>`, `<else>`, `<for>`, `<empty>`). The build translates everything to the
internal `data-*` the runtime reads (`{}` → `data-text` template or comment markers,
`@click` → `data-on-click`, `:href` → `data-attr-href`, `.x` → `data-class-x`, `$x` →
`state.x` or prop). Writing directive `data-*`, `<template>`, `:text`, `state.` or `props.` in
source is an error.

Paths: only `root.a.b` (no expressions, calls, indexes). Roots: `$x` (owner script name),
`data` (`load` result), `shared` (stores), `params` `query` (strings), `page`
(`page.loading`, `page.error`), `<for>` variable. A function met on a path in a value position
(`{}`, `:attr`, `.class`, `:show`, `<if>`, `<for>`, `:model` read) is a derived value: called with
no arguments on each render (`$total`, `shared.cart.total`).

| Syntax | Where | Effect |
|---|---|---|
| `{p}` | text, attribute value | text (escaped); in an attribute, part of a string (`alt="Photo of {p.name}"`) |
| `:<attribute>="p"` | element | whole attribute; `false`/`null`/`undefined` removes; `true` = empty |
| `:--<name>="p"` | element or component tag (goes to its root) | CSS custom property `--name` (`style.setProperty`); `false`/`null`/`undefined`/`""` removes; a value with `;` `{` `}` `<` is refused (dev: error); name in lowercase kebab-case (`html/css-var`). Build: `data-var-<name>`; SSR: `["attr","style","--name:{p}"]` |
| `.<class>="[!]p"` | element | toggles the class |
| `:show="[!]p"` | element | `hidden` when false |
| `:model="$p"` | input/select/textarea | two-way; only `$` or `shared` |
| `@<event>="fn"`, `@<event>="fn(a, …)"`, `@<event>="shared.store.fn(…)"` | element | calls the owner's function (or a store function); without parentheses it gets the event; with them only the listed args: path, `'text'`/`"text"`, number, `true`/`false`/`null`, `event`, `el` (the element with the `@`); no expressions, no `$` in the name |
| `@open="id"` `@close` | button | opens/closes overlay (section 6) |

`{}` rules: only paths (no `!`); `null`/`undefined` → empty; literal in text whose nearest
parent element is `<code>` or `<pre>` (a child element inside them interpolates; directives,
blocks and components are processed everywhere, `<pre>`/`<code>` included; only `<script>` and
`<style>` content is raw); `\{` is a literal brace elsewhere; not allowed in `class` (`html/class`) nor in a component prop
(`html/component`); `{{ }}`/`${ }` → `html/interpolation`; `:text` → `html/text-removed`.
Compiled form: an element whose children are only text gets `data-text="Hi {state.name}"` (a
template; children emptied); text next to child elements becomes `<!--{path}-->value<!--}-->`
(`<!--n{path}-->` for slot content n components up); an attribute becomes
`data-attr-<name>="… {path} …"` (a `data-attr-*` value with `{` is a string template, without
`{` a path). Templates made only of static props are inlined at build time. Without `ssr` the
prebuilt HTML never shows `{…}`: the value is empty until the runtime fills it at boot.

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
component's `.html` with a script, the component; in a page, the page. Slot content belongs to
its author. The build marks each slot-content root that has directives with `data-slot="n"`
(n = how many components up the owner is); the runtime evaluates that element in the scope
outside that component. Directives on the root of a component with a script belong to the component.
`$x` and `@fn`/`fn(…)` must be declared by the owner's script (`html/unknown-name`).

### Components

`<c-x>` = `lib/components/x/x.html`; `<p-x>` = `_components/x/x.html` in the page's folder or
any ancestor up to `routes/`. 1 root element. Expanded at build (macro).
- Component **with a script** (§4): props are declared with `export let name = default`; an
  undeclared prop at a call site is `html/unknown-prop` (`class` is not a prop: it is appended
  to the root). Static `title="X"` reaches the instance as `data-prop-title="X"` (bare →
  `"true"`), dynamic `:total="path"` as `data-with-total="<path in the caller's scope>"`
  (evaluated each render, so any value, including the caller's `$x`, passes live). Inside, `$x`
  is always the instance scope (`state.x`: state, props and functions).
- Component **without a script**: props are build-time. Static `title="X"` is a literal
  (`$title` is replaced by its text); `:total="path"` is the caller's path. Falsy: missing,
  `""`, `"false"`; truthy: bare attribute or any other value.
- `<slot></slot>` receives children; `<slot name="actions">` receives children with `slot="actions"`.
- **Self call**: a component with a script may render itself, directly, inside `<if>`/`<for>`,
  without slot content (`html/component` otherwise; indirect cycles too). The build emits the call
  as its root with no children and `data-self="<tag>"`, and once per page a
  `<template data-def="<data-c id>">` with the expanded body (at the end of the page content; at
  the end of the body for layout components). The runtime fills an empty `data-self` root from it
  and throws past 64 nested levels. SSR ops inline the body for 12 levels (64 calls per page); the
  rest stays empty for the client.
- The build marks the root with `data-c="x"` for `c-x`, and `data-c="x-<hash>"` for `p-x` (6 hex
  of the component folder path), so same-named private components in different folders never
  share CSS or the module lookup. The script loads only on pages using the component.

### Rendered block DOM (client and SSR identical)

The generated `<template>` stays in the DOM. Each instance is inserted as a sibling right after
the template of the branch that produced it, with `data-of="<tid>"` (the `data-tid` of the
`if`/`for` opening the chain) and, in `for`, `data-key="<value>"`. Updates diff by key; existing
nodes are not recreated.

Motion (client only, CSS-driven): an instance inserted after its block's first render gets
`data-entering` until its finite animations end (`getAnimations({ subtree: true })`). A removed
instance gets `data-leaving` + `inert`, is skipped by later renders, and leaves the DOM when its
finite animations end (timeout fallback: computed end time + 50 ms); with no animation, or under
`prefers-reduced-motion: reduce`, it leaves at the end of the same render. If its block wants it
back before then, the same node returns. Component cleanups run when the node leaves the DOM.

### Compiled format (for SSR)

The build emits `templates/<route>.json`: a list of ops.
```
["html", "<literal>"]
["text", "template"]                        "Hi {data.name}"; literals are HTML source, values escaped
["attr", "name", "path"]                    inside the opening tag
["attr", "name", "template"]                string attribute ("/p/{data.id}")
["class", "static", [["name","path",neg]]]
["show", "path", neg]
["if", tid, "path", neg, [ops]]
["elif", tid, "path", neg, [ops]]           only if no earlier branch in the chain rendered
["else", tid, [ops]]
["for", tid, "var", "path", "keyPath", [ops]]
["empty", tid, [ops]]                       only if the preceding for had no items
["comp", "id", [[prop, "text"|"path", value]], [ops]]   root of a component with a script (§5)
["slot", n, [ops]]                          slot content, rendered in the scope n components up
```
Each block op follows its template's html. A chain with a `shared` path is left entirely to the
client, and so is one with a `state` path when the server has no instance for that scope (§5); a
`text`/`attr` template with such a path is too. Instances carry `data-of` and `data-key` as above. A block newly inserted on the
client with `[autofocus]` receives focus.

### 3.1 WebMCP tools (forms)

pragma is agent-first: WebMCP is on by default (`"webmcp": true` in `pragma.json`; absent =
true). Each `<form>` is a WebMCP declarative tool for in-browser agents, written with the
standard attributes (Chrome declarative API / W3C WebMCP draft):

```html
<form toolname="search_products" tooldescription="Search the catalog by name or category" toolautosubmit @submit="search">
  <input name="query" required toolparamdescription="Words to search for">
</form>
```

- `toolname` + `tooldescription` on `<form>`: both required (one without the other = the browser
  registers nothing). `toolname`: 1–128 chars of `[A-Za-z0-9_.-]`; pragma style is lowercase
  `snake_case` verb_object (`search_products`, `create_order`).
- `toolautosubmit` (optional): the agent submits without a user click. Never on forms that
  delete, pay or send.
- `toolparamdescription` (optional) on input/select/textarea; fallback is the `<label>` text,
  then `aria-description`. Every field needs `name` (it is the parameter).
- `notool` on a `<form>`: opt-out for forms that must not be tools (login, password). The build
  removes it from the output.
- All these attributes are static: no `:` binding, no `{}`.
- Lifecycle is the DOM: the browser registers the tool when the form is in the document and
  removes it when it leaves, so router navigation needs no runtime code.
- Result to the agent: `@submit="fn"` gets the `SubmitEvent` (the runtime already called
  `preventDefault()`); `event.agentInvoked` is true for an agent call; `event.respondWith(promise)`,
  called before any `await`, returns the resolved value to the model. Read fields with
  `new FormData(event.target)`: an agent fill may not fire input events, so `:model` state can lag.
- CSS: `:tool-form-active` (the form) and `:tool-submit-active` (its submit button) are valid
  pseudo-classes in project CSS; `base.css` styles them with theme tokens.
- `"webmcp": false`: the `webmcp/*` rules are off and any of `toolname`, `tooldescription`,
  `toolparamdescription`, `toolautosubmit`, `notool` in source is an error (`webmcp/disabled`).
- The imperative API (`document.modelContext.registerTool`) is out of v0.

## 4. Page, component and store scripts (client)

A page or component script is one `<script>` at the top level of its `.html` (an ES module; no
`src`, no attributes except `type="module"`, one per file) or the separate `x.js`; both is
`html/duplicate-module`. A `<style>` at the top level is the component's (or page's) CSS, handled
like `x.css` (both = `html/duplicate-module`). The layout has neither.

```html
<script>
  import { shared, call, params } from 'pragma'

  export let start = 0
  export let step = 1

  let count = start
  let saving = false

  const atStart = () => count === start

  function increase() { count += step }
  async function save() {
    saving = true
    await call('save', { id: params.id, count })
    saving = false
  }
  function mount(el) {
    const timer = setInterval(() => { count += 1 }, 1000)
    return () => clearInterval(timer)
  }
</script>
<button @click="increase" :disabled="$saving">{$count}</button>
```

The top-level script is the component (no `export default`, `this` or `ctx`):
- top-level `let` = reactive state; top-level `const` / function = exposed to HTML. A function
  used in `@event` is an action (called with the call's arguments, §3), used in `{}` or a value
  binding a derived value (called with no arguments on each render; in JS call it: `atStart()`).
  `@event` also takes any top-level `const` (`const saveLater = debounce(save, 300)`): the build
  does not check its value; the runtime calls the function it holds and throws in dev when it is
  not a function.
- `export let name = default` = prop (§3): read-only (`js/prop-write`), live, coerced by the
  default's type (number: `"5"` → 5; boolean: bare/`"true"` → true, `"false"` → false; string:
  text; other defaults: the value as passed). An object or array prop is read-only all the way
  down: in dev the runtime hands it out through a read-only view that throws on any write.
  Pages and stores have no props.
- `function mount(el) { …; return cleanup }` runs after the first render, in the browser only;
  `el` is the component's root element (for a page, its `<main data-page>`), for observers,
  measuring or focusing its own DOM. The cleanup runs when the page is left or the component
  removed. An `async mount` resolves to its cleanup; if the component is gone by then, the
  cleanup runs at once.
- Derived values: a top-level `let`/`const` computed from state at startup is
  `js/stale-derived`, a function used as a value in JS without `()` is `js/derived-call`, `data`
  read at the top level is `js/top-level-data`, `$x` in a script is `js/dollar-in-script`.
- State is a deep Proxy over plain objects and arrays (`items.push(x)` works); every change
  schedules one render per microtask, from actions, `await`, timers or anywhere else. Maps,
  Sets, Dates and class instances are not tracked: replace them (`when = new Date()`).
- A top-level `const` changed in place (`items.push`) and read in HTML is `js/const-state`.
- Imports are hoisted out of the per-instance factory. No top-level `await` (do it in `mount()` or
  an action), no `var`, no destructuring `let`, no other `export` (`js/component-script`); `$` and
  `$pragma` are reserved names.
- Services: `import { shared, data, params, query, page, log, call, stream, navigate, open, close, confirm } from 'pragma'`.
  They are live: `data` is undefined until the loader is done (read it in functions, or rely on
  `ssr`: then it is already set when the script runs). Only page, component and store scripts
  import `pragma` (`js/import` elsewhere).
- Old API (`export const state`/`actions`, `export function mount`, `ctx.$`, `ctx.update`,
  `store()`) is `js/legacy-api`.

Compiled module (what the browser loads): `export default function ($, $pragma) { … return {
fns and consts } }`, with every read/write of a top-level `let` `x` rewritten to `$.x`, each
`export let p = d` to `$pragma.prop("p", d)` (read as `$.p`), each name imported from `pragma`
to `$pragma.name`, and the same lines as the source. `$` is the instance scope that HTML reads as
`state.*`; the runtime adds the returned functions/consts to it.

**Stores** (`lib/state/cart.js`) use the same rule: top-level `let` = state, functions = actions
or derived, all exposed as `shared.cart.*` (in HTML and in scripts through `shared` from
`pragma`). `export const persist = ['items']` keeps those `let`s in `localStorage['pragma:cart']`
(restored at startup, synced across tabs); it is the only export a store has. One instance per
store, created when a page that uses it loads (a store used by another store loads with it). A
direct `import` of a store is `js/store-import`.

## 5. Data, loaders and SSR

`routes/shop/[id].server.js`:
```js
export async function load({ params, query, request, env, log }) { return { product } }
export const actions = { async save({ input, params, env, log }) { return { ok: true } } }
export const ssr = false
```
- Default, no SSR: prebuilt HTML has everything outside blocks; the `<if page.loading>` chain is
  an inert `<template>` the runtime instantiates on boot, so the skeleton appears once
  `core/boot.js` runs, not in the raw HTML.
  The client fetches `GET /_data/<path>?<query>` → `{ "data": ... }` or `{ "error": { "message" } }`.
- In the `<head>` of pages with a loader, the build injects a minimal inline script that starts
  the fetch before modules: `self.__pd = fetch('/_data' + location.pathname + location.search)`.
  The runtime uses `self.__pd` if present.
- `call('name', input)` (from `pragma`) → `POST /_action/<path>/<name>` with JSON → action result.
  `stream('name', input, { signal })` → same request (`accept: application/x-ndjson, application/json`),
  async iterator over the NDJSON lines of a streamed `Response`.
- Wire contract for `/_data` and `/_action`: success → 200 with the JSON (`{ "data" }` for
  `/_data`, the action value for `/_action`, or a raw `Response` such as NDJSON); a redirect →
  200 `{ "redirect": { "status": 303, "location": "/x" } }` (the only key); an error → the real
  4xx/5xx with `{ "error": <body> }`. Client: `call`/`stream` throw `ActionError` (`status`,
  `body`, `message` = `body.message`); a redirect navigates (action: push; `load`: replace; other
  origin or 20+ hops in a chain: full load) and `call` resolves to `undefined`;
  `page.error` = `{ ...body, status }`.
- `ssr = true`: the server runs `load`, renders the compiled template and embeds data in
  `<script type="application/json" id="pragma-data">`. The client binds directives without
  rebuilding the DOM.
- With `ssr`, the server also runs the page script and the script of each component on the page
  (once per request, no `mount`, no events), so `$x` bindings (initial `let`s, props, derived
  functions) and slot content are in the first HTML. In those scripts `data`, `params`, `query`
  and `page` are read-only, `shared`, `call`, `stream`, `navigate`, `open`, `close` and `confirm` throw.
  A script is left to the browser (its `$x` bindings filled at boot, as without `ssr`) when it
  imports anything other than app code (`routes/`, `lib/`, `utils/`: no npm package, no
  `core/` module), when it throws while running on the server (reading `window`, `document`,
  `localStorage` or a store), or when a prop comes from a store. The rest of the page still
  renders. Keep browser-only work in `mount(el)` and actions.
- Error in `load` → `page.error` on the client; 500 with ERROR log on the server.
- Context of `load`, actions (`{ input, ...context }`), endpoints and middleware:
  `{ request, url, params, query, env, db, kv, files, log, cookies, locals, setHeaders, waitUntil }`.
  `url` is the page URL (`/_data/x` → `/x`). `cookies.get/getAll/set/delete`, defaults
  `Path=/; HttpOnly; Secure; SameSite=Lax` (no `Secure` on plain http localhost; `__Host-` and
  `__Secure-` always `Secure`). `setHeaders` rejects `set-cookie`. Headers and cookies are applied
  to the final response, including a returned or middleware-changed `Response`.
- `import { redirect, error } from 'pragma/server'` (only in server modules; the build rewrites
  it to `core/server/http.js`). Both throw. `/_data` and `/_action`: redirect → 200
  `{"redirect":{"status","location"}}` (also a 30x `Response` from an action or middleware);
  error → its status with `{"error": body}` (`body` = `{ message, ... }`). Documents: redirect →
  real 30x + `Location`; error → SSR shell with `page.error` and the status, or `_404.html` (404) /
  plain text. Endpoints: real 30x; error → its status with `{"error": body}`.
- Action returning a `Response`: passed through untouched (status, headers, streaming body).
- `routes/_middleware.server.js` exports `handle({ ...context, next })`; runs before every
  request except `/assets/*`, `/_pragma/*`, `/_otel`; `next()` returns the `Response`; must return
  a `Response`.
- Endpoint: `routes/<path>.server.js` without `.html`, exports only `GET`, `HEAD`, `POST`, `PUT`,
  `PATCH`, `DELETE`, `OPTIONS`; each returns a `Response` (else 500 naming the file). `HEAD`
  falls back to `GET`; other methods → 405 with `Allow`. WebSocket (101) only on Workers.

### 5.1 Head tags (SEO, link previews)

- `<head>` = the layout `<head>` merged with the page's top-level `<title>`, `<meta>`, `<link>`.
  A page tag replaces the layout tag with the same key (`title`; `meta` by `name`, `property` or
  `http-equiv`; `link` by `rel` + `hreflang`), in place; the others are appended. No title at
  all → `pragma.json` `name`.
- The layout `<head>` is static: any directive there is `html/head`.
- Page head tags take only `{path}` text (title), and `:<attr>` or `{path}` in an attribute
  (meta, link), with roots `data`, `params` or `query`, and require `export const ssr = true`
  (`html/head`). The build emits them as `data-text`/`data-attr-*` in the shell; the handler
  renders them before sending (escaped). A `:<attr>` path that is null/false keeps the static
  fallback; a `meta` left without `content` or a `link` without `href` is removed.
- JSON-LD: `<script type="application/ld+json">` at the top level of a page (not in a
  component, not nested: `html/script`) is structured data, never the page module. The build
  moves it into `<head>` (appended), checks its JSON (`html/json-ld`, with line and column) and
  allows `{path}` only inside JSON strings, with roots `data`, `params` or `query`
  (`html/head`), needing `ssr` like bound head tags. The handler fills each value JSON-escaped
  (`<` as `<`); the JSON syntax around it is kept. A literal `{` in a string is `{`.
  Any other `<script type>` is `html/script`.
- The router replaces `<title>` and every `<meta>`/`<link>` except charset, viewport,
  stylesheet and modulepreload with the target document's on each navigation.

## 6. Overlays: modal, alert, dropdown, popover

Always HTML + JS. Primitive in `core/overlay.js`; components in `lib/components/`.
- Modal and alert: `<dialog>` with `showModal()`. Alert uses `role="alertdialog"`.
- Dropdown and popover: native `popover` attribute.
- Open: `<button @open="about">`. Close: `@close` inside the overlay, Esc, or backdrop click
  (modal). In JS (`import { open, close, confirm } from 'pragma'`): `open(id)`, `close(id)`,
  `await confirm(id)` → `value` of the clicked `@close` button (`'cancel'` if closed otherwise).
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
- Navigate: fetch HTML, swap the content of `<main data-page>` and the head tags (§5.1), unmount the
  previous page (cleanups), import modules and start the loader in parallel.
  The layout is not recreated; its directives are reapplied.
- `popstate`: the overlay manager intercepts first.
- Scroll: top on new navigation; restored on back/forward. Focus moves to `<main>`.
- `navigate(url, { replace: true })` uses `history.replaceState`.
- Links `href="/path"` (no query or hash) to the current route get `aria-current="page"`:
  the build marks prebuilt HTML and the router keeps it on each navigation.
- `document.startViewTransition` when available, except under `prefers-reduced-motion: reduce`
  (instant swap).
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
  Component CSS goes in its `<style>` or `<name>.css`; for a `p-` component the build rewrites
  `[data-c="name"]` (`"name"`, `'name'` or unquoted) in its own CSS to its id.
- Scoping (automatic): each selector of a component's CSS that does not already contain its
  `[data-c="id"]` gets `:where([data-c="id"],[data-c="id"] *)` on its subject (before a
  pseudo-element), so it matches only the component root and its content and adds no specificity.
  A page's `<style>`/`.css` is scoped the same way to `[data-page="<page>"]`: the build writes
  the page name on the layout `<main data-page>` of pages with CSS, and the router copies it on
  navigation. `:global(…)` opts out only what is inside it (Svelte semantics; the wrapper is
  removed): the scope goes on the last compound that does not start with `:global(`
  (`:global(html.dark) .row` → `html.dark .row:where(…)`, `.list :global(.hit)` →
  `.list:where(…) .hit`); a selector whose every compound starts with it is global. Style rules
  inside `@media`, `@supports`, `@layer`, `@container`, `@starting-style` are scoped; other
  at-rules (`@keyframes`, `@font-face`…) are copied.
- WebMCP pseudo-classes `:tool-form-active` and `:tool-submit-active` are valid in any project
  CSS (§3.1).
- v0 output: one global minified CSS (lightningcss) with hash.

## 9. Build (`pragma build`)

```
dist/
  client/                 HTML per route, hashed assets, /_pragma/manifest.json
  server/worker.js        Cloudflare Workers entry (if any *.server.js)
  server/node.mjs         Node adapter (node:http) with static files
  server/wrangler.jsonc   assets dist/client, html_handling drop-trailing-slash, run_worker_first (no not_found_handling)
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
- The build prints the JS gzip size of each page and the largest CSS size. Information only: no
  size limits, no size errors or warnings.
- Server WebAssembly: a `.wasm` imported by the server graph (project file, or a package export
  that resolves to `.wasm`, its specifier rewritten to the copy) is copied to `dist/server/`
  (`node_modules/x` → `npm/x`); `wrangler.jsonc` gets `rules: [{ type: "CompiledWasm", globs:
  ["**/*.wasm"] }]` and `node.mjs` calls `registerWasm()` (Node load hook: default export =
  `WebAssembly.Module`) before a dynamic `import('./worker.js')`. `.wasm` in client code is
  `js/import`. `cloudflare:*` imports stay external in server code (Workers only; not on Node)
  and are `app/package` in client code.

## 10. Check (`pragma check`)

Errors with file:line:column, rule and message.
- JS (oxc): HTML in JS (section 0); `console.*`; TS or JSDoc types (`@type`, `@param {`,
  `@returns {`, `@typedef`, `@ts-`); import outside the project and `core/`; `import()` with
  a computed specifier (variable or `${}`) in app code (`js/import`); comment
  (`js/comment`); vague name (`js/vague-name`). Scripts (§4): old API (`js/legacy-api`, with
  the migration hint); assignment to a prop (`js/prop-write`); top-level `const` changed in
  place and read in HTML (`js/const-state`); `import` of a store (`js/store-import`); `var`,
  destructuring `let`, exports other than props/`persist`, top-level `await`, unknown name
  imported from `pragma`, reserved `$`/`$pragma` (`js/component-script`); `pragma` imported
  outside a script (`js/import`); `data` read at the top level (`js/top-level-data`); a
  top-level `let`/`const` computed from state or a prop at startup and never reassigned
  (`js/stale-derived`); a top-level function used as a value in a test, operator or template
  without a call (`js/derived-call`); `$x` in a script (`js/dollar-in-script`). All script rules
  of a file are reported in one pass; only `js/legacy-api` stops the HTML name checks.
- HTML: unknown or legacy syntax; invalid path root; `<for>` without `key`; malformed block or
  `<template>` in source; misplaced `<else-if>`/`<else>`/`<empty>`; block without single root;
  `$x`, `@fn` or `fn(…)` not declared by the owner's script, or a script-less owner
  (`html/unknown-name`); undeclared prop at a call site of a script component
  (`html/unknown-prop`); `<script>`/`<style>` both inline and as `x.js`/`x.css`
  (`html/duplicate-module`); component not found; inline `style`/`on*`; `:class`
  (`html/class`); `{{ }}`/`${ }` outside text directly in `<code>`/`<pre>` (`html/interpolation`); `:text`
  (`html/text-removed`); `{}` with an unclosed brace or an invalid path, event call with an
  expression argument (`html/invalid-path`); misplaced or
  wrongly bound head tags (`html/head`, §5.1); `<script>` with `src` or attributes, two
  `<script>`s, or one that is not at the top level of a page or component (`html/script`).
- CSS: unknown class; arbitrary value.
- `core/` version: warning `app/core-outdated` when the app's `core/` is not byte-equal to the
  runtime, docs and `base.css` embedded in the binary (a file changed, missing or extra); fix
  with `pragma upgrade` (§11).
- Accessibility warnings (`a11y/*`, non-blocking; `pragma check --strict` makes them errors), on
  the expanded page, pointing at the source file: `img-alt`, `accessible-name` (button or a
  without text, `{path}`, `aria-label`/`labelledby` or `title`), `label` (input, select,
  textarea; `type=hidden|submit|reset|button|image` exempt), `dialog-name`,
  `click-non-interactive` (`@click` on a non-interactive element without `role` and
  `tabindex`), `heading` (skipped level), `single-h1` (if/else branches count separately),
  `tabindex` (> 0), `duplicate-id`, `iframe-title`, `a-no-href`.
- WebMCP (§3.1; only with `"webmcp": true`), on the expanded page, pointing at the source file.
  Errors (the tool is broken): `webmcp/pair` (only one of `toolname`/`tooldescription`),
  `webmcp/name` (`toolname` outside `[A-Za-z0-9_.-]{1,128}`), `webmcp/duplicate` (same
  `toolname` twice on a page; if/else branches count separately), `webmcp/param-name` (field
  without `name` in a tool form; `type=submit|reset|button|image` exempt), `webmcp/dynamic`
  (`:` binding or `{}` on a tool attribute; checked on the source, before expansion), `webmcp/notool` (`notool` with `toolname`, or on a
  non-form element), `webmcp/attribute` (`toolname`/`tooldescription`/`toolautosubmit` off a `<form>`,
  `toolparamdescription` off input/select/textarea). Warnings (non-blocking; `--strict` makes
  them errors): `webmcp/form-tool` (`<form>` without `toolname`+`tooldescription` and without
  `notool`), `webmcp/name-style` (not lowercase `snake_case` with at least one `_`),
  `webmcp/description` (`tooldescription` under 4 words), `webmcp/param-description` (field
  without `toolparamdescription`, label or `aria-description`), `webmcp/autosubmit`
  (`toolautosubmit` on a tool whose name starts with `delete_`, `remove_`, `destroy_`, `pay_`,
  `buy_`, `purchase_`, `cancel_`, `send_` or `transfer_`).

## 11. Server and dev

- Server code is JS in `fetch(request, env, ctx)` form. Cloudflare Workers is the preferred
  target; Node ≥ 22 via `dist/server/node.mjs`.
- `pragma dev` (Rust, hyper + tokio, port 5180): incremental build in `.pragma/dev/`, watch,
  live reload via a WebSocket at `/_pragma/reload` (CSS swaps without reload; not SSE, because an
  `EventSource` per tab uses up the 6 HTTP/1.1 connections per host). If any `*.server.js`
  exists, starts `node .pragma/dev/server/node.mjs` on an internal port and proxies `/_data`,
  `/_action`, `/_otel`, ssr pages and endpoints (any method); with a middleware, every path
  except `/assets/*` and `/_pragma/*` (from `server/routes.json`). A build with errors still writes its outputs (pages without
  errors keep working, with the error overlay): node is restarted for it too, so it always serves
  the same build as the client files.
- `pragma dev --cf`: same, but node gets `PRAGMA_CF_CONFIG=<root wrangler config>` and `serve`
  merges wrangler `getPlatformProxy({ configPath, remoteBindings: true })` `env` (bindings win;
  `ASSETS` stays the static server), uses its `ctx` and sets `globalThis.caches`. Wrangler is
  resolved from the app (`npm i -D wrangler@4`); missing config or wrangler stops with the fix.
  Node is stopped with SIGTERM (disposes workerd) and restarted when the config or `.dev.vars`
  changes; `.wrangler/` is not watched.
- `pragma serve`: same server without watch, over `dist/`. Ctrl+C or SIGTERM also stop the
  child node.
- Markdown for agents: a request without a file extension whose `Accept` lists
  `text/markdown` with a q greater than or equal to that of `text/html` (or `text/html` absent;
  browsers never send `text/markdown`) gets the static
  `<path>.md` or `<path>/index.md` (`/` → `/index.md`) from the client output (from `public/`)
  when it exists, as `text/markdown; charset=utf-8`. Responses for such paths (HTML or
  markdown) carry `Vary: Accept`. Same behavior in `pragma dev`, `pragma serve` and the handler
  (Workers and Node). No `.md` = normal HTML.
- A path ending in `.html` is never an SSR page (it is the prebuilt HTML the router fetches),
  in `pragma dev/serve` and in the handler.
- `pragma new <dir>`: copies `starter/` + `runtime/` → `<dir>/core/` + `docs/` → `<dir>/core/docs/`
  (embedded in the binary).
- `pragma upgrade`: makes `<app>/core/` equal to the embedded runtime, docs and `base.css`
  (writes changed and missing files, removes files the binary does not have) and prints each
  file updated, added or removed. Writes nothing outside `core/` (`AGENTS.md`, `pragma.json`,
  `routes/`, `lib/`… are the app's).
- An outdated `core/` (`app/core-outdated`) is a warning in `check` and at the start of `dev`, and
  stops `pragma build` with an error (the compiler may emit ops an older runtime cannot render)
  unless `--allow-outdated-core`.

## 12. Logs (OpenTelemetry)

- `core/log.js` (client) and `core/server/log.js`: `log.debug|info|warn|error(msg, attrs)`.
  Records follow the OTel Logs Data Model; attributes use semantic conventions.
- Client: captures `error`, `unhandledrejection`, navigation and fetch failures; batches to
  `POST /_otel` (`keepalive`/`sendBeacon` on `visibilitychange`). Every runtime fetch sends
  `traceparent`.
- Server: span per request; `/_otel` forwards to OTLP/HTTP if `OTEL_EXPORTER_OTLP_ENDPOINT`
  is set, else prints OTel JSON to stdout.
- `pragma dev` prints readable logs in the terminal and starts node with
  `PRAGMA_LOG_LEVEL=debug` (unless set). Elsewhere the default minimum is `info`.
- `pragma serve` without a backend answers `POST /_otel` with 204. The client stops sending
  after a 404/405 from `/_otel` (static hosts).

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
- `__pd` only on pages with a loader. Extra `<head>` tags come from the layout `<head>` merged
  with the page's head tags (§5.1), right after `<title>`.
- `#pragma-route`: `{ "pattern", "page": "<page module url or null>",
  "components": { "<data-c id>": "<url>" }, "stores": { "<name>": "<url>" }, "loader": bool, "ssr": bool }`.
  Only components with a script. Stores: read in HTML (`shared.x`), in page or component
  scripts (`shared.x` from `pragma`), and the stores those stores read.
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
- `core/state.js` (owner: client runtime): `reactive(value)` (deep Proxy; changes call the
  `onChange` callback, which `page.js` turns into `schedule()`), `persist(target, name, keys)`,
  `persistStores()`.
- `core/page.js`: one instance per page and per script component (`data-c`): calls the
  compiled factory with `$` (its reactive scope) and `$pragma` (the services plus
  `prop(name, default)`). `hooks.enter(el, scope, kind, withScope)` (from `directives.js`)
  returns `{ __proto__: scope, state: instance scope }`; props read `withScope` (the
  `data-with-*` values of that render) or `data-prop-*`. Events: `eventCall(el, type, event)`
  (`directives.js`) gives `[name, args]` and `page.js` calls the owner's function.
- Component script owner: lexical (§3). The runtime stores the owner in each element's scope;
  `data-slot="n"` goes up n components. With no component owner, the owner is the page. The
  layout has no script in v0: its directives use `shared`, `params`, `query`.
- `/_pragma/reload` (WebSocket) and `/_pragma/dev.js` (live reload client) are served by
  `pragma dev`, injected only in dev.

### CSS the runtime expects from `base.css`
`html[data-scroll-lock]{overflow:hidden}`, `html[data-scroll-lock=gutter]{scrollbar-gutter:stable}`, `[hidden]{display:none!important}`,
default `dialog::backdrop` and `.skeleton` (pulsing block honoring `prefers-reduced-motion`).

## 15. Server ↔ build contract

```js
// core/server/handler.js
export function createHandler({ routes }) → { fetch(request, env, ctx) }
// routes: [{ pattern: '/shop/:id', server: <.server.js namespace>, ssr: bool,
//            template: <compiled ops> | null, shell: '<prebuilt page html>' | null,
//            page?: <page script module>, components?: { '<data-c id>': <component script module> } }
//          | { pattern: '/files/:name', endpoint: <.server.js namespace>, file: 'routes/files/[name].server.js' }]
// middleware?: handle from routes/_middleware.server.js
```
- `page`/`components`: the compiled scripts (factories, §4) an `ssr` route renders with, only
  those whose imports are all app code; `render.js` `renderPage(ops, scope, { page, components, log })`
  runs them with read-only services and falls back to the client per page or component.
- Handles `/_data/*`, `/_action/*`, `/_otel` and `ssr` pages (body and head tags). Everything
  else: `env.ASSETS.fetch(request)` (Workers) or the Node adapter's static server; on a 404 it
  resolves non-ssr param routes via `/_pragma/manifest.json` and otherwise answers `_404.html`
  with status 404.
- The build emits `dist/server/worker.js`: imports `./core/server/handler.js` and each
  `.server.js` (copied to `dist/server/` with relative imports, paths preserved), embeds
  templates and shells as JS objects and does `export default createHandler(...)`.
- The build emits `dist/server/node.mjs`:
  `import h from './worker.js'; import { serve } from './core/server/node.mjs'; serve(h, { root: new URL('../client/', import.meta.url), port: process.env.PORT ?? 3000, service: "<pragma.json name>" })`.
  `serve` logs with `service.name` = `service` (`OTEL_SERVICE_NAME` wins), as the handler does.
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
