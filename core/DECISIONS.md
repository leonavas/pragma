# pragma decisions

What was decided and why. The **how** is in `SPEC.md`, the v0 contract, which overrides this
file. Changing a **[firm]** decision requires an explicit author decision.

Status: **[firm]** · **[firm v0]** v0 contract, revisable later · **[likely]** ·
**[proposal]** · **[open]** · **[superseded]** · **[discarded]**. All items dated 2026-11-18.

## Summary: the pragma contract

1. Plain JavaScript (ES modules). Never TypeScript or JSDoc types.
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
- **[firm]** Overlays (modal, alert, popup, dropdown) are always HTML + JS. Back (browser or
  Android) closes the overlay instead of leaving the page. Modal and alert lock scroll (SPEC §6).
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
- **[firm v0]** Server writes via `.server.js` `actions` and `ctx.call` →
  `POST /_action/<path>/<name>`.
- **[superseded]** SSR via `HTMLRewriter` on Workers and `lol_html` in `pragma serve`: replaced
  by compiled ops rendered by the JS handler.
- **[superseded]** "No SSR (SPA + API)".

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

- **[likely]** v0: actions mutate state and the runtime reapplies directives. No exposed
  signals (SPEC §4). Global state in stores (`lib/state/`), optional `persist`. Derived values
  are getters in `state(ctx)` or the store, recomputed each render (not copied by actions or
  persisted).
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
- **[firm]** Configurable sampling and minimum level; `DEBUG` off in production. `pragma dev`
  shows readable logs grouped by trace.
- **[firm]** The app uses `log.*`, not `console.*`; `pragma check` enforces it.
- **[superseded]** `/api/otel` endpoint: now `/_otel`.

## Server and deploy

- **[likely]** Server in JS, `fetch(request, env, ctx)` form. *Why:* Workers isn't Node (V8
  isolates; Rust only as Wasm, costing size, cold start and a string boundary). Rust stays in
  the CLI.
- **[likely]** `pragma dev` runs Node (≥ 22) for loaders and proxies `/_data`, `/_action`,
  `/_otel` and SSR pages (SPEC §11, §15). Replaces workerd in dev.
- **[likely]** Cloudflare Workers is the preferred target (`dist/server/worker.js` +
  `wrangler.jsonc`); Node via `dist/server/node.mjs`; static-only hosting when there's no
  `*.server.js`.
- **[likely]** No host lock-in: small interfaces (`db`, `kv`, `files`) over `env`. v0 exposes
  `env` directly to `load`.
- **[superseded]** `/api/*` prefix and `pragma serve` forwarding to Node/Deno/Bun: now
  `/_data`, `/_action`, `/_otel`, and `serve` is the dev server without watch.

## Tool (Rust CLI)

- **[firm]** Rust app to run locally, build and check: `pragma new`, `dev`, `build`, `check`,
  `serve`, single binary. *Why:* best JS/CSS ecosystem (oxc, lightningcss), memory safety and
  single binary; Go and C++ discarded.
- **[firm]** `pragma check` validates everything: JS, HTML, directives, CSS and budgets (SPEC
  §10). Confirmed by the author (2026-11-18).
- **[firm]** `check` emits accessibility warnings (non-blocking; `--strict` makes them errors):
  img without alt, control without accessible name, input without label, dialog without name,
  `data-on-click` on a non-interactive element, skipped heading levels, more than one h1,
  `tabindex` > 0, duplicate id, iframe without title, `<a>` without href.
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
- [ ] Workers and param routes: with SPA `not_found_handling`, `/shop/123` gets `index.html`;
      needs resolving to `shop/[id].html` (on Node, the manifest does it).
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
