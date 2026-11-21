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
- **[firm]** (DX review, 2026-11-19) Component and page CSS is scoped by the build: each selector
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
- **[firm]** (2026-11-21) Styled select is `c-select` (starter): `c-dropdown` plus a hidden native
  `<select name>`, so `FormData` and WebMCP get the value. A component may pass a tool attribute on
  with `:toolparamdescription="$prop"`; the caller's fixed text is written at build time, a bound
  value is `webmcp/dynamic`. AGENTS.md: rules do not change the request, target is current Chrome,
  Edge, Firefox and Safari, and limits are told before coding. *Why:* an agent asked for "a dropdown
  with HTML" shipped `appearance: base-select` (Chrome/Edge only) because it was "the simplest code
  that passes `pragma check`" and `c-dropdown` sends no form value.
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

## Data and SSR

- **[firm]** Direction: pages open fast and fetch what they need via async load and loaders,
  showing a skeleton (SPEC §5).
- **[firm]** No SSR by default. Data in `routes/<route>.server.js` with `load(...)`; the client
  fetches `/_data/<path>`, started by an inline `<head>` script before modules.
- **[firm]** SSR is per-page opt-in with `export const ssr = true`; the client binds directives
  without rebuilding the DOM.
- **[firm]** Never HTML in JS, not even in SSR: the server renders from the same `.html`,
  compiled to ops (`templates/<route>.json`).
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

## CSS

- **[firm]** Reuse CSS as much as possible.
- **[firm]** Tailwind v4 subset reimplemented in Rust (`pragma-css` crate), generating only used
  classes, no arbitrary values. Goal: fewer classes; custom classes allowed for refined cases
  (SPEC §8). *Why:* CSS stops growing, agents already know the names, no naming decisions.
- **[firm v0]** `base.css` (preflight + layers) and `lib/theme.css` (tokens and custom
  classes); dark theme via `[data-theme="dark"]` and `dark:`; one global hashed CSS.
- **[superseded]** "No Tailwind, preflight only" and "Tailwind v4 vs own CSS".
- **[superseded]** Per-route CSS: v0 emits one global CSS (per-route CSS is out of v0).

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
  `page`, `log`, `call`, `navigate`, `open`, `close`, `confirm`. They are live (`data` is the
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
- **[firm v0]** No bundler: each module minified (oxc) and hashed; `immutable` assets;
  uncached HTML.
- **[firm v0]** Budgets in `pragma.json` (`initialJsKb`, `routeJsKb`, `cssKb`), checked by
  build and check. Reference ceiling: 7.2 to 8.2 kB gzip total JS in the `tests_1` natives.
- **[superseded]** Per-route chunks with a bundler and chunk manifest: v0 uses loose modules
  and the `/_pragma/manifest.json` manifest.

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
  and budget. Subpaths of a package share its entry; a name repeated across subpaths (or
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
- **[superseded]** `/api/*` prefix and `pragma serve` forwarding to Node/Deno/Bun: now
  `/_data`, `/_action`, `/_otel`, and `serve` is the dev server without watch.

## Tool (Rust CLI)

- **[firm]** Rust app to run locally, build and check: `pragma new`, `dev`, `build`, `check`,
  `serve`, `upgrade`, single binary. *Why:* best JS/CSS ecosystem (oxc, lightningcss), memory safety and
  single binary; Go and C++ discarded.
- **[firm]** `pragma check` validates everything: JS, HTML, directives, CSS and budgets (SPEC
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
- **[firm]** Distribution via npm with prebuilt per-platform binaries (`optionalDependencies`,
  esbuild/biome model) + minimal JS shim; `npx pragma` works without knowing it's Rust. Also
  GitHub releases for non-Node users.
- **[firm]** On publishing, the project will be open source and auditable: binaries built only
  in CI from the public repo, `npm publish --provenance` + GitHub attestations, signed
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
      actions, overlays, npm packages, a11y, budgets).
- [ ] `db`, `kv` and `files` interfaces over `env`.
- [ ] Default budget values in `pragma.json`.
- [ ] Publishing: release pipeline (cargo-dist), per-platform npm packages,
      provenance/attestations and signed checksums (see "Tool (Rust CLI)").
- [ ] Bundler for app modules after v0 (`rolldown` already bundles npm packages), and whether
      it beats one file per module with `modulepreload`.
- [ ] Per-route CSS and inline critical CSS.
- [ ] Brotli or gzip; precompress at build.
- [ ] 103 Early Hints on Cloudflare.
- [ ] Measure LCP, INP and navigation on simulated slow 4G, with the `tests_1` bench, against
      SvelteKit with prerender or SSR.
- [ ] Out of v0 (SPEC §13): SSG with data, nested layouts, `[...rest]`, Speculation Rules, i18n.
