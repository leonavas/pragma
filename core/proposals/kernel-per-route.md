# Proposal: kernel per route (minimal JS per page)

Status: contract, 2026-11-24. Decisions are final (author + council); this file is the only
shared document between the two implementers:

- **runtime**: `core/runtime/*.js`, `core/runtime/server/`, runtime tests.
- **compiler**: `core/cli/crates/pragma-compiler` (`lib.rs`, `chunks.rs`, `compile.rs`, a new
  pass for detection and descriptors), `pragma-css` (`base.rs`), compiler snapshots.

Everything marked **[contract]** is the boundary: change it only in this file, and tell the other
side. Everything marked **[runtime-internal]** is guidance; the runtime may rename it.

Rules that hold for all of it: plain JS, never TypeScript, no JSDoc types. No feature flags. No
mangle-props or terser. When the build is not sure that a route needs a module, it includes it.

---

## 0. Summary of the change

| Today | After |
| --- | --- |
| `boot.js` on every page, `#pragma-route` JSON, `URL#export` per script | One generated **entry module per route**; it imports only what the route uses and calls `start(R)` |
| `directives.js` parses `data-*` attributes and `<!--p:…-->` comments with regexes | The compiler writes **descriptors** (pre-parsed) into a table in the entry; HTML carries only `data-b="<id>"`, `<!--p:<id>-->` and `<!--{-->` |
| `render(document.body)` after each change with a block or no tracked reader | Re-visit only the **owner** element of a structural read; patch bindings for the rest |
| Every page loads JS | **Static route = zero JS** (no module, no modulepreload, no route JSON) |
| A click before the runtime is lost or replayed | JS-only controls ship `disabled`; the kernel enables them when their handler is live. No replay |

---

### Block styles [contract, 2026-11-24]

A prebuilt production route whose large stylesheet has deferrable component rules passes
`applyBlockCss` to `start(R, applyBlockCss)`. Its markup contains a style preload with
`data-pragma-full-css` (canonical fallback URL) and `data-pragma-blocks` (space-separated
template ids, self definitions prefixed with `@`). Only cloning the named bodies waits for CSS.
The timeout is 1.5 s. Other controls mount immediately. The router stages the destination sheet,
adopts the critical styles in document order and applies that route's hook at entry. SSR keeps
its complete sheet because the request can render different component scopes.

## 1. Runtime modules

All files are in `core/runtime/` (copied to `.pragma/` of an app). "Kernel" = the three eager
files. Per-use modules are imported **statically by the route entry** only when the route uses
them. Lazy modules are loaded with `import()` by the kernel or by another lazy module, never
modulepreloaded.

| File | Exports | Imported by | Load | Trigger |
| --- | --- | --- | --- | --- |
| `state.js` | `reactive`, `create`, `readBy`, `track` API (`readers`, `forgetReads`, `prune`), `S` (notify hook, target being created) | `dom.js`, `page.js`, `persist.js`, `model.js` | eager (kernel) | every reactive route |
| `dom.js` | `visit`, `render`, `revisit`, `patch`, `bind`, `get`, `fill`, `clone`, `of`, `sib`, `scopes`, `desc` (table lookup), `body` (template lookup), `useTables`, `reveal`, `H` (hooks), `B` (extra binders), `L` (leaving symbol), `outer` access, `DEV` | `page.js`, all per-use modules, `overlay.js` | eager (kernel) | every reactive route |
| `page.js` | `start(R)` **[contract]**, `install(use)`, `K` (registry), `enter`, `leave`, `schedule`, `closeOverlays`, `popOverlay` | the route entry (`start`), `router.js` (via the object it gets in `startRouter`) | eager (kernel) | every reactive route |
| `branch.js` | `default install(K)` | entry | per use | route has `<if>`, `<else-if>`, `<else>` or `<empty>` |
| `list.js` | `default install(K)` (keyed `<for>` **and** slicing, always together) | entry | per use | route has `<for>` |
| `model.js` | `default install(K)` | entry | per use | route has `data-model` |
| `motion.js` | `default install(K)`, `afterMotion(n, done)` | entry (install), `overlay.js` (`afterMotion` only) | per use | route has a block **and** its CSS mentions `data-entering`/`data-leaving` (§4) |
| `self.js` | `default install(K)` | entry | per use | route has `data-self` |
| `vars.js` | `default install(K)` (binder kind 6) | entry | per use | route has `data-var-*` |
| `persist.js` | `default install(K)` | entry | per use, **eager and synchronous** | a store of the route exports `persist` |
| `net.js` | `default install(K)`, `fetchData`, `fetchLayout`, `load`, `loadResult` | entry, `router.js` | per use | route has `load` or (`routes/_layout.server.js` exists and route is reactive) |
| `trace.js` | `traceHeaders`, `newTrace` | `net.js`, `action.js`, `log.js`, `router.js` | with its importers | never alone |
| `router.js` | `startRouter(api)`, `navigate`, `redirect` (prefetch and view transitions inside) | `page.js` via `import()` | lazy | idle after the first mount; or first `navigate()`/`redirect()` |
| `log.js` | `log`, `initClientLog` | `page.js` via `import()` | lazy | idle after the first mount; or first `log.*` (the kernel keeps a queue stub until then) |
| `overlay.js` | `initOverlays`, `open`, `close`, `confirm`, `closeAll`, `handlePopState` | `page.js` via `import()` | lazy | `enter` of a route with `R.overlays`; or first `open()`/`close()`/`confirm()` |
| `action.js` | `call`, `stream`, `ActionError` | `page.js` via `import()` | lazy | first `call()` / `stream()` |
| `theme.js` | `theme`, `setTheme` | app code | as today | as today |
| `contract.js` | shared constants and helpers | server + client modules (named imports only, tree-shaken) | — | — |

Removed: `boot.js` (its work moves to `start`), `directives.js` (split into `dom.js` + per-use
modules), `loader.js` (split into `net.js` + `action.js`).

`contract.js` changes: drop `ROUTE_ID`, `PERSIST_SUFFIX`, `THEME_KEY` (and the `THEME_KEY`
migration in `state.js`); `matchPattern(pattern, path)` compares segments directly (no
`sortRoutes` of a one-route table). The server keeps importing what it imports today.

**[contract] One kernel instance.** Lazy modules are imported by a relative specifier
(`import('./router.js')`) from `page.js`. In dev the entry must import the kernel by the exact URL
under which `page.js` is served (`urls[".pragma/page.js"]`), so relative `import()` lands on the
same module graph. In prod rolldown bundles all of it.

---

## 2. Registration API

### 2.1 Per-use module **[contract]**

Each per-use module has no side effects at evaluation and exports one function:

```js
// .pragma/list.js
import { H, visit, clone, of, sib, scopes } from './dom.js'
export default function install(K) {
  H.list = list
  K.slices = { resume, pending: () => deferred.length > 0 }
}
```

The route entry lists the install functions in `R.use`. The kernel installs them:

```js
// page.js [runtime-internal]
const installed = new WeakSet()
export function install(use = []) {
  for (const f of use) if (!installed.has(f)) installed.add(f), f(K)
}
```

- **Idempotent**: same module ⇒ same function object (rolldown shares chunks; dev serves one URL).
- **Additive**: nothing is ever uninstalled. After visiting a heavy route, a light route keeps the
  modules; that is correct, not a leak.
- **Order**: `start(R)` and the router call `install(R.use)` **before** anything of that route
  runs (stores, page script, first render). So `persist` restores before the first render and
  mount.
- **Invariant** the compiler guarantees: the modules installed always cover every construct in the
  live DOM. The layout is in every route's closure (§4), so a module the layout needs is in every
  route's `use`.
- A missing hook is a compiler bug: in DEV the kernel throws `pragma: <construct> needs <file>.js
  (build bug)`; in prod it skips the construct.

### 2.2 Kernel registry **[runtime-internal]**

`K` (from `page.js`) and `H`/`B` (from `dom.js`) are what install functions mutate:

| Slot | Set by | Kernel default |
| --- | --- | --- |
| `H.branch(anchor, d, s, last, id)` | `branch.js` | DEV error |
| `H.list(anchor, d, s, last, id)` | `list.js` | DEV error |
| `H.made(n, d)`, `H.drop(n)`, `H.stay(n)`, `H.done()` | `motion.js` | no-op / remove at once / no-op / no-op |
| `H.self(el)`, `H.depth(dir, el)` | `self.js` | DEV error / no-op |
| `H.model(el, s, p)`, `H.input(e)`, `B[7]` | `model.js` | none |
| `B[6]` | `vars.js` | none |
| `K.persist(target, name, keys)`, `H.flushed()` | `persist.js` | none |
| `K.net` (`loadData`, `loadLayout`, `fetchData`, `fetchLayout`, `refreshLayout`) | `net.js` | none |
| `K.slices` | `list.js` | none |
| `K.overlay` (loaded overlay module) | `page.js` when `overlay.js` arrives | `null` |

Services that live in lazy modules are stubs made by one factory, `later(file, names, pick)`,
which imports on call and then forwards (`navigate`, `redirect`, `open`, `close`, `confirm`,
`call`, `stream`). `log` is a queue stub: calls before `log.js` arrives are stored as
`[level, msg, attrs]` and replayed by `initClientLog`.

### 2.3 What the route declares **[contract]**

The entry's default export `R`:

| Key | Type | Meaning |
| --- | --- | --- |
| `pattern` | string | route pattern (`/shop/:id`) |
| `loader` | `1` or absent | page has `load` |
| `ssr` | `1` or absent | page is server-rendered |
| `layout` | `1` or absent | `routes/_layout.server.js` exists |
| `overlays` | `1` or absent | route has `<dialog>` or `[popover]` |
| `use` | array of install functions | per-use modules (§4), in the order of §1's table |
| `page` | function or absent | default export of the page script (the factory) |
| `comps` | `{ [dataCId]: factory }` | components **with a script** used by the route |
| `stores` | `{ [name]: [factory] \| [factory, persistKeys] }` | stores of the route (transitive), with their `persist` export when it exists |
| `t` | `{ [bodyId]: html }` | block bodies (today's template module `default`) |
| `d` | `{ [dataCId]: html }` | self-call bodies (today's template module `d`); omitted when empty |
| `b` | `{ [descId]: descriptor }` | descriptors (§5) |

`t`, `d` and `b` hold both regions: ids that start with `l` belong to the layout region (outside
`<main data-page>`), the others to the page region. The kernel takes `l…` ids from the tables of
the route the document **booted** on and the other ids from the **current** route (same rule as
today's `layoutBodies`). This is consistent by construction: the layout DOM never changes after
boot. Self-call bodies are looked up in the current route's `d`, then in the boot route's `d`.

### 2.4 `start(R)` **[contract for the call; body is runtime-internal]**

```js
export function start(R) {
  install(R.use)
  if (booted) return          // SPA: the router drives enter()
  booted = true
  useTables(R, true)          // boot tables (l…) + current tables
  // stores (persist restores here, synchronously), net (__pd / __pl), page instance, first render
  enter(R, new URL(location.href), document, self.__pd)
  // after the first mount, in idle: import('./log.js'), then import('./router.js').startRouter(...)
  // R.overlays: import('./overlay.js') now
}
```

---

## 3. Entry and HTML

### 3.1 Entry module **[contract]**

One file per reactive route. Prod: a rolldown input (replaces both the `boot` input and the
`export { default as p } …` re-export entries); output `routes/<out_name>-<hash>.js`. Dev: written
as is to `/assets/entry/<out_name>.js` (not `/assets/routes/…`: that is the dev URL of the page
script `routes/<name>.js`), imports by dev URL.

Order inside the file: kernel import, per-use imports (order of §1's table), app scripts, `R`,
`export default R`, `start(R)`.

```js
import { start } from '/assets/core/page.js'
import branch from '/assets/core/branch.js'
import list from '/assets/core/list.js'
import model from '/assets/core/model.js'
import persist from '/assets/core/persist.js'
import net from '/assets/core/net.js'
import page from '/assets/routes/todos.js'
import c0 from '/assets/routes/_components/todo-item.js'
import s0, { persist as s0p } from '/assets/lib/stores/prefs.js'

const R = {
  pattern: '/todos',
  loader: 1,
  layout: 1,
  use: [branch, list, model, persist, net],
  page,
  comps: { 'todo-item': c0 },
  stores: { prefs: [s0, s0p] },
  t: {
    t0: '<li data-b="p8"><input type="checkbox" disabled data-b="p9"><span data-b="pa"></span></li>',
    t1: '<p>Nothing yet</p>',
  },
  b: {
    l0: { x: 2 },
    p0: { e: { submit: [['add'], [0]] } },
    // … see §5
  },
}
export default R
start(R)
```

- A store without a `persist` export is `[s0]` (do not import a name that does not exist; use
  `pragma_jslint::exported_names`).
- Prod `t` strings are compact (as `template_module(…, true)` today).
- The separate `/assets/tpl/*.js` template module disappears; its content is `R.t` / `R.d`.

### 3.2 HTML of a reactive route **[contract]**

`<head>` in this order (only the lines that change are described):

```html
<meta charset="utf-8"> <meta name="viewport" …> <title>…</title>
<script>self.__pd=fetch('/_data'+…)…</script>     <!-- loader && !ssr (unchanged) -->
<script>self.__pl=fetch('/_layout'+…)…</script>   <!-- layout && !ssr (unchanged) -->
… head extras, font preloads, THEME script, CSS (unchanged) …
<script>self.__pragmaDev=true</script>            <!-- dev only (unchanged) -->
<link rel="modulepreload" href="/assets/chunks/page-1a2b3c4d.js">
<link rel="modulepreload" href="/assets/chunks/list-5e6f7a8b.js">
…                                                  <!-- static-import closure of the entry, entry excluded, load order; npm deps expanded as today -->
<script type="module" src="/assets/routes/todos-9f8e7d6c.js"></script>
<script>VT</script>                                <!-- §3.4, every page -->
<script type="speculationrules">…</script>         <!-- prod (unchanged) -->
```

- No `<script type="application/json" id="pragma-route">`. Removed.
- Lazy chunks (router, log, overlay, action) are never in the modulepreload list.
- `<body>`: the page with `data-b`, `<!--p:…-->`, `<!--{-->`/`<!--}-->` and the build `disabled`
  (§5, §6).

### 3.3 Static route **[contract]**

A route is **static** when none of these is true (computed on the closure of §4):

- an element or anchor with a runtime directive: `data-text`, `data-attr-*`, `data-var-*`,
  `data-class-*`, `data-show`, `data-if`, `data-else-if`, `data-else`, `data-for`, `data-empty`,
  `data-on-*`, `data-model`, `data-open`, `data-close`, `data-self`, or a `#interp`;
- a page script, a component script, a store;
- `load` in the page's `.server.js`;
- a `<dialog>` or `[popover]`.

A static route writes: no entry, no modulepreload, no `#pragma-route`, no `__pd`/`__pl`, no
`data-b`, no build `disabled`. It keeps CSS, `THEME` (when `theme.js` is in the app), `VT` and
speculation rules. In the manifest its `entry` is `null`. `data-c` and `data-s` stay (CSS).

Head tags (`<title>`, `<meta>`, `<link>`, JSON-LD) never get descriptors, in any route: they keep
`data-text` / `data-attr-*` for `renderHead` on the server (unchanged).

### 3.4 Navigation between routes **[contract]**

- `pragma-css` base: add `@view-transition { navigation: auto; }`.
- `VT` (inline in every page's head, compiler constant): sets `html[data-view-transition]` while a
  cross-document transition runs, so `[data-page]` gets its `view-transition-name`:

```html
<script>addEventListener('pageswap',e=>e.viewTransition&&document.documentElement.setAttribute('data-view-transition',''));addEventListener('pagereveal',e=>{const v=e.viewTransition,h=document.documentElement;if(v)h.setAttribute('data-view-transition',''),v.finished.finally(()=>h.removeAttribute('data-view-transition'))});addEventListener('pageshow',e=>e.persisted&&document.documentElement.removeAttribute('data-view-transition'))</script>
```

- Manifest entry: `{ "pattern", "html", "entry": url | null, "modules": [preload urls], "loader", "ssr" }`.
- Router (`go()`): target with `entry: null` ⇒ full load (`location.assign`). Otherwise it fetches
  the HTML, then `const R = (await import(e.entry)).default`, calls `install(R.use)` (the entry
  already did it when it evaluated; calling again is a no-op), loads route CSS, then
  `enter(R, url, doc, pending)` inside the view transition. It no longer reads `#pragma-route`.
- A click on a link before `router.js` is in ⇒ normal MPA navigation (cross-document transition).

---

## 4. Detection rules (compiler)

### 4.1 Closure of a route **[contract]**

Scan, **after `compile::hoist`**:

- `o.exp.body`: layout + nested layouts + page + every expanded component + slot content
  (all inlined by `expand.rs`), with the block anchors (`<template data-if …>`);
- `o.bodies`: every `<template data-tpl>` (block bodies, at any depth) and `<template data-def>`
  (self-call bodies);
- scripts: `o.page.module`, the components with `module`, `o.stores` (already transitive:
  `shared.x` in HTML + `shared_refs` of scripts + store → store);
- server: `o.loader`, `p.layout_server`;
- CSS: the shared CSS string and this route's CSS string, as written (after pruning).

Not in the closure: head tags. `call`/`stream`/`navigate`/`log` need no detection (lazy).

### 4.2 Table **[contract]**

| Construct in the closure (internal attribute after sugar/expand) | Descriptor (§5) | Module in `R.use` / `R` key |
| --- | --- | --- |
| `data-text`, `#interp`, `data-attr-*`, `data-class-*`, `data-show` | `b` kinds 0–5 | kernel |
| `data-var-*` | `b` kind 6 | `vars.js` |
| `data-on-*` | `e` | kernel |
| `data-model` | `m` | `model.js` |
| `data-if`, `data-else-if`, `data-else` | anchor `t` 1/4/3 | `branch.js` |
| `data-empty` | anchor `t` 3 | `branch.js` (and `list.js`, implied by its `<for>`) |
| `data-for` | anchor `t` 2 | `list.js` (slicing included) |
| `data-self` (element with an empty root) | `self` | `self.js` |
| `data-c` (any component root, with or without script) | `x: 1`, `c` | kernel; scripted ones in `R.comps` |
| `data-with-*` | `w` | kernel |
| `data-prop-*` | `p` | kernel |
| `data-slot` (element, anchor, marker) | `u` / anchor `u` / marker slot | kernel |
| `<main data-page>` | `x: 2` | kernel |
| `data-open`, `data-close` | `o: 1` (gates `d`) | `R.overlays = 1` (a `d` control with `o` is enabled only when `overlay.js` is in, so the route loads it at start) |
| `<dialog>`, any `[popover]` | — | `R.overlays = 1` |
| page script | — | `R.page` |
| store in `o.stores` | — | `R.stores`; if its source exports `persist` → `persist.js` |
| `o.loader` | — | `R.loader = 1`, `net.js` |
| `p.layout_server.is_some()` and route reactive | — | `R.layout = 1`, `net.js` |
| ≥ 1 block anchor (`if`/`elif`/`else`/`for`/`empty`) **and** (shared CSS + route CSS) contains the substring `data-entering` or `data-leaving` | — | `motion.js` |

Notes:

- **Layout** is part of every route's closure, so its constructs set modules on every route.
- **Slots**: slot content is already inlined in the component expansion; scanning the expanded
  tree covers it. Its `data-slot` level counts every component root, scripted or not, so every
  `data-c` root in a reactive route gets a descriptor with `x: 1`.
- **Bodies**: a construct only inside an inactive branch or an empty list still counts (it is in
  `o.bodies`). This is the main risk the plan names; scanning `o.bodies` is mandatory.
- **Self**: `data-def` bodies are scanned like any body (a recursive tree can contain `<for>`,
  events, etc.).
- **Motion**: the substring test is deliberately coarse. A false positive costs ~1 kB; a false
  negative breaks enter/leave animations.
- `overlay.js` uses `afterMotion` from `motion.js` directly (static import inside the lazy chunk);
  that does not need `install`.

---

## 5. Descriptors

### 5.1 Shapes **[contract]**

Common values:

- **path**: array of keys, root included: `state.items.length` → `["state","items","length"]`.
- **tpl**: array of odd length, literal / path / literal …: `"{t.text} left"` →
  `["", ["t","text"], " left"]`. `\{` is already unescaped in the literals.
- **literal**: `{ "v": <json> }`.
- **neg**: `0` or `1` (`!path`).

Binding tuple `[kind, a, n, name]` (one shape so `bind` destructures once):

| kind | name | tuple | from |
| --- | --- | --- | --- |
| 0 | TEXT | `[0, tpl]` | `data-text` |
| 1 | SHOW | `[1, path, neg]` | `data-show` |
| 2 | ATTR | `[2, path, 0, attr]` | `data-attr-<attr>` without `{` |
| 3 | CLASS | `[3, path, neg, class]` | `data-class-<class>` |
| 4 | ATPL | `[4, tpl, 0, attr]` | `data-attr-<attr>` with `{` |
| 5 | MARK | `[5, path, k, slot]` | `#interp`: `k` = index of this marker among the element's direct `<!--{-->` children; `slot` = level (0 = none) |
| 6 | VAR | `[6, path, 0, name]` | `data-var-<name>` (`vars.js`) |

Kind 7 is reserved for the model record inside the runtime (never in descriptors). Bindings keep
the order of the source attributes, MARK bindings last (as today).

Element descriptor (all keys optional, absent = false/empty):

| Key | Value |
| --- | --- |
| `b` | array of binding tuples |
| `e` | `{ [type]: [fn, args] }`; `fn` = path (`["add"]` or `["shared","cart","add"]`); `args` = array of `0` (event), `1` (el), literal, path. Without parentheses in the source: `[0]` |
| `m` | path of `data-model` (root `state` or `shared`) |
| `w` | `[[name, path \| literal], …]` from `data-with-*` |
| `p` | `{ [name]: "text" }` from `data-prop-*` |
| `x` | `1` component root, `2` `<main data-page>` |
| `c` | component id (same as `data-c`), with `x: 1` |
| `self` | tag for messages (`"c-tree"`), on a self-call root |
| `u` | slot level of the element |
| `o` | `1`: has `data-open`/`data-close` |
| `d` | `1`: build-disabled control (§6) |

Anchor descriptor (has `t`):

| Key | Value |
| --- | --- |
| `t` | `1` if, `2` for, `3` else or empty, `4` else-if |
| `s` | body id in `R.t` |
| `u` | slot level |
| `p`, `n` | condition path and neg (`t` 1 and 4); list path (`t` 2) |
| `v`, `k` | item variable name and key path (`t` 2) |

### 5.2 Ids and tables **[contract]**

- Element: one attribute `data-b="<id>"`. Anchor: comment `<!--p:<id>-->`. Marker:
  `<!--{-->text<!--}-->` (no path).
- Ids: `l<base36>` in the layout region, `p<base36>` in the page region (inside
  `<main data-page>`). A body or def gets the prefix of the region it was hoisted from; its inner
  descriptors too. Body ids keep today's scheme (`l…`, `t…`) in `R.t`.
- Equal descriptors (same JSON) in the same region share one id.
- Descriptors are **immutable** and shared. All per-node state lives on the node, under symbols:
  descriptor cache, marker text nodes, binding records, owner record, enabled flag, call cache,
  anchor `rendered`/`partial`/`end`. (Today's runtime writes `d.c`, `d.rendered`, `d.partial`,
  `d.end` on descriptors; with shared descriptors that is a bug.)
- Attributes that **stay** in HTML: `data-c`, `data-page`, `data-s`, `data-of`, `data-key`,
  `data-open`, `data-close`, `data-reload`, `data-placement`, `data-theme`, `id`, every non-directive
  attribute. Attributes that **leave** body HTML (moved into the descriptor): `data-text`,
  `data-attr-*`, `data-var-*`, `data-class-*`, `data-show`, `data-on-*`, `data-model`, `data-with-*`,
  `data-prop-*`, `data-slot`, `data-self`, and the `data-if`/`data-for`/… `<template>` anchors.

Runtime lookup **[runtime-internal]**: on visit, `el.getAttribute('data-b')` → `desc(id)` → cache
on `el[D]`; no attribute → walk children only. Comment `n.data` starting `p:` → anchor
descriptor; `n.data === '{'` → marker. Event delegation walks from `e.target` up through
`parentElement` and takes the first `el[D].e[type]` (no `closest('[data-on-…]')`). Listener types
are registered with `H.on(type)` the first time a descriptor with that type is visited.

### 5.3 Example: before / after

Source (page):

```html
<script>
  let items = [], draft = ''
  function add() { items.push({ id: Date.now(), text: draft, done: false }); draft = '' }
  function toggle(t) { t.done = !t.done }
  const left = () => items.filter(t => !t.done).length
</script>
<form @submit="add">
  <input :model="draft">
  <button>Add</button>
</form>
<p>Total: {items.length} <span>{left()} left</span></p>
<ul .empty="!items.length">
  <for t in items key=t.id>
    <li .done="t.done"><input type="checkbox" :checked="t.done" @change="toggle(t)"><span>{t.text}</span></li>
  </for>
</ul>
<if !items.length><p>Nothing yet</p></if>
```

Body HTML **before** (today):

```html
<main data-page="todos">
<form data-on-submit="add"><input data-model="state.draft"><button>Add</button></form>
<p>Total: <!--{state.items.length}--><!--}--> <span data-text="{state.left} left"></span></p>
<ul data-class-empty="!state.items.length"><!--p:for t0 t state.items t.id--></ul>
<!--p:if t1 !state.items.length-->
</main>
```

Template module **before**:

```js
export default {t0:'<li data-class-done="t.done"><input type="checkbox" data-attr-checked="t.done" data-on-change="toggle(t)"><span data-text="{t.text}"></span></li>',t1:'<p>Nothing yet</p>'}
```

Body HTML **after**:

```html
<main data-page="todos" data-b="l0">
<form data-b="p0"><input data-b="p1"><button disabled data-b="p2">Add</button></form>
<p data-b="p3">Total: <!--{--><!--}--> <span data-b="p4"></span></p>
<ul data-b="p5"><!--p:p6--></ul>
<!--p:p7-->
</main>
```

`R.t` and `R.b` **after**:

```js
t: {
  t0: '<li data-b="p8"><input type="checkbox" disabled data-b="p9"><span data-b="pa"></span></li>',
  t1: '<p>Nothing yet</p>',
},
b: {
  l0: { x: 2 },
  p0: { e: { submit: [['add'], [0]] } },
  p1: { m: ['state', 'draft'] },
  p2: { d: 1 },
  p3: { b: [[5, ['state', 'items', 'length'], 0, 0]] },
  p4: { b: [[0, ['', ['state', 'left'], ' left']]] },
  p5: { b: [[3, ['state', 'items', 'length'], 1, 'empty']] },
  p6: { t: 2, s: 't0', v: 't', p: ['state', 'items'], k: ['t', 'id'] },
  p7: { t: 1, s: 't1', p: ['state', 'items', 'length'], n: 1 },
  p8: { b: [[3, ['t', 'done'], 0, 'done']] },
  p9: { b: [[2, ['t', 'done'], 0, 'checked']], e: { change: [['toggle'], [['t']]] }, d: 1 },
  pa: { b: [[0, ['', ['t', 'text'], '']]] },
},
use: [branch, list, model],
```

More shapes:

```js
// <article data-c="card" data-prop-step="2" data-with-items="data.items" data-with-open="false" data-text="{state.n}">
{ x: 1, c: 'card', p: { step: '2' }, w: [['items', ['data', 'items']], ['open', { v: false }]], b: [[0, ['', ['state', 'n'], '']]] }
// self call: <ul data-c="tree" data-self="c-tree" data-with-node="c"></ul>
{ x: 1, c: 'tree', self: 'c-tree', w: [['node', ['c']]] }
// slot content one level up: <b data-slot="1" data-text="{state.open}"> ; <!--1{state.title}--> ; <!--p:if1 t3 state.open-->
{ u: 1, b: [[0, ['', ['state', 'open'], '']]] }      // element
[5, ['state', 'title'], 0, 1]                          // marker binding on its parent
{ t: 1, s: 't3', p: ['state', 'open'], n: 0, u: 1 }    // anchor
// @click="shared.cart.add(item, 'x', 2, true, null, event, el)"
{ e: { click: [['shared', 'cart', 'add'], [['item'], { v: 'x' }, { v: 2 }, { v: true }, { v: null }, 0, 1]] } }
// :--size="state.size"
{ b: [[6, ['state', 'size'], 0, 'size']] }
```

### 5.4 SSR (`core/runtime/server/`)

**[contract]** SSR keeps its own format: the compiled **ops** (`compile::ops`). Ops and descriptors
are generated by the compiler from the same tree in the same build, so they cannot disagree. The
server never parses descriptors.

What the compiler changes in ops (the server sees it as opaque `html` strings):

- `html` ops print `data-b="…"` instead of directive attributes, `<!--p:<id>-->` instead of
  today's anchor, and `<!--{-->` instead of `<!--{path}-->`. The `text`, `attr`, `class`, `show`,
  `if`/`elif`/`else`/`for`/`empty`, `comp`, `slot` ops keep their current shapes and paths.
- A control with `d` (§6): its `html` contains ` disabled`; if it also has `:disabled`, the
  `["attr","disabled",…]` op is not emitted (the kernel applies the binding after it enables).
- Bodies inside `for`/`if` ops are written from hoisted bodies with the same writer, so SSR list
  items carry the same `data-b` ids as `R.t`.

What the server code changes:

- `render.js`: no change in op semantics. `instance()` still inserts `data-of`/`data-key` after the
  tag name (works with `data-b` present). `renderHead` unchanged (head keeps legacy attributes).
- `handler.js`: no change (`shellWith`, `pragma-data`, early hints work on the new shell).
- `contract.js`: changes listed in §1.
- Tests and fixtures in `core/runtime/test/server` and the compiler snapshots must be updated to
  the new HTML.

---

## 6. `disabled` for JS-only controls **[contract]**

A control gets `d: 1` and a static `disabled` attribute in the HTML (page HTML, SSR ops and `R.t`
bodies alike) when **all** hold:

1. The source does **not** write a static `disabled` on it (the app owns it; the build never
   touches it, the kernel never enables it).
2. It is one of these elements:
   - `<button>` of any type;
   - `<input type=button|submit|reset|image|checkbox|radio|range|color|file>`;
   - `<select>`.
   Never: text-like `<input>`, `<textarea>`, `<a>`, or any element without a `disabled` property
   (`div`, `li`, `summary`, …).
3. And one of:
   - it has a `data-on-*`, `data-open` or `data-close`, **except** a submit control (`<button>`
     without `type` or with `type=submit`, `<input type=submit|image>`) whose form owner (ancestor
     `<form>`, or `form="id"` resolved in the route) has an `action` attribute and no
     `data-on-submit`: the native submit is a useful fallback, so it stays enabled;
   - or it is a submit control of a form that has `data-on-submit` and no `action` (even with no
     event of its own).

`<form>`, `<a>` and native submits of forms with `action` stay operable before the kernel. No
replay of early clicks (the overlay "early click" handler is removed: `@open` buttons are
`disabled` until `overlay.js` is in).

`:disabled="path"` on a `d` control is allowed: the build still writes `disabled` and drops the SSR
`attr` op for it; the kernel enables, then the binding sets the app's value.

Kernel **[contract for behaviour; code runtime-internal]**, in `visit(el)`, after registering the
descriptor's event types and **before** running its bindings:

```js
if (d.d && !el[ENABLED] && (!d.o || K.overlay)) {
  el[ENABLED] = 1
  el.removeAttribute('disabled')
}
```

- Ancestors are visited first, so a `<form @submit>` has its listener before its submit button
  is enabled.
- Controls with `o` wait for `overlay.js`; when it arrives the kernel runs one whole render, which
  enables them. Controls in a closed `<dialog>`/`[popover]` are enabled when it first opens
  (`reveal` renders its content).
- `el[ENABLED]` makes it once per element: a later `:disabled` binding is never overridden.

---

## 7. Invalidation by owner

### 7.1 Readers **[runtime-internal, behaviour is contract]**

`reads: WeakMap<rawTarget, Map<key, Set<reader>>>`. No `ANY` marker any more. A reader is one of:

- **binding record** `[el, binding, scope]`: one per (element, binding index), kept on
  `el[RECORDS][i]` and **reused** across visits (only `scope` is updated), so a re-visit does not
  add duplicates to the sets;
- **model record** `[el, MODEL, scope]` (`model.js`, kind 7), same reuse;
- **owner record** `{ el, s, o, dp }`: `el` = the element whose visit made a structural read,
  `s` = the scope `visit(el, s)` received (before lexical/with/component), `o` = snapshot of the
  `outer` stack, `dp` = self depth. Kept on `el[OWNER]`, reused, created lazily on the first
  structural read of that element.

What reads under which reader:

| Read | Reader |
| --- | --- |
| a binding of `el` (`bind`) | its binding record |
| `data-model` value | its model record |
| `w` values, component props, component `enter` | owner record of `el` |
| anchor condition, list path, list items and keys (anchors are children of `el`) | owner record of `el` (the anchor's parent element) |

### 7.2 Schedule and flush

- `schedule(t, k)`: for each reader in `reads.get(t)?.get(k)`: if its `el` is disconnected,
  delete it from the set (prune); else add it to `dirty`. **No reader ⇒ nothing to do** (today it
  promotes to a whole render).
- `schedule()` without a key ⇒ whole render. Only these call it: `enter` (boot, navigation),
  replacing `cur.data` (loader result), replacing `layout` (layout load / refresh after an action),
  `overlay.js` arrival, overlay `reveal`, `storage` restore from another tab.
- Event handlers: **no `schedule()` after the handler, and none after its promise**. Writes go
  through the Proxy and schedule their own keys. Same for `model` input (`H.input` writes through
  the Proxy, no extra schedule). `deleteProperty` notifies `(t, k)`.
- `cur.page` (`loading`, `error`) becomes `reactive(...)`, so its writes are tracked like state.

`flush()`:

1. `whole`: `forgetReads()`, `render(document.body, rootScope)` (as today).
2. Else: split `dirty` into owners and records. Owners: keep connected ones, sort in document
   order, drop each owner whose `el` is inside another kept owner's `el`. For each: restore
   `outer = o`, `deep = dp`, `visit(el, s)`. Records: skip those whose `el` is inside a re-visited
   owner; for the others `readBy(r); bind(...r)` (model records through `B[7]`).
3. Both: `H.done()` (motion settle), focus, run pending mounts (after **any** flush that created
   instances, not only whole ones), `H.flushed()` (persist), schedule the next slice if
   `K.slices.pending()`.

Lists: `list.js` keys its deferred slice jobs by anchor; a re-visit of an owner that contains a
list replaces that list's job. A whole render drops all jobs (as today).

Stale readers: sets are pruned when a key is notified, and fully reset on every whole render.
Elements removed while their keys never change again stay in sets until the next whole render;
that bound is accepted.

---

## 8. Build report

`pragma build` prints, per route, first-load JS = entry + its modulepreload list, as raw / gzip /
brotli bytes (static routes print `0`). `PageReport.js_gzip_bytes` uses the same set.

## 9. Work split

| Runtime | Compiler |
| --- | --- |
| New files of §1; remove `boot.js`, `directives.js`, `loader.js` | Detection pass (§4) on `exp.body` + `o.bodies` + scripts + CSS |
| `start`, `install`, `K`/`H`/`B` | Descriptor tables and writers: `data-b`, `<!--p:id-->`, `<!--{-->`, build `disabled` (§5, §6); head writer keeps legacy attributes |
| Descriptor lookup, delegation by `el[D].e`, enable rule (§6) | Ops: same shapes, new `html`; drop `attr disabled` op on `d` controls |
| Owner invalidation (§7) | Entry generation (dev file / rolldown input), remove the boot input and `tpl` module |
| `router.js` with `entry` imports, MPA for `entry: null` | HTML head (§3.2, §3.3), `VT`, manifest `entry`, `.pragma/page.js` replaces the `boot.js` check |
| `persist.js` synchronous, no theme migration; `contract.js` changes | `@view-transition` in `pragma-css` base; build report (§8) |
| Runtime tests and server fixtures | Compiler snapshots |
