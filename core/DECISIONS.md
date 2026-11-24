# pragma decisions

What was decided and why. The **how** is in `SPEC.md`, the v0 contract, which overrides this
file. Changing a **[firm]** decision requires an explicit author decision.

Status: **[firm]** · **[firm v0]** v0 contract, revisable later · **[likely]** ·
**[proposal]** · **[open]** · **[superseded]** · **[discarded]**. All items dated 2026-11-18.

## Summary: the pragma contract

1. Plain JavaScript (ES modules). Never TypeScript or JSDoc types. The top level of a page,
   component or store script is compiled (top-level `let` = state); the rest is the JS you wrote.
2. Fixed folder structure. Every page is a `.html` in `routes/`; its path is the route.
3. Never HTML in JS, not even on the server. Markup lives in `.html`.
4. Closed, boring, deterministic directives: one way to do each thing.
5. Reused components; each page serves the minimum (per-page modules + modulepreload).
6. Pages open fast: prebuilt HTML with skeleton, data via async loader. SSR is opt-in.
7. Overlays always HTML + JS; back closes the overlay; modal and alert lock scroll.
8. CSS: Tailwind v4 subset in Rust; custom classes only for refined cases.
9. OpenTelemetry logs by design; the app does not use `console.*`.
10. Server in JS as a fetch handler; local tool in Rust (`pragma`).
11. Simple, lean, LLM-oriented docs.
12. Agent-first: every form is a WebMCP tool by default; pages can serve markdown to agents.

Empirical basis: `../../pragma-tests_1/COMPARACAO.md` (outside the repo; SvelteKit SPA × astra ×
fable). The native apps shipped about 4× less on first visit (10.8 to 12.3 kB gzip vs 41.5 kB)
and painted in half the time without throttling (108 vs 212 ms). n = 1, one model per app.

## Language

- **[firm]** Never TypeScript: no `.ts`, no TS build, no optional support.
- **[firm]** No JSDoc types either (`@type`, `@param {…}`, `@typedef`, `@ts-*`, `tsc`).
- **[firm]** No comments anywhere: app and framework (JS, HTML, CSS, Rust). Code is the
  documentation. In the app, comments and vague names are `pragma check` errors (`*/comment`,
  `js/vague-name`). *Why:* comments rot and spend context; good names don't.
- **[firm]** English everywhere: messages, docs, identifiers, UI text. Generated `<html lang>`
  comes from `pragma.json` `"lang"` (default `"en"`).

## HTML, directives and components

- **[firm]** Markup only in `.html`. No `innerHTML`, `createElement`, `h()` or template
  literals with markup. *Why:* agents master HTML and CSS, and they are easiest to review.
- **[firm]** Pages prebuilt at build; JS attaches behavior to existing nodes. Re-rendering what
  arrived ready is forbidden (no hydration that rebuilds the DOM).
- **[firm]** Closed, boring, deterministic directives, Go style (SPEC §3): `root.a.b` paths
  without expressions; anything not listed is an error. *Why:* one way to do each thing
  reduces agent variance. Settles "how JS finds nodes" and "component syntax".
- **[firm]** Reuse components. `<c-name>` is expanded at build (macro), with literal props and
  slots; the component `.js` loads only on pages using it (SPEC §3).
- **[firm]** (2026-11-21) Literal values: `:attr`, `:--name` and props (`:prop`) take `true`,
  `false`, `null`, a number (`-?\d+(\.\d+)?`) or a quoted string besides a path. The build
  resolves them: a static attribute or `style="--x:…"` on elements, a build-time prop for
  components without a script, and a JSON value in `data-with-*` (typed in the browser and in
  SSR) for components with a script. *Why:* `:comferiado="false"` was read as the path `false`
  (undefined) and the component got its default; `comferiado="false"` (text) only worked because
  the default was a boolean. `<if>`, `:show`, `.class` and `:model` stay path-only.
- **[firm]** User data and dynamic lists come from blocks; updates diff by key.
- **[firm]** Blocks are source tags: `<if p>`, `<else-if p>`, `<else>`,
  `<for x in p key=x.id>`, `<empty>`, path unquoted. The build emits `<template data-*>`.
  *Why:* `<template :if>` was noise and, without `else-if`, loading/error/content became a
  pyramid of wrapper `<div>`s. Svelte's model (chained branches, empty in for) but as HTML
  tags: the tree stays HTML and agents don't pull in Svelte vocabulary (expressions, `{x}`).
  `<template>` written in source is an error.
- **[discarded]** Svelte-style `{#if}`/`{:else}` and `<if test="p">`: the first is no longer
  HTML; the second is noise.
- **[firm v0]** Each block keeps exactly 1 root element.
- **[firm v0]** No `:class` (bypassed the class check, CSS generation and overwrote `.x`
  toggles): static `class` plus `.name="path"` per class. `{{ }}`/`${ }` in text or attributes
  are `html/interpolation` errors (allowed as text directly inside `<code>`/`<pre>`).
- **[firm]** Text interpolation `{path}` is the only way to put a value in text, and it also
  works inside attribute strings (`alt="Photo of {p.name}"`); `:text` was removed
  (`html/text-removed`), `:attr` stays for whole and boolean values (2026-11-19, proposal
  `single-file-components.md`). Only paths (and derived functions, called on each render), no
  expressions; always escaped; literal in text directly inside `<code>`/`<pre>`, `\{` elsewhere. *Why:*
  `<p :text="$count"></p>` hid the text from the reader and split a sentence into many spans;
  `{}` keeps the sentence readable and is what agents already write. It stays HTML because the
  value is a path, never an expression (the reason `{#if}` was discarded still holds).
  *How:* an element with only text gets a `data-text` template; text next to child elements
  gets invisible comment markers (`<!--{path}-->…<!--}-->`), so no wrapper element changes the
  DOM or the CSS; SSR fills both; the prebuilt HTML never shows a raw `{…}`.
- **[firm]** Inside `<pre>`/`<code>` everything is processed (directives, blocks, components);
  only text whose nearest parent element is `<pre>` or `<code>` keeps `{}` literal, so
  `<code>{x}</code>` shows code and `<pre><span>{$path}</span></pre>` shows a value
  (2026-11-19, site BUGS.md BUG-S-16). *Why:* skipping the whole subtree sent `<for>`, `:attr`
  and `@event` raw to the browser and `check` stayed silent; dynamic code and terminal output
  (highlighted lines, a requested path) had no syntax at all.
- **[firm]** Events call with arguments: `@change="toggle(task)"`, `@input="search(event, $q)"`.
  Arguments are paths, simple literals or `event`; no expressions. *Why:* replaces
  `ctx.scope.task`, which made the agent guess the `<for>` variable name inside the action.
- **[firm]** (DX review, 2026-11-19) Without parentheses a handler gets the event
  (`@input="search"` → `search(event)`); with them only what is listed. New argument `el` (the
  element with the `@`), because events are delegated and `event.currentTarget` is the document.
  Store functions are handlers too (`@click="shared.cart.remove(item.id)"`), so components need no
  wrapper actions. *Why:* every other framework passes the event by default; `fn` with no args
  was the most common first-try bug.
- **[superseded]** by "Scoped CSS matches only the owner's markup" (2026-11-21). (DX review,
  2026-11-19) Component and page CSS is scoped by the build: each selector
  gets `:where(<scope>, <scope> *)` on its last part (no specificity added), unless it already
  names the scope; `:global(…)` opts out. *Why:* `button { color: red }` in a component shipped as
  a global rule and nothing warned; agents from Svelte/Vue expect scoping. The hand-written
  `[data-c="x"]` prefix keeps working, so existing CSS does not change (the site's global CSS
  stayed at 14.8 kB gzip).
- **[firm]** (BUG-S-21, 2026-11-19) `:global(…)` opts out only what is inside it (Svelte
  semantics): `:global(html[data-theme="dark"]) .row` keeps `.row` scoped; only a selector whose
  every compound is `:global(…)` is global. *Why:* the old rule unscoped the whole selector, so the
  documented dark-mode example colored every `.row` of the app.
- **[firm v0]** Prop truthiness is the same in HTML and JS: missing, `""`, `"false"` are falsy;
  bare attribute and other values truthy. In `ctx.$`, `"true"`/bare → `true`, `"false"` →
  `false`, other values stay strings.
- **[firm]** Overlays (modal, alert, popup, dropdown) are always HTML + JS. Back (browser or
  Android) closes the overlay instead of leaving the page. Modal and alert lock scroll (SPEC §6).
- **[firm]** (2026-11-21) `c-dropdown placement="bottom-start|bottom-end|top-start|top-end"`
  (side, then shared edge; Floating UI names), with automatic flip to the other side or edge.
  CSS anchor positioning (`position-area` + `position-try-fallbacks`, set inline by the runtime,
  so it follows scroll and resize); the old measured placement stays as the fallback where
  `position-try-fallbacks` is unsupported. *Why:* the source menus open upward and right-aligned
  next to a send button; one closed prop beats free-form CSS per menu.
- **[firm]** (2026-11-21) Styled select is `c-select` (starter): `c-dropdown` plus a hidden native
  `<select name>`, so `FormData` and WebMCP get the value. A component may pass a tool attribute on
  with `:toolparamdescription="$prop"`; the caller's fixed text is written at build time, a bound
  value is `webmcp/dynamic`. AGENTS.md: rules do not change the request, target is current Chrome,
  Edge, Firefox and Safari, and limits are told before coding. *Why:* an agent asked for "a dropdown
  with HTML" shipped `appearance: base-select` (Chrome/Edge only) because it was "the simplest code
  that passes `pragma check`" and `c-dropdown` sends no form value.
- **[firm]** (2026-11-22) Block bodies once per page: each block `<template>` is an empty anchor
  and its body goes to one `<template data-tpl="<tid>">` per page; the tid is a hash of the body.
  *Why:* a cloned list item or self call carried every inner block body (one chat line of
  escalei `/app/[id]` cloned 66 kB and 234 nested templates; now 220 B), and equal bodies from
  repeated components are emitted once (365 → 317 kB). Separate `l`/`t` prefixes keep the
  persistent layout and the swapped page from sharing an id.
- **[firm]** (2026-11-22) Block anchors are comments, `<!--p:<kind>[slot] <tid> <values>-->`
  (values percent-encoded outside `[A-Za-z0-9_$.!]`), and every block and self-call body goes to
  one template module per page, `/assets/tpl/<route>-<hash>.js` (`export default { tid: 'html' }`,
  `d` for self calls). The page modulepreloads it; the runtime imports it with the route modules
  and parses a body on its first render (in the SVG/MathML namespace when the anchor sits there).
  Tids are `l`/`t` + a base36 counter per page; `l` ids resolve in the module the page booted with
  (the layout stays), `t` ids in the current page's, so pages never collide. *Why:* escalei
  `/app/[id]` shipped 317 kB of HTML whose DOM held 1442 `<template>` elements next to 546 visible
  ones; now 14 kB of HTML and none. A JS module, not `.html`: Workers redirects `*.html` to the
  clean URL, and modules get modulepreload, the router's prefetch and the module cache for free.
  No inline threshold for small pages: one path in build and runtime, and the extra request runs
  in parallel with the route chunk the first render already waits for. Instances must follow their
  anchor (text nodes aside), since a comment does not stop `nextElementSibling` the way a
  `<template>` did.
- **[firm]** (2026-11-21) A component may render itself (Svelte's `<svelte:self>`): only
  directly, only inside `<if>`/`<for>`, only with a `<script>` (live props), never filling slots.
  The build expands the body once more into a `<template data-def>`; each self call is an empty
  root with `data-self`, filled by the runtime from it (guard: 64 levels). The server renders 12
  levels (64 self calls per page) from the same ops; deeper levels render on the client. *Why:*
  markdown and comment trees need recursion; components are build-time macros, so the open end must
  be a runtime clone, and the closed rules turn infinite loops into build errors.
- **[superseded]** "Component syntax" and "how JS finds nodes" (previously open): solved by
  directives.

## Structure and routes

- **[firm]** The app folder structure is fixed (SPEC §2).
- **[firm]** Every visible page is a `.html` in `routes/`; the file path is the route.
- **[firm]** `index.html` serves its folder; `x.html` plus `x/index.html` is a build error;
  `[name]` is a path param; static beats param; a `_` prefix excludes from routing.
- **[firm]** Path params and query string on client and server.
- **[firm v0]** Client router: swaps `<main data-page>` with the route's prebuilt HTML, with
  prefetch on hover, focus and viewport (SPEC §7). Replaces the open "MPA vs router".
- **[firm]** (2026-11-23) The router marks the clicked link `aria-current="page"` at the click,
  before the next page loads; the swap marks again. *Why:* a sidebar marker moved only 60–90 ms
  after the click (v3chat). A failed navigation is a full page load, so nothing reverts.

## Data and SSR

- **[firm]** Direction: pages open fast and fetch what they need via async load and loaders,
  showing a skeleton (SPEC §5).
- **[firm]** No SSR by default. Data in `routes/<route>.server.js` with `load(...)`; the client
  fetches `/_data/<path>`, started by an inline `<head>` script before modules.
- **[firm]** SSR is per-page opt-in with `export const ssr = true`; the client binds directives
  without rebuilding the DOM.
- **[firm]** Never HTML in JS, not even in SSR: the server renders from the same `.html`,
  compiled to ops (embedded in `server/worker.js`).
- **[firm v0]** With `ssr`, the server runs page and component scripts (BUG-S-17, 2026-11-19):
  the compiled factory runs once per request with read-only `data`/`params`/`query`/`page`
  (stores and browser services throw), so initial `let`s, props and derived values are in the
  first HTML; `mount` and events stay in the browser. A script whose imports are not all app
  code (npm package, `core/`), that throws on the server, or that gets a prop from a store is
  left to the browser for that page or component only. *Why:* adding one `let` to a component
  moved all its prop-driven content out of the SSR HTML (empty list until boot, nothing without
  JS); running the same factory is the only way the server and the first client render agree.
  Rejected: resolving only props on the server (derived values and initial state would still
  flash), and a server-only variant of the script (two ways to write one component).
- **[firm v0]** Server writes via `.server.js` `actions` and `call` (from `pragma`) →
  `POST /_action/<path>/<name>`.
- **[firm v0]** Per-page head (SEO, link previews): a page writes `<title>`, `<meta>` and
  `<link>` at its top level; the build moves them to `<head>`, where they replace the layout tag
  with the same key (`name`/`property`/`http-equiv`, `rel`+`hreflang`). Values from the server
  use the existing bindings (`:text`, `:content`, `:href`) on `data`, `params`, `query`, and
  require `ssr = true`: the handler fills them in the shell, so bots without JS see them. The
  layout `<head>` is static (directives = error); head tags elsewhere = error; `<title :text>`
  no longer silently drops the binding. The router swaps head tags on navigation. *Why:* no new
  syntax (head tags are HTML in the page, bindings are the closed list); a fixed "canonical in
  the layout" pointed every page at `/`; client-only head updates are invisible to Facebook, X,
  Slack and LinkedIn previews, so a binding that cannot reach the first HTML is rejected
  instead of shipping half-working. Discarded: an exported `head()`/`meta` object in
  `.server.js` (HTML in JS, a second way to write tags), a page `<head>` block (one more
  structure rule for the same thing), evaluating head bindings on the client.
- **[firm v0]** Request context, redirects, errors, middleware, endpoints (2026-11-21, for
  apps such as escalei: cookie sessions, auth redirects, R2 files, NDJSON, WebSocket voice).
  `load`, actions, endpoints and middleware get one context: `cookies` (defaults `httpOnly`,
  `secure`, `sameSite=lax`, `path=/`; `__Host-` enforced), `setHeaders`, `waitUntil`, `locals`,
  plus `request`, `url`, `params`, `query`, `env`, `log`. `redirect`/`error` come from
  `'pragma/server'` (the build rewrites it to `core/server/http.js`) and throw. Wire for
  `/_data` and `/_action`: redirect = 200 `{"redirect":{status,location}}` (a 30x would be
  followed silently by `fetch`), error = its status with `{"error": body}`; full documents get a
  real 30x. Cookies and headers apply to every outgoing response, raw ones included. One
  `routes/_middleware.server.js` `handle({ ..., next })` runs before every request except
  `/assets/*`, `/_pragma/*`, `/_otel`; `url` is the page path also for `/_data`/`/_action`, so a
  path check cannot be bypassed through the internal URLs. A `.server.js` without `.html` is an
  endpoint with uppercase method handlers returning a `Response` (files, ranges, streams,
  webhooks; WebSocket only on Workers). *Why:* SvelteKit's `cookies`/`setHeaders`/`locals`/
  `handle` proved the shape; pragma keeps one middleware (no `sequence`), one context object,
  no `+server`/`+page` split. Page data still never goes through endpoints: `app/endpoint-json`
  warns on `Response.json` and a non-`Response` return is a 500 naming the file.
- **[firm v0]** Action and load results keep status and body (2026-11-21): errors answer the
  real 4xx/5xx with `{ error }` and `call`/`stream` throw `ActionError` with `status` and `body`;
  redirects travel as 200 `{ redirect: { status, location } }` and the client router follows
  them (action: push, like SvelteKit `applyAction`; `load`: replace, so Back skips the
  redirecting URL). `stream()` reads NDJSON from an action that returns a `Response`. *Why:* a
  409 with the conflicting slots or an AI agent reply streamed line by line was lost behind a
  generic message; a real 3xx on `fetch` is followed invisibly and the URL never changes.
  Rejected: a SvelteKit-style `deserialize` envelope for every result (one more shape to learn)
  and SSE (`EventSource` cannot POST).
- **[superseded]** SSR via `HTMLRewriter` on Workers and `lol_html` in `pragma serve`: replaced
  by compiled ops rendered by the JS handler.
- **[superseded]** "No SSR (SPA + API)".
- **[firm]** (2026-11-23) The build renders every prebuilt page once with the SSR renderer
  (`server/render.js`) in QuickJS embedded in the CLI (rquickjs; no Node to build): page and
  component scripts run with no request, so initial `let`s, props, derived values and the blocks
  they decide are in the HTML, layout included. `data`, `layout`, `params`, `query` and `page`
  are unknown at build (bindings on them stay to the browser; a script that reads them at the
  top falls back like SSR). `Date`, `Math.random`, `toLocale*` and `localeCompare` throw at
  build, so their scripts fall back too (stale or wrong-locale first paint, non-deterministic
  builds). A failed render keeps the unrendered HTML. 2 s and 256 MB per page. *Why:* the first
  paint must equal the state after boot; a `<c-logo size="sm">` painted at the default size and
  shrank at boot. Rejected: resolving only literal props in Rust (derived values and blocks still
  flash), Node at build (breaks "one binary").
- **[firm]** (2026-11-23) Page and component scripts that import npm packages run in SSR and
  in the build prerender. A script is still left to the browser when it reaches a `core/` module,
  throws, or a package it imports throws on import. On the server the package comes from
  `node_modules` (Workers: `wrangler` bundles it) and the script is imported on its first request
  (`() => import()` in `worker.js`), so a package that reads `window` at import time disables
  only that script, not the server. At build the prerender (QuickJS) resolves the package to its
  client bundle and imports each script with `await import().catch()`, so one failing script
  does not drop the whole page render. *Why:* v3chat `c/[id]` renders markdown with `marked`
  (pure, no DOM); because of that one import the messages never reached the first HTML, and LCP
  waited for `boot.js` → `/_data` → render. Rejected: an allowlist of "server-safe" packages
  (one more list to keep), and keeping npm scripts browser-only (most formatters and parsers
  are pure).
- **[firm]** (2026-11-23) `routes/_layout.server.js` exports only `load`; its result is
  `layout` (HTML root and `import { layout } from 'pragma'`), fetched from `/_layout/<path>` (an
  inline `<head>` fetch, like `/_data`), embedded with `ssr`, kept across client navigations and
  fetched again after each `call`/`stream`. *Why:* the signed-in user in a sidebar read the
  page's `data`, which is empty during every navigation, so the sidebar blinked. Rejected:
  keeping the previous page's `data` until the next arrives (another page's shape, wrong values
  for a moment), a store fed by an endpoint (one more API per app). `layout/data-in-layout`
  warns on a component in the layout whose script reads `data`.

## Motion

- **[firm v0]** Block enter/leave animations are CSS on runtime attributes (2026-11-21):
  `data-entering` on a block root inserted after the block's first render, `data-leaving` (+
  `inert`) on a removed one, which stays until its finite CSS animations end
  (`Animation.finished`, timeout = computed end + 50 ms). No rule → immediate removal;
  `prefers-reduced-motion` → no wait. Same ideas as Svelte `in:`/`out:` (no intro on first
  render, outro keeps the node and makes it inert, a block that comes back reuses the node), no
  JS animation engine and no new syntax. *Why:* ported apps use ~110 transitions; `@starting-style`
  covers entry but nothing covers exit without keeping the node. Rejected: a `transition`
  directive (closed syntax, JS easing), `animationend`/`transitionend` listeners (miss
  descendants and transitions that never start).
- **[firm]** (2026-11-21) `c-modal`/`c-alert` leave the same way: closing sets `data-leaving` +
  `inert` on the `<dialog>`, waits for its CSS animations (same helper as blocks, `::backdrop`
  included), then calls `close()`. Esc goes through `cancel` (prevented, then the same path). The
  overlay leaves the stack when closing starts (history, scroll lock, `confirm()`), like Svelte's
  `onoutrostart`; focus returns after `close()` (the opener is inert while the dialog is modal).
  Opening again while leaving closes it at once and reopens. `autofocus` on the component focuses
  the dialog itself (WAI-ARIA, text-first content); otherwise the first focusable element.
  *Why:* the ported app's sheets and modals slide/fade out; a native `<dialog>` vanished at once.
- **[firm]** (2026-11-23) Page transitions: `<main data-page>` has `view-transition-name: page`;
  only that group fades in (150 ms from opacity 0.5); `root` (the layout) swaps without
  animation. The old snapshots stay hidden (Chromium can paint an empty old snapshot white).
  *Why:* the whole document faded from opacity 0, so a fixed sidebar blinked on every navigation.
- **[firm]** (2026-11-23) No directive for the first scroll position (`data-scroll="end"`): a
  reverse-column scroll container opens at its end before the first paint (`css.md`). *Why:* CSS
  already does it, and the syntax stays closed.

## CSS

- **[firm]** (2026-11-23) CSS vendor prefixes come from the build: lightningcss runs with a fixed
  baseline (Chrome/Edge 111, Firefox 128, Safari/iOS 16.4) that only adds prefixes (every other
  feature is excluded, so nothing is compiled down), and a hand-written prefixed property with a
  standard version is `css/vendor-prefix` (prefix-only properties such as `-webkit-line-clamp`
  are allowed). *Why:* v3chat wrote `backdrop-filter` followed by `-webkit-backdrop-filter`; the
  minifier merged them and kept only the prefixed one, so the blur vanished in Chrome and Firefox
  only in production. lightningcss 1.0.0-alpha.72 does this even with targets (its fallback
  handler replaces the standard declaration with a later prefixed one), so the check rule is what
  closes it. Council: 3×0 it is a bug; targets and the rule 2×1. Rejected: a general warning with
  no targets, patching the merge in lightningcss (astra's preference).
- **[firm]** Reuse CSS as much as possible.
- **[firm]** Tailwind v4 subset reimplemented in Rust (`pragma-css` crate), generating only used
  classes, no arbitrary values. Goal: fewer classes; custom classes allowed for refined cases
  (SPEC §8). *Why:* CSS stops growing, agents already know the names, no naming decisions.
- **[firm v0]** `base.css` (preflight + layers) and `lib/theme.css` (tokens and custom
  classes); dark theme via `[data-theme="dark"]` and `dark:`.
- **[firm]** (2026-11-21) Scoped CSS matches only the owner's markup (Svelte semantics): the build
  writes a `data-s` token on each element a page or component wrote (only on elements one of its
  selectors can match) and appends `:where([data-s~="<token>"])` to each selector's subject. Child
  component markup and slot content (which belongs to its writer) are not matched; `:global(…)`
  opts out as before; a component root that gets `class` from its caller also carries the
  caller's token. *Why:* the escalei port (Svelte) had to rename about 15 owners' classes
  (`.titulo`, `.lista`) because page and component styles leaked into child components.
  *Cost:* slotted items styled by a component need `:global(…)` (starter dropdown updated); HTML
  grows about 7% gzip on a component-heavy page (escalei `/app/[id]`: 56.1 to 59.8 kB).
- **[firm]** (2026-11-22) The `data-s` token goes only where a selector of its owner can match,
  ancestors included. After the page is expanded (layout, slots and block bodies in place), each
  token is checked against its owner's selectors: the own compound needs the element's name,
  classes and `.x` toggles (`:is()`/`:where()` need one alternative); each compound to its left
  needs an ancestor on the page, nearest first. When in doubt it keeps the token: `>` counts as any
  ancestor, sibling compounds are skipped, `:not()`/`:has()`/attributes/pseudo-classes never rule
  out, `&` or `|` keep it everywhere, the body of a self-rendering component has unknown ancestors,
  and an ancestor compound that names `html`, `body`, `:root`, a tag or a class the page never
  writes may be matched outside pragma's markup. *Why:* escalei `/app/[id]` wrote 1641 tokens;
  `.tabela td` put one on every `td` of the component. Now 1375 (28.0 to 23.5 kB raw; gzip 54.9 to
  54.6 kB, brotli 42.1 to 41.9 kB); the rest are elements the styles do target. Computed styles
  of every element (block bodies expanded) and screenshots of all 15 escalei pages are identical. A shorter token value (4 hex) would save about
  0.3 kB gzip and needs an app-wide collision check, so it stays 6.
- **[firm]** (2026-11-21) Cascade: `reset, tokens, base, components, utilities` layers, then
  component and page CSS outside any layer. Theme classes < utilities < scoped styles, as with
  Tailwind v4 + Svelte. *Why:* in the port about 20 components had to move utilities into their
  CSS, because a utility on the element beat the component's state, hover and media rules.
- **[firm]** (2026-11-21) Utilities and project classes with the same name coexist: the utility is
  always generated. A `lib/theme.css` rule whose whole selector is a utility name (`.flex`,
  `.hidden:hover`) is `css/utility-name`. *Why:* a selector like `.heroi > .contents` silently
  removed `contents` (and `mr-auto`, `overflow-y-auto`) for the whole app.
- **[firm]** (2026-11-21) Per-route CSS: one shared file (base + layout utilities) and one route
  file with the theme rules whose selectors can match the route's markup (element names and
  classes, static or `.x`; attribute, id and pseudo-class parts never drop a rule), the
  component and page CSS of the route without rules for classes their own elements never have,
  and the route's utilities. Both inline when their gzip sum is <= 14 kB. *Why:* the theme (46 kB
  in escalei) shipped whole and render-blocking on every page.
- **[firm]** (2026-11-21) Automatic font preload per route: for each `@font-face` family the
  route's CSS uses (`font-family`/`font`, through `var()` chains, only rules that can match), one
  `<link rel="preload" as="font" crossorigin>` for the upright `.woff2` that covers Latin letters
  (weight 400 when there are several). Manual preloads in `_layout.html` should be removed
  (not repeated when present). *Why:* the layout preloaded 130 kB of fonts the landing did not
  use. Metric overrides (`size-adjust`) for fallbacks are not generated.
- **[firm]** (2026-11-22) CSS `url()` files are hashed assets. In production, each `url(/path)` in
  the emitted CSS (shared and route, inline and external) that names a file in `public/` points to
  a copy at `/assets/<folder>/<name>-<hash8>.<ext>` (URL-safe, same content hash and compression
  policy as other assets: `.br`/`.gz` only for text types, so not `.woff2`); the font preloads
  follow, and the file stays at its `public/` URL. `data:`, `http(s):`, `//`, `#`, relative paths
  and paths with no file in `public/` stay as written, with no diagnostic (a path can be an
  endpoint). `pragma dev` keeps the `public/` URL. The Workers handler now sets
  `public, max-age=31536000, immutable` on `/assets/*` (Workers static assets answer
  `max-age=0, must-revalidate`; `serve` and Node already did). *Why:* escalei served fonts from
  `/fonts/` with `max-age=0, must-revalidate`; every visit revalidated them (304) before use, and
  since pragma paints early (inline CSS) the hero showed the fallback font and swapped on every
  visit. SvelteKit/Vite hashes `url()` files the same way. Relative `url()` is not resolved: it
  means a different file inline (page URL) and external (stylesheet URL).
- **[firm]** (2026-11-21) Computed values (bar position, width, `--cols`, `--i/--n`) go through CSS
  custom properties: `:--name="path"` sets `--name` (Svelte `style:--name`); the CSS reads
  `var(--name)`. `false`/`null`/`undefined`/`""` remove it; a value with `;`, `{`, `}` or `<` is
  refused (dev throws). Inline `style`, `:style` and arbitrary classes stay errors. *Why:* the style
  stays in CSS files, one variable per value is easy for agents, and the value cannot add
  declarations. The server writes `style="--name:value"` (HTML-escaped).
- **[superseded]** "No Tailwind, preflight only" and "Tailwind v4 vs own CSS".
- **[superseded]** Per-route CSS: v0 emits one global CSS (per-route CSS is out of v0).
- **[superseded]** One global hashed CSS with the whole theme (replaced by per-route CSS, 2026-11-21).

## Reactivity and state

- **[firm]** Single-file components (2026-11-19, proposal `single-file-components.md`): a page
  or component `.html` holds one `<script>` (always an ES module; no `src`, no classic script)
  and one `<style>` (handled like the component `.css`). Separate `x.js` / `x.css` stay valid;
  both for the same file is `html/duplicate-module`. `.server.js` stays separate. *Why:* one
  file per component is what agents open and edit; the markup and its behavior stay together.
- **[firm]** The top-level script is the component; no `export default`, `this` or `ctx`.
  Top-level `let` = reactive state; a top-level function used in `@event` is an action, used in
  `{}` or a value binding a derived value (called on each render; in JS you call it: `total()`);
  `function mount(el) { …; return cleanup }` runs after mount with the component's root element
  (a page: `<main data-page>`; BUG-S-18, it replaces `ctx.el`); imports are hoisted out of the
  per-instance factory. State is a deep Proxy (`items.push()` works) and every change schedules
  one render per microtask, so assigning a `let` from a timer just works (no `ctx.update`).
  *Why:* the old API split one idea into `export const state`, `export const actions` and
  `ctx.$.x`, and agents kept guessing `ctx` members (`ctx.state`, `ctx.props`, `ctx.scope`);
  plain declarations are the JS every model already writes, and it is the Svelte 4 model they
  know.
- **[firm]** This changes the rule "plain JS, no compile magic" (until now the build shipped app
  JS as written and only rewrote import URLs). The build now rewrites the top level of page,
  component and store scripts: reads and writes of a top-level `let` become `$.x`, `export let`
  becomes a prop, names imported from `pragma` become services, and the module becomes a
  per-instance factory. *Why it is acceptable:* the rewrite is mechanical and scope-resolved
  (oxc semantic: a shadowing parameter is left alone), it keeps every line in place (diagnostics
  and DevTools lines match the source), it touches only top-level declarations, and anything it
  cannot rewrite is a `check` error (`js/component-script`) instead of a silent difference. All
  other JS (utils, `.server.js`, inner functions) is exactly what was written.
- **[firm]** Props, Svelte 4 style: `export let step = 1`. The default's type drives coercion of
  attribute text (`"5"` → 5; bare/`"true"`/`"false"` → boolean); props are read-only
  (`js/prop-write`) and live (`:items="$visible"` passes the caller's value, not text). An
  undeclared prop at a call site is `html/unknown-prop`. `export` means "prop" and nothing else.
- **[firm]** Object and array props are read-only all the way down (DX council, 2026-11-19): in
  dev a prop object is a read-only view that throws on write (production skips the view).
  *Why:* writing into a prop object silently changed the caller's data while `js/prop-write`
  only stopped reassigning the name; copying is the explicit way to own a value.
- **[firm]** Traps of "top-level `let` is reactive" are `check` errors, not docs (DX council):
  `js/stale-derived` (a value computed once from state at startup), `js/derived-call` (a
  derived function used as a value without `()`), `js/top-level-data` (`data` before the
  loader), `js/dollar-in-script` (`$x` in JS). All script errors of a file come in one pass
  with "did you mean" hints; only the old API stops the HTML name checks. `async mount()`
  resolves to its cleanup. *Why:* the three reviewers hit each trap and `check` passed; an
  agent fixes what `check` names.
- **[firm]** Services come from the virtual module `pragma`: `shared`, `data`, `params`, `query`,
  `page`, `log`, `call`, `stream`, `navigate`, `open`, `close`, `confirm`, `frame`. They are live (`data` is the
  current page's once the loader is done). Only page, component and store scripts import it.
- **[firm]** Stores follow the same rule (`lib/state/x.js`): top-level `let` = state, functions =
  actions or derived, all reached as `shared.x.*`; `export const persist = ['items']` keeps those
  fields in `localStorage['pragma:x']`. A direct `import` of a store is `js/store-import`: one way
  to reach it, and the build knows which pages load it.
- **[firm]** Old API (`export const state`/`actions`, `ctx.$`, `ctx.update`, `store()`) is a
  `check` error (`js/legacy-api`) with the migration hint; there is no compatibility layer.
- **[superseded]** v0 `state(ctx)` + `actions` + `ctx.$` and `store(initial, { persist })`:
  actions mutated state and the runtime reapplied directives; derived values were getters in
  `state(ctx)` or the store.
- `tests_1` finding: signals vs store + `dispatch` barely matters (1.5 kB gzip difference;
  3 to 10 ms without throttling and 20 to 40 ms at 4× CPU over 500 tasks, wins on both
  sides). In fable, "writing a signal inside an effect throws" caught a real bug: a sync
  `blur` when hiding the focused field triggered a write inside the effect.

## Performance

- **[firm]** The app must be very fast. Each page serves the minimum: per-page and
  per-component modules, all `modulepreload`ed (SPEC §9).
- **[superseded]** No bundler: each module minified (oxc) and hashed; `immutable` assets;
  uncached HTML.
- **[firm]** (2026-11-21) Route chunks in `pragma build` (rolldown, already used for npm): the
  core runtime is one chunk; each route is one chunk (page, components, stores, single-route
  code); code used by 2+ routes is grouped by the exact set of routes, and a group under 4 kB
  joins the closest group of a wider set (no page loads a chunk it would not load anyway).
  `import()` stays separate; npm files stay in `assets/npm/` as externals. The route JSON points
  to `chunk.js#p` / `#c<i>` / `#s<i>` (named re-exports, no namespace helper). `boot.js` has no
  top-level await (rolldown turns off chunk merging with TLA). `pragma dev` keeps one file per
  module. Measured on escalei (prod, gzip JS): `/` 20 files 23.4 kB → 5 files 20.3 kB, `/entrar`
  14 / 16.7 → 6 / 14.7, `/app/[id]` 99 / 186.0 → 10 / 153.2. *Why:* ~0.6 kB modules cost one
  request each (115 requests on `/app/[id]`). The router also watches links added after load
  (MutationObserver) for viewport prefetch, so client-rendered `/app/<id>` lists get prefetched.
- **[superseded]** Budgets in `pragma.json` (`initialJsKb`, `routeJsKb`, `cssKb`, per-route
  `budgets.routes`), checked by build and check.
- **[firm]** (2026-11-21) Binding-only updates: the full render records which bindings read each
  reactive (object, key) (`state.js`); a change read only by `{}`, `:attr`, `:class`, `:show` or
  `:--name` (also through derived functions) re-runs just those bindings in the next microtask.
  A key read by `<if>`, `<for>`, `:model`, props or components, a key not read by the last render,
  and any change without a key (events, loads, navigation) still render the page. Page `data` is
  read through the reactive proxy so its reads are tracked too. Measured: 60 `:--x` frames on a
  1000-row page, 49 ms → 3 ms (full render +15% for the bookkeeping). *Why:* per-frame animations
  (hero scroll, timeline drag, 520 ms send) walked the whole page every frame. Trade-off (same as
  Svelte): a derived function that reads non-state values (`Date.now()`) refreshes only on a full
  render or when state it reads changes.
- **[firm]** (2026-11-21) Updates stay microtask-batched; no automatic rAF deferral. *Why:* code
  that waits with `queueMicrotask` and then measures expects the new DOM (escalei
  `depoisDoRender`, the client tests), and a change made inside a `requestAnimationFrame`
  callback would land one frame late. Changes in one task already make one DOM pass, and a patch
  costs ~0.1 ms (60 `:--x` frames, 1000 rows). The real per-frame cost is read/write
  interleaving: each rAF callback writes when it returns, so the next one's
  `getBoundingClientRect` forces layout. Instead, `frame(fn)` from `'pragma'` runs all frame
  callbacks of the page in one rAF and writes their changes once at the end. Measured (10
  callbacks that read layout and set `:--x`, 1000 rows): rAF 42–50 ms/frame and 10 style writes,
  `frame` 4.3 ms and 1 write.
- **[firm]** (2026-11-21) Per-frame `var()` in layout properties is documented (`css.md`, "Values
  that change every frame"), not a `check` warning. *Why:* knowing that a value changes every
  frame needs data flow from `frame`/rAF/scroll handlers to the bound `let`, through derived
  functions and object fields; the escalei hero sets layout variables once per resize inside the
  same per-frame object, so a static rule would warn on correct code and miss the moving ones.
- **[firm]** Budgets removed (2026-11-21): they broke builds on every runtime growth; size is
  reported, not enforced. `pragma build` prints the JS gzip size per page. A `budgets` key in an
  older `pragma.json` is accepted and ignored.
- **[superseded]** Per-route chunks with a bundler and chunk manifest: v0 uses loose modules
  and the `/_pragma/manifest.json` manifest. (Route chunks came back on 2026-11-21, above; the
  manifest stays.)
- **[firm]** (2026-11-23) Font preload follows the faces the page uses: for each used family,
  the Latin `.woff2` of each style and weight that the rules naming the family set, plus italic
  and 700 for every family when a rule without a family or a UA-styled tag (`em`, `i`,
  `strong`, `h1`…) needs them; at most six. *Why:* only the upright 400 face was preloaded, so
  an italic wordmark in the layout swapped fonts after the first paint (v3chat). The markup it
  reads changed on 2026-11-23 (below).
- **[firm]** (2026-11-23) Font preload reads the prebuilt HTML, not the route's source markup:
  tags and classes of the HTML the build writes after the QuickJS render (layout included),
  without block bodies left as anchors and without the children of `<dialog>` and `[popover]`.
  Same face rules and limit of six. *Why:* v3chat `c/[id]` preloaded 5 faces (~374 KB) on the
  critical path: the italic for `em` in answers that render only after `/_data`, and faces used
  only in a closed dialog or the sidebar drawer. A face for later content swaps when it appears,
  which is the cost of not blocking the first paint. Rejected: an opt-out per `@font-face`
  (a comment is not allowed; a descriptor is one more thing to know).
- **[firm]** (2026-11-23) Italic and 700 from `em`/`strong`/`h1`… (and from a matching rule
  without a family) go only to the base text family, the one a `:root`/`html`/`body`/`*` rule
  names, else the first used family; at most three preloads, the base upright 400 first.
  *Why:* after 0.2.7 v3chat still preloaded 4 files (~184 KB): an `em` in the shell gave italic to
  every used family, mono included. The build reads tags and classes, not the tree, so the
  inherited family of each `em` is not known; the base family is right for running text. Council
  (astra, fable, neon): base-family rule 3×0, cap of three 2×1 (astra: the face order does not
  measure visual importance, so the cap could drop a title face).
- **[firm]** (2026-11-23) Large lists render in slices, with no attribute (budget and scheduling
  revised below): once a full render has
  run 30 ms, an outer `<for>` (not inside another `<for>` item) creates at least one more item and
  leaves the rest for the next task (`setTimeout`), which creates only the missing items. In a
  reverse-column scroller (`css.md`, "Scroll that starts at the end") the last items come first.
  Items created by a later slice do not get `data-entering`. `mount` runs after each slice for
  the components that slice created. *Why:* v3chat `c/[id]` (40 markdown answers) built the whole
  conversation in one task (319, 160 and 80 ms tasks, TBT 350 ms against 60 ms in SvelteKit).
  Rejected: `<for chunk="20" from="end">` (one more choice for the agent to get wrong; the reverse
  column already says where the list starts), a count per slice (cost per item varies by 100×),
  slices for nested lists (a half-rendered answer).
- **[firm]** (2026-11-23, v3chat second round, council astra/fable/neon by majority) Slices take
  12 ms and `mount` calls share that budget (the rest run in order in the next task); later slices
  run at background priority (`scheduler.postTask`, else `setTimeout`) and continue from a cursor
  instead of rebuilding the list's keys and scopes. `__pd`/`__pl` come before the stylesheets (an
  inline script waits for the CSS above it: ~110 ms on mobile). Viewport prefetch waits for the
  first `/_data` and `/_layout`. The router scans added nodes for links once per idle period, not
  per mutation. *Why:* v3chat still had a 234 ms task (one item plus all its mounts) and the
  prefetch of `/`, its manifest and chunks competed with `/_data` at ~450 ms. Rejected by the
  council: one request for `/_data` + `/_layout` (ties the fast load to the slow one), splitting
  route CSS by first paint (cascade order, flash when a block opens), marking `column-reverse` at
  build (`@media`, dynamic classes; the runtime reads it once per list), `priority: 'high'` on the
  data fetches, building slices in a `DocumentFragment`, automatic `content-visibility`.
- **[firm]** (2026-11-23, bug in 0.2.7/0.2.8) A script creating its own state does not schedule a
  render: the top-level `let`s of a component, page or store, set while its factory runs, are read
  by the render that created it. *Why:* each new component's `let copied = false` scheduled a full render
  (nobody had read the key yet), the full render dropped the pending slices and started over, so
  every slice cost a whole page render: v3chat TBT 350 → 830 ms with list slicing on.
- **[firm]** (2026-11-23, council 3×0, form by 2×1) `[data-page]` gets `view-transition-name:
  page` only while a transition runs (`html[data-view-transition]`, set before
  `startViewTransition` and removed when the newest transition finishes). *Why:* a named element is
  always a stacking context and a backdrop root; v3chat's frosted topbar and composer stopped
  blurring the scroller behind them. Rejected: `:active-view-transition` (astra; newer than the
  browsers pragma targets).
- **[firm]** (2026-11-23) A closed `<dialog>` or `[popover]` binds only its own attributes; its
  content renders on the first open (the overlay manager renders the page right before
  `showModal`/`showPopover`) and stays rendered after that. Components inside are created, and
  their `mount` runs, on that first open. *Why:* the app cannot keep a closed overlay out of the
  DOM (`overlays.md` forbids open state in a `let`), so v3chat carried a session dialog with
  per-turn tables and a second copy of the history list in a drawer (2866 elements). Rejected: a
  `lazy` opt-in (closed by default is the only useful choice).

- **[firm]** (2026-11-23) `pragma build` prints the CSS gzip size of each page (shared + route,
  the number that decides inline at <= 14 kB) and marks a page over 14 kB as a render-blocking
  stylesheet. Information only, like JS (no budget). *Why:* v3chat `c/[id]` shipped 17 kB gzip of
  external CSS and nobody saw it: the table showed only JS and the largest CSS. Council vote 3×0.
- **[firm]** (2026-11-23) `html/meta-description` warning: a page (not `_404.html`) with no
  `<meta name="description">` in its merged `<head>` (page or layout). The starter layout has one.
  *Why:* agents forget it (v3chat SEO 91); a layout meta covers private apps with one line.
  Council vote 2×1 (against: a description is editorial).

## npm dependencies

- **[firm]** npm packages are allowed. *Why:* it's a web dev project; banning the ecosystem
  drives away people and agents. Replaces "no npm packages".
- **[firm]** Install with any manager (npm, bun, pnpm); pragma has no manager of its own, it
  only reads `package.json` (`dependencies`) and `node_modules`.
- **[firm]** Client: packages prebundled with `rolldown` (Rust crate, as a library, 1.3.x; same
  oxc as pragma). The build collects the names the app imports from each package (named,
  default, `* as`, side-effect; `import()` counts as namespace), creates one virtual entry per
  package re-exporting only those, and runs a single build (ESM, browser, tree-shaking,
  minified in prod, `process.env.NODE_ENV` replaced, shared chunks). Output: one
  `/assets/npm/<package>-<hash>.js` per package (`@date-fns/tz` → `date-fns__tz`), chunks in
  `/assets/npm/_*`; the app import is rewritten to that URL and counts toward `modulepreload`
  and page size. Subpaths of a package share its entry; a name repeated across subpaths (or
  `* as`/`import()` of a subpath) is re-exported as `$N$name` and the app import rewritten to
  `{ $N$name as name }`. CommonJS works (rolldown converts it). TS, JSX, `.vue`, imported
  CSS/assets and Node modules remain `app/package` errors; a name the package doesn't export
  is `js/import`; the error points at the app import. Cache in `<out>/.cache/npm` (key:
  entries, mode, `package.json`/lockfiles and each package's `package.json`), so `pragma dev`
  doesn't rebundle. *Why:* without a bundler, `date-fns/format` was 37 files and the `date-fns`
  index 304 (84 kB gzip); importing the index now costs only what's used. Replaces "no bundler,
  each package module is an asset". App modules stay unbundled.
- **[firm]** Guardrails: importing a package outside `dependencies` is a `js/import` error; UI
  frameworks (React, Vue, Svelte, lit, jQuery…) are rejected on the client; the docs
  (`docs/packages.md`) list recommended and "don't use" packages.
- **[firm]** Server: `package.json` packages imported normally (CJS included), not resolved at
  build; Node finds `node_modules` walking up from `dist/server`; the build writes
  `server/package.json` with `"type": "module"`. Workers: `wrangler` bundles.
- **[firm]** Server WebAssembly as a compiled module, like adapter-cloudflare + wrangler: the
  `.wasm` is copied next to the server code with a `CompiledWasm` rule; Node gets the same
  `WebAssembly.Module` through a load hook. *Why:* Workers cannot compile wasm from bytes (e.g.
  `highs`), and one import must work on Workers and in `pragma dev`.
- **[superseded]** No bundler for packages (each `node_modules` module as an asset, CJS
  forbidden): `rolldown` now prebundles. Import maps remain discarded (the import URL is
  rewritten).
- **[discarded]** Production CDN (esm.sh, jsdelivr) and manual vendoring as default.

## Observability (OTel)

- **[firm]** OpenTelemetry logs by design, client and server, zero config (SPEC §12): OTel Logs
  Data Model, semantic conventions, `traceparent` on every runtime fetch, span per request,
  unhandled error becomes `ERROR` + 500.
- **[firm]** No personal data by default (no body, cookies or auth headers).
- **[firm]** Client uses its own logger, not the official SDK (too heavy); batches to same-origin
  `/_otel`.
- **[firm]** OTLP/HTTP (JSON) export if `OTEL_EXPORTER_OTLP_ENDPOINT` is set, else OTel JSON
  on stdout. No gRPC.
- **[firm]** Configurable sampling and minimum level; `DEBUG` off in production, on in
  `pragma dev` (`PRAGMA_LOG_LEVEL=debug` unless set). `pragma dev` shows readable logs grouped
  by trace. The client stops posting after a 404/405 from `/_otel` (static hosting).
- **[firm]** The app uses `log.*`, not `console.*`; `pragma check` enforces it.
- **[superseded]** `/api/otel` endpoint: now `/_otel`.

## Server and deploy

- **[likely]** Server in JS, `fetch(request, env, ctx)` form. *Why:* Workers isn't Node (V8
  isolates; Rust only as Wasm, costing size, cold start and a string boundary). Rust stays in
  the CLI.
- **[likely]** `pragma dev` runs Node (≥ 22) for loaders and proxies `/_data`, `/_action`,
  `/_otel` and SSR pages (SPEC §11, §15). Replaces workerd in dev.
- **[firm v0]** Workers without `not_found_handling`: the handler (`run_worker_first`) serves
  assets, then resolves non-ssr param routes via `/_pragma/manifest.json`, then `_404.html` with
  404, like the Node adapter. Replaces the SPA fallback that sent every unknown URL to
  `index.html`.
- **[likely]** Cloudflare Workers is the preferred target (`dist/server/worker.js` +
  `wrangler.jsonc`); Node via `dist/server/node.mjs`; static-only hosting when there's no
  `*.server.js`.
- **[firm v0]** No `*.server.js` → no `server/` bundle and no Node child in `dev`/`serve`
  (BUG-S-1). `/_otel` alone does not justify a server: `pragma dev` prints client logs itself,
  `serve` answers 204 and static hosts get the 404 that stops the client.
- **[likely]** No host lock-in: small interfaces (`db`, `kv`, `files`) over `env`. v0 exposes
  `env` directly to `load`.
- **[likely]** `pragma dev --cf` uses wrangler `getPlatformProxy` in the node child (as
  adapter-cloudflare's dev), opt-in, wrangler from the app's `node_modules`. *Why:* real D1/R2/
  `send_email`/rate-limit bindings (including `remote: true`) in dev without workerd as the dev
  server; plain `pragma dev` stays dependency-free. `cloudflare:workers` imports stay Workers-only.
- **[superseded]** `/api/*` prefix and `pragma serve` forwarding to Node/Deno/Bun: now
  `/_data`, `/_action`, `/_otel`, and `serve` is the dev server without watch.
- **[firm v0]** `pragma dev`/`serve` also proxy endpoint paths (any method) and, when
  `routes/_middleware.server.js` exists, every path except `/assets/*` and `/_pragma/*`. The
  build writes `dist/server/routes.json` (`{ middleware, endpoints }`) for that decision; the
  client manifest keeps only pages.

## Tool (Rust CLI)

- **[firm]** Rust app to run locally, build and check: `pragma new`, `dev`, `build`, `check`,
  `serve`, `upgrade`, single binary. *Why:* best JS/CSS ecosystem (oxc, lightningcss), memory safety and
  single binary; Go and C++ discarded.
- **[firm]** `pragma check` validates everything: JS, HTML, directives and CSS (SPEC
  §10). Confirmed by the author (2026-11-18).
- **[firm]** `check` emits accessibility warnings (non-blocking; `--strict` makes them errors):
  img without alt, control without accessible name, input without label, dialog without name,
  `data-on-click` on a non-interactive element, skipped heading levels, more than one h1,
  `tabindex` > 0, duplicate id, iframe without title, `<a>` without href.
- **[firm]** `core/` is vendored in the app (copied by `pragma new`) and updated only by
  `pragma upgrade`, which makes it equal to the binary's embedded runtime, docs and `base.css`
  and touches nothing outside `core/`. `check` warns (`app/core-outdated`) when `core/`
  differs. *Why:* after a framework change, apps kept the old runtime and agents kept reading
  the old docs (site BUGS.md BUG-S-4). Byte comparison, no version file: nothing to keep in sync,
  and a hand edit in `core/` is also detected.
- **[firm v0]** `hyper` + `tokio`, no framework; `oxc`; `lightningcss`; `pragma-css`.
- **[firm v0]** Own strict HTML parser in the compiler: every non-void tag closes explicitly;
  errors with line and column. Replaces the open `lol_html` vs `html5ever`.
- **[firm]** The CLI stays in Rust; a Node rewrite was discarded. *Why:* single binary and
  instant startup (good for agents calling the CLI in loops), native parser (oxc), and Node
  adds no security: an npm package runs with the same permissions as a binary.
- **[firm]** Distribution only through GitHub Releases (`install.sh`, `pragma self-update`).
  No npm package, now or later (decided 2026-11-22; the npm wrapper packages were removed).
- **[firm]** On publishing, the project will be open source and auditable: binaries built only
  in CI from the public repo, GitHub attestations, signed
  checksums (sigstore), no `postinstall`, and a CLI with no network access and nothing outside
  the project. *Not done yet; only at publishing.*
- **[likely]** Release tool: `cargo-dist`. Goal: match `tests_1` esbuild (build about 0.33 s,
  dev about 0.4 s).
- **[proposal]** Optional WASM/WASI build (e.g. `wasmtime --dir=.`) as a real sandbox and for a
  browser playground.
- **[superseded]** "`pragma check` validates only JavaScript".

## Agent-first and WebMCP

- **[firm]** pragma is a framework for agents, both ways: agents build with it, and the apps
  they build are usable by agents. *Why:* people building with LLMs still reach for React or
  Angular out of habit and pay in performance. Others skip frameworks and write plain HTML and
  JS: the fastest result, but it does not scale; over time it eats context, turns into a
  tangle, and the agent loses track of it. Others pick SvelteKit, Astro or htmx, made for
  humans first. pragma keeps the speed of plain HTML and JS and adds only what stops the
  tangle: the minimum lint, components and reactivity on native browser features, and a Rust
  compiler, linter and validator. Never TypeScript. Agent-first means an agent given the
  landing URL understands everything it needs, and an app built with pragma works for agents
  too (WebMCP).
- **[firm v0]** WebMCP on by default (`"webmcp": true`; absent = true), with opt-out per form
  (`notool`) and per app (`"webmcp": false`, then any tool attribute is `webmcp/disabled`).
  *Why:* a default nobody has to remember is the only one agents apply every time; login and
  password forms are the exceptions, so they opt out.
- **[firm v0]** Declarative first: every `<form>` is a tool through the standard attributes
  (`toolname`, `tooldescription`, `toolparamdescription`, `toolautosubmit`), static only
  (SPEC §3.1). *Why:* the tool lifecycle is the DOM: the browser registers the form when it
  enters the document and removes it when it leaves, which matches the router with zero runtime
  code. The result goes back through `@submit` (`event.agentInvoked`, `respondWith`).
- **[firm v0]** The imperative API (`document.modelContext.registerTool`) is out of v0. *Why:*
  it needs runtime code and manual register/unregister on navigation; forms cover the actions
  apps expose.
- **[firm v0]** `check` severity (SPEC §10): rules for a broken tool (`webmcp/pair`, `name`,
  `duplicate`, `param-name`, `dynamic`, `notool`, `attribute`) are errors, because the browser
  registers nothing or the wrong thing; quality rules (`form-tool`, `name-style`,
  `description`, `param-description`, `autosubmit`) are warnings, errors with `--strict`.
  `toolautosubmit` on destructive or payment names is a warning: never act without the user
  for delete, pay or send.
- **[firm v0]** Markdown for fetch agents (SPEC §11): a request whose `Accept` prefers
  `text/markdown` gets `public/<path>.md` when it exists, with `Vary: Accept`; same in `dev`,
  `serve`, Workers and Node. *Why:* agents that fetch without a browser read markdown at a
  fraction of the tokens of HTML; a static file keeps it zero-cost and explicit.

## Documentation

- **[firm]** Simple, lean, LLM-oriented docs: `core/docs/`, copied to `<app>/core/docs/`, plus a
  short `AGENTS.md` in the app. Imperative rules and one canonical example per topic.
- **[firm]** Automated tests don't run on every change; agents run them only when they
  recommend it and the author accepts (`TESTING.md`).

---

## Open

- [ ] Better test app (replaces the removed `meu-app`): a todo app exercising every concept
      (routes and params, components and slots, state and stores, loader and SSR, server
      actions, overlays, npm packages, a11y).
- [ ] `db`, `kv` and `files` interfaces over `env`.
- [ ] Publishing: release pipeline (cargo-dist), per-platform npm packages,
      provenance/attestations and signed checksums (see "Tool (Rust CLI)").
- [x] Per-route CSS and inline critical CSS (2026-11-21).
- [ ] Brotli or gzip; precompress at build.
- [ ] 103 Early Hints on Cloudflare.
- [ ] Measure LCP, INP and navigation on simulated slow 4G, with the `tests_1` bench, against
      SvelteKit with prerender or SSR.
- [ ] Out of v0 (SPEC §13): SSG with data, nested layouts, `[...rest]`, Speculation Rules, i18n.

- **[firm]** `js/html-in-js` string checks apply only to client code. `.server.js` strings may hold markup that never reaches the page DOM (e-mail bodies, LLM prompt tags). DOM APIs (`innerHTML`, `createElement`) stay errors everywhere.
- **[firm]** `waitUntil` flushes the request logs again when its promise settles, so logs written after the response (background agent turns, e-mails) are not lost.
