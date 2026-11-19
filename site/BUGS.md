# pragma bugs found while building the inventory

Repro app: `pragma new /tmp/pinv && cd /tmp/pinv`, then add the files listed. Binary: `~/.cargo/bin/pragma` (2026-11-18 22:38).

## B1 — Workers: non-SSR param routes and unknown URLs get `index.html`
- Repro: app with `routes/blog/[slug].html` (no `ssr`) and any `*.server.js`; `pragma build`; `npx wrangler dev --config dist/server/wrangler.jsonc`; open `/blog/hello` and `/nope`.
- Expected: `/blog/hello` → `blog/[slug].html`; `/nope` → `_404.html` with 404.
- Actual (by code; already an open item in `core/DECISIONS.md`): `wrangler.jsonc` uses `not_found_handling: "single-page-application"` and the handler sends every non-SSR page to `env.ASSETS.fetch(request)`, so both get `/index.html` (200), and boot mounts the home route.
- Suspected: `core/cli/crates/pragma-compiler/src/lib.rs:644-650` (wrangler config), `core/runtime/server/handler.js:79` (no manifest lookup for non-SSR pages, unlike `server/node.mjs:83`).
- **Status:** FIXED — `core/runtime/server/handler.js` (on an ASSETS 404: manifest lookup for non-ssr param routes, then `_404.html` with 404), `core/cli/crates/pragma-compiler/src/lib.rs` (wrangler.jsonc without `not_found_handling`). Verified with a Workers-like ASSETS mock: `/doc/abc` → `doc/[id].html` 200, `/nope` → `_404.html` 404.

## B2 — `<meta>`/`<link>` in a page fragment silently ends up inside `<main>`
- Repro: `routes/about.html` = `<title>About</title>\n<meta name="description" content="About page">\n<h1>About</h1>`; `pragma check --strict` → no problems; `pragma build`.
- Expected: placed in `<head>`, or a check error (pages cannot have head tags).
- Actual: `<main data-page>\n<meta name="description" ...>\n<h1>About</h1>` in `<body>`.
- Suspected: `core/cli/crates/pragma-compiler/src/expand.rs:141-150` (only `<title>` is lifted), `validate.rs:206-213` (no rule for `meta`/`link`/`base` in pages).
- **Status:** FIXED — top-level `<meta>`/`<link>` in a page now go to `<head>` and replace the layout tag with the same key; anywhere else (or in components) → `html/head`. `core/cli/crates/pragma-compiler/src/expand.rs`, `core/cli/crates/pragma-compiler/src/validate.rs`, `core/cli/crates/pragma-compiler/src/lib.rs`.

## B3 — Directives on `<title>` are silently dropped
- Repro: `routes/blog/[slug].html` starts with `<title :text="data.post.title">Post</title>` (with a `load`). `pragma check --strict` passes.
- Expected: dynamic title, or an error saying the title is static.
- Actual: `<title>Post</title>`; attributes discarded; nothing on server (SSR) or client.
- Suspected: `expand.rs:143-147`, `lib.rs:534-546`.
- **Status:** FIXED — `<title :text="data.x">` is kept in the shell and rendered by the server (requires `ssr = true`, else `html/head`). `core/cli/crates/pragma-compiler/src/expand.rs`, `core/cli/crates/pragma-compiler/src/lib.rs`, `core/runtime/server/render.js`, `core/runtime/server/handler.js`.

## B4 — Directives in the layout `<head>` pass check but are never evaluated
- Repro: in `routes/_layout.html` `<head>` add `<meta property="og:url" :content="params.slug">`; `pragma check --strict` passes; `pragma build`.
- Expected: evaluated (at least on SSR) or rejected.
- Actual: emitted literally `<meta property="og:url" data-attr-content="params.slug">` on every page. Runtime renders only `document.body` (`core/runtime/page.js:27`), SSR only `<main>` (`handler.js shellWith`), head extras are written raw (`lib.rs:547-549`) and never scope-checked (`lib.rs:406`).
- Impact: no way to get per-page/per-entity OG tags; the check gives false confidence.
- **Status:** FIXED (rejected) — any directive in the layout `<head>` → `html/head`; per-page bound tags go at the top of the page (see SEO in `core/docs/routes.md`). `core/cli/crates/pragma-compiler/src/validate.rs`, `core/cli/crates/pragma-compiler/src/expand.rs` (`Scoped::head`).

## B5 — `:class` bypasses the class check and CSS generation
- Repro: `routes/cls.html` = `<title>Cls</title>\n<h1 :class="$tone">Cls</h1>`, `routes/cls.js` = `export const state = () => ({ tone: 'text-warning-700' })`; check passes; `pragma build`; grep the CSS.
- Expected: `:class` rejected (like `:style`) pointing to `.class` toggles, or classes validated.
- Actual: no `.text-warning-700` rule in `dist/client/assets/app-*.css`; at runtime the attribute also overwrites static classes and `.x` toggles.
- Suspected: `validate.rs:338-343` (only `on*`/`style` blocked for `Dir::Attr`).
- **Status:** FIXED — `:class` → `html/class` error pointing to `.name="path"`. `core/cli/crates/pragma-compiler/src/validate.rs`.

## B6 — `{{ }}` / `${ }` are not rejected though the docs say they are forbidden
- Repro: page with `<p>{{ data.post.title }}</p>`; `pragma check --strict` → no problems.
- Expected (per `core/docs/syntax.md` "Forbidden: … `{{ }}`, `${ }`"): an error.
- Actual: renders the literal text `{{ data.post.title }}`.
- Suspected: no check exists in `validate.rs`; docs/code mismatch.
- **Status:** FIXED — `{{ }}`/`${ }` in text or attribute values → `html/interpolation` (allowed as text inside `<code>`/`<pre>`). `core/cli/crates/pragma-compiler/src/validate.rs`.

## B7 — Bare boolean prop is truthy in HTML but `''` (falsy) in component JS
- Repro: `<c-toggle label="Dark" locked></c-toggle>` with a `toggle.js`; built root has `data-locked=""`.
- Expected: same truthiness in `$locked` (HTML) and `ctx.$.locked` (JS).
- Actual: HTML `<if $locked>` is true (`expand.rs:41-47 prop_truthy`), JS `ctx.$.locked === ''`. Likewise `locked="false"` is false in HTML, `'false'` (truthy) in JS.
- Suspected: `expand.rs:483-490` (prop → `data-*` value), `core/runtime/page.js:46`.
- **Status:** FIXED — bare prop emitted as `data-x="true"`; `ctx.$` maps `"true"`→`true`, `"false"`→`false`; HTML treats `""` as falsy too. `core/cli/crates/pragma-compiler/src/expand.rs`, `core/runtime/page.js`, `core/docs/components.md`.

## B8 — Store used only from JS is not loaded; check passes, runtime TypeError
- Repro: `lib/state/prefs.js` (store with getter `greeting`), `routes/st.html` = `<title>St</title>\n<h1 :text="$greeting">x</h1>`, `routes/st.js` = `export const state = ctx => ({ get greeting() { return ctx.shared.prefs.greeting } })`. `pragma check --strict` passes; build; open `/st`.
- Expected: store available as `ctx.shared.prefs` (docs: "read as `shared.tasks` in HTML and `ctx.shared.tasks` in JS").
- Actual: `#pragma-route` has `"stores":{}`; console `Uncaught TypeError: Cannot read properties of undefined (reading 'greeting')` (verified headless). Works only by accident after visiting a page that loaded the store.
- Suspected: `lib.rs:428` and `lib.rs:525` (stores only from HTML `shared.x` refs or static imports).
- **Status:** FIXED — stores referenced as `shared.<name>` in the page/component modules are added to `#pragma-route.stores` and preloaded. `core/cli/crates/pragma-compiler/src/lib.rs` (`shared_refs`). Verified headless: `/st` renders "Hi Ana".

## B9 — "Prebuilt HTML arrives with skeleton" is not what is emitted
- Repro: any page with `<if page.loading><p class="skeleton h-8"></p></if>`; look at `dist/client/<page>.html`.
- Expected (SPEC §5, data.md): skeleton visible in the first HTML.
- Actual: only `<template data-if="page.loading">…</template>` (inert); the skeleton appears after boot and modules load. Doc imprecision; matters for LCP/no-JS.
- Suspected: `compile.rs write_element` (templates never instantiated at build) vs SPEC wording.
- **Status:** FIXED (docs) — SPEC §5 and `core/docs/data.md` now say the skeleton appears when `core/boot.js` runs (inert `<template>` in raw HTML); use `ssr` for no-JS content. Instantiating the loading branch at build would contradict "blocks are templates" and double the DOM swap; not done.

## B10 — Server `log.debug` never shows in `pragma dev`
- Repro: `log.debug('catalog loaded')` in a `load` (as in `core/example/routes/shop/index.server.js`); run `pragma dev`, request the page.
- Expected (logs.md: "debug: off in production"): visible in dev.
- Actual: dropped; `minSeverity` defaults to `info` unless `PRAGMA_LOG_LEVEL=debug` is set, and `pragma dev` does not set it.
- Suspected: `core/runtime/server/log.js:38-40`, `core/cli/crates/pragma-cli/src/node.rs:45`.
- **Status:** FIXED — `pragma dev` starts node with `PRAGMA_LOG_LEVEL=debug` unless already set. `core/cli/crates/pragma-cli/src/node.rs`, `dev.rs`, `lib.rs`.

## B11 — Client logs POST `/_otel` on static deploys / `pragma serve` without a server
- Repro: app with no `*.server.js`; `pragma build && pragma serve`; navigate between pages.
- Expected: no failing requests when there is no log endpoint.
- Actual (by code): each navigation logs `info` → batch `POST /_otel` → 405 from `pragma serve` (`server.rs:164-176` only handles `/_otel` in dev) / 404-405 on static hosts; repeated failed requests in the network panel.
- Suspected: `core/runtime/boot.js:10` (always `initClientLog`), `core/runtime/log.js:49-51`.
- **Status:** FIXED — `pragma serve` without a backend answers `POST /_otel` 204 (`core/cli/crates/pragma-cli/src/server.rs`); the client stops posting after a 404/405 (`core/runtime/log.js`).

## BUG-S-1 — Node server is built and started even with no `*.server.js`
- Repro: `pragma new /tmp/pnode && cd /tmp/pnode && pragma dev` (starter has no `.server.js`).
- Expected (SPEC §9 "server/worker.js … (if any *.server.js)", §11 "If any *.server.js exists, starts node"): no server bundle, no Node child.
- Actual: prints `node running on port …`; `.pragma/dev/server/{worker.js,node.mjs}` emitted with `routes: []`. Same for `pragma build` (`dist/server/`). Possibly intentional for `/_otel`; then SPEC/cli.md are wrong.
- Suspected: `core/cli/crates/pragma-compiler/src/lib.rs:619` (gated on `core/server/handler.js` existing, not on `server_routes`).
- **Status:** FIXED — gated on `server_routes` (SPEC §9/§11 and DECISIONS "static-only hosting when there's no `*.server.js`" were right; `/_otel` alone does not need Node: `pragma dev` prints client logs itself, `serve` answers 204). No `*.server.js` → no `server/` output, build report without "with server", no Node child. `core/cli/crates/pragma-compiler/src/lib.rs`; test `real_starter_is_clean` asserts no server bundle; decision recorded in `core/DECISIONS.md` (Server and deploy). Verified: fresh starter `pragma build` → `dist/client` only; `pragma dev` → no "node running", `POST /_otel` 204 and printed.

## BUG-S-2 — No way to emit a per-page canonical, description or og:url
- Context: building the site's SEO head (`site/routes/_layout.html`).
- Expected: per-page `<link rel="canonical">`, `<meta name="description">` and `og:title/og:url`, the minimum for a docs site.
- Actual: only `<title>` is per page; layout `<head>` is static for every page (B4: directives there are not evaluated; B2: head tags in a page land in `<main>`). A static canonical would point every page at one URL, so the site omits canonical entirely.
- Suggestion: lift `<meta>`/`<link rel="canonical">` at the top of a page fragment into `<head>` like `<title>`.
- Suspected: `core/cli/crates/pragma-compiler/src/expand.rs:141-150`, `lib.rs:534-550`.
- **Status:** FIXED with B2/B3/B4 — page-level `<title>`/`<meta>`/`<link>` go to `<head>` and override the layout; bound values need `ssr = true`. See SEO in `core/docs/routes.md`.

## BUG-S-3 — Foreign framework attributes (`v-if`, `x-show`) pass check
- Repro: `routes/index.html` = `<title>Home</title>\n<ul>\n  <li v-if="done">Task</li>\n</ul>`; `pragma check --strict` → no problems (only `onclick` gets flagged when present).
- Expected (SPEC §0 "anything not listed here is an error", §10 "unknown or legacy syntax"): an `html/unknown-directive` error suggesting `<if done>`; agents trained on Vue/Alpine write these often.
- Actual: emitted as a plain attribute; nothing happens at runtime.
- Suspected: `core/cli/crates/pragma-compiler/src/validate.rs` (legacy list only covers `data-*`).
- **Status:** FIXED — Vue (`v-*`), Alpine (`x-data`, `x-show`, `x-on:…`…), Svelte (`on:`, `bind:`, `class:`…), Angular (`*ngIf`, `[x]`, `(x)`, `[(ngModel)]`) and React (`className`, `htmlFor`) attributes → `html/unknown-directive` naming the pragma form, e.g. "`v-if` is Vue syntax and does nothing in pragma; wrap the element in `<if path>…</if>`". SVG namespaced attributes (`xlink:href`, `xml:lang`) still pass. `core/cli/crates/pragma-compiler/src/validate.rs` (`foreign_directive`, unit test `foreign_directives`), test `foreign_framework_attributes` in `tests/build.rs`; docs `core/docs/cli.md`, `core/docs/syntax.md`.

## B12 — Loader logs a false "loader failed" when leaving before data arrives (Firefox)
- **Status:** owned by @claude-105 (`core/runtime/loader.js`).

## B13 — Inline `self.__pd=fetch(...)` has no catch: unhandled rejection on early unload (Firefox)
- **Status:** owned by @claude-105 (`PD` const in `core/cli/crates/pragma-compiler/src/lib.rs`).

## BUG-S-4 — No way to update an app's vendored `core/` after a framework change
- **Repro:** `pragma new site`, then change `core/runtime/page.js` (or `core/docs/*`) and reinstall the CLI. `site/core/` stays stale; `pragma dev/build` keep using the old runtime, and the old docs keep being given to agents. Example: the fix for B7 (`prop()` conversion in `page.js`) did not reach `site/` until the files were copied by hand.
- **Expected:** a `pragma upgrade` command, or `check` warning when `core/` differs from the CLI's embedded runtime/docs version.
- **Status:** FIXED — new `pragma upgrade` makes `core/` equal to the CLI's embedded runtime, docs and `base.css` (writes changed/missing files, removes extra ones, prints each path, touches nothing outside `core/`); `pragma check` warns `app/core-outdated` when `core/` differs. `core/cli/crates/pragma-cli/src/upgrade.rs`, `core/cli/crates/pragma-cli/src/new.rs`, `core/cli/crates/pragma-cli/src/lib.rs`, `core/cli/crates/pragma-cli/tests/upgrade.rs`; docs in `core/docs/cli.md`, `core/SPEC.md` §10/§11, `core/DECISIONS.md`, `core/starter/AGENTS.md`. Verified in `site/`: `check` warned (`core/docs/cli.md`), `pragma upgrade` updated only that file, `check` → no problems.

## BUG-S-5 — Unknown npm export error leaks rolldown internals
- Repro: app with `date-fns` in `dependencies`; `routes/err.js` = `import { formatt } from 'date-fns'`; `pragma check`.
- Expected: `js/import` "`formatt` is not exported by date-fns" (like the other `js/import` messages).
- Actual: `[js/import] package: "formatt" is not exported by "node_modules/date-fns/index.js", imported by "../../pragma-check-1346273-1795053972461316908/.cache/npm/entries/date-fns.js".` (stray `package:` prefix, temp build dir and the synthetic entry file).
- Suspected: rolldown error passthrough in `core/cli/crates/pragma-compiler/src/bundle.rs` (message not rewritten).
- **Status:** FIXED — `MISSING_EXPORT` is rewritten: from app code → `js/import` "`formatt` is not exported by date-fns"; between packages → `app/package` "package \"a\" imports `x` from \"b\", which does not export it". Other rolldown messages drop the temp entry path and keep only `node_modules/…`. `core/cli/crates/pragma-compiler/src/bundle.rs` (`describe`, `tidy`), test in `core/cli/crates/pragma-compiler/tests/build.rs` (`npm_package_missing_export`).

## BUG-S-6 — `import()` with a computed specifier passes check, then fails in the browser
- Repro: `routes/err.js` = `export async function mount() { const pkg = 'date-fns'; await import(pkg) }` (or ``import(`${name}`)``); `pragma check` → `no problems found`; build has no `assets/npm/` entry.
- Expected: `js/import` error, as for ``import(`date-fns`)`` ("must use quotes, not a template literal").
- Actual: passes; at runtime the browser cannot resolve a bare specifier.
- Suspected: `core/cli/crates/pragma-jslint/src/lib.rs` (only literal template specifiers are checked).
- **Status:** FIXED — in app code (client and `.server.js`), `import()` with a variable or a `${}` template → `js/import` "`import()` needs a quoted path or package name…" (core runtime keeps its computed imports). `core/cli/crates/pragma-jslint/src/lib.rs`, test `computed_dynamic_import` in `core/cli/crates/pragma-jslint/tests/jslint.rs`; docs `core/docs/cli.md`, `core/SPEC.md` §10.

## BUG-S-7 — Node startup log uses `service.name: "pragma-app"`, not the app name
- Repro: `pragma new my-app`, add any `*.server.js`, `pragma dev` (or `pragma serve`, or `node dist/server/node.mjs`).
- Expected: `"resource":{"service.name":"my-app"}` like the per-request records.
- Actual: the `pragma serving at http://localhost:…` record has `"service.name":"pragma-app"`; request records say `my-app`.
- Suspected: `core/runtime/server/node.mjs:8,165` uses the module-level `log` from `server/log.js:72`, built with `resourceFrom(env)` and no service name.
- **Status:** FIXED — the generated `dist/server/node.mjs` passes `service: "<pragma.json name>"` to `serve()`, which builds its logger with `resourceFrom(process.env, service)` (`OTEL_SERVICE_NAME` still wins). `core/runtime/server/node.mjs`, `core/cli/crates/pragma-compiler/src/lib.rs`, snapshot `tests/snapshots/server__node.mjs`, test in `core/runtime/test/server/node.test.js`; `core/SPEC.md` §15. `site/core` synced with `pragma upgrade`.

## BUG-S-8 — `pragma dev` with a check error anywhere: new pages are served but their `load` 404s
- Repro: `pragma dev` on an app; add a check error in any unrelated file (e.g. `class="nope"` in another page); then add `routes/x.html` (reads `data.items`) and `routes/x.server.js` with a `load`. Open `/x`.
- Expected: either the new page is not served until the build is clean (overlay only), or it is served together with its loader.
- Actual: the build writes the new client files to `.pragma/dev/client/` and `server/worker.js`, so `/x` is served with `"loader":true`, but `rebuild` returns false and `node.sync` is skipped: the old Node process answers `GET /_data/x` with 404 `{"error":{"message":"Route has no data"}}` and the page logs `loader failed`. Seen on this site while other sections had errors. Fixing the unrelated error makes it work.
- Suspected: `core/cli/crates/pragma-cli/src/dev.rs:50-54` (skips `node.sync`/`reload_manifest` on failure) while the compiler has already written outputs (`pragma-compiler/src/lib.rs`, outputs are not withheld on error).
- **Status:** FIXED — "served together with its loader": after every rebuild, also a failed one, `pragma dev` reloads the route manifest and runs `node.sync`, so Node always serves the same build as the client files (withholding outputs would freeze every page while any other section has an error). The overlay still shows the error. `core/cli/crates/pragma-cli/src/dev.rs`; `core/SPEC.md` §11, `core/docs/cli.md`. Verified with the repro: `class="nope"` in `routes/other.html`, then `routes/x.html` + `x.server.js` → Node restarted, `GET /_data/x` 200 with the items.

## BUG-S-9 — `pragma dev` live reload (EventSource) exhausts the browser's per-host connection limit
- **Repro:** `pragma dev`, open the app in ~6 tabs on the same host (e.g. 127.0.0.1:5181). Each tab holds an open `EventSource("/_pragma/reload")` (`core/cli/crates/pragma-cli/src/dev.js:1`). Under HTTP/1.1 Chromium allows 6 connections per host, so new navigations, `/_data` fetches and assets hang forever (tabs stuck in "loading"). Seen with several agent tabs in the Sheltie browser.
- **Expected:** many dev tabs work. Fix options: WebSocket for reload (not counted against the HTTP/1.1 per-host pool, what Vite does), or share one connection across tabs (BroadcastChannel + leader tab / SharedWorker).
- **Status:** FIXED — live reload is a WebSocket at `/_pragma/reload` (hand-written RFC 6455 server half in `server.rs`: handshake + text frames, JSON messages `hello`/`css`/`reload`/`error`; no new crates). A plain GET there gets 426. Same semantics: CSS hot swap, error overlay (also sent on connect after a failed build), reload, reconnect every 1 s and reload when the server restarted. `core/cli/crates/pragma-cli/src/server.rs`, `core/cli/crates/pragma-cli/src/dev.js`, tests `websocket_emits_reload_events` and `many_reload_sockets_leave_http_working` in `tests/server.rs`; `core/SPEC.md` §11, `core/docs/cli.md`. Verified with headless Chromium (Playwright), 10 tabs + 1 extra tab on one host: all load in < 70 ms, CSS swap without reload in all tabs, HTML edit reloads all, error overlay in all, fix recovers all. Control with the old CLI: tabs 7+ hang (timeout).

## BUG-S-11 — Same-named components in different folders share one CSS scope
- **Repro:** two route-local components with the same name, e.g. `routes/examples/syntax/_components/badge/` and `routes/examples/components/_components/badge/` (both exist on this site), or a `lib/components/x/` and a `routes/_components/x/`. Give their `.css` different values for the same selector (`[data-c="badge"] .badge-icon { width: 0.75rem }` vs `{ width: 0.5rem }`). `pragma check --strict` → no problems.
- **Expected:** `components.md` says `p-x` is private to its folder, so either its styles stay private too, or `check` reports the name clash.
- **Actual:** both get `data-c="badge"` and both CSS files go into the one global `app.css`, so the later file wins on both pages: on `/examples/components` the badge icon is 0.75rem and accent-colored (syntax's rule), not 0.5rem. Same for `<c-terminal>` vs the landing's old `<p-terminal>` (renamed to `p-cli-session` to avoid it).
- **Suspected:** `core/cli/crates/pragma-compiler/src/expand.rs:479` (`data-c` = bare name) with all component CSS concatenated into `app.css`. Fix options: scope `p-` components by folder (`data-c="examples/syntax/badge"` or a hash), or make a duplicate component name an `html/component` error.
- **Site workaround:** rename one of the two `badge` components (owners: syntax, components sections). Not done here.
- **Status:** FIXED — `p-` components get a per-folder id: `data-c="<name>-<6 hex of the folder path>"` (e.g. `badge-091823`); `c-` keep `data-c="<name>"`. The build rewrites `[data-c="<name>"]` (also `'<name>'` and unquoted) in the component's own `.css` to that id, so authors keep writing `[data-c="<name>"]` and the site needed no CSS edits. Route JSON `components` and the owner/action check use the same id; the unknown-action message names the tag (`<p-badge>`). `core/cli/crates/pragma-compiler/src/project.rs` (`Component::id`), `src/expand.rs`, `src/lib.rs` (`scope_component_css`); test `same_named_route_components_are_scoped_by_folder` in `tests/build.rs`; snapshot `client__shop__[id].html` (`price` → `price-cead5e`); docs `core/docs/components.md`, `core/docs/css.md`, `core/SPEC.md` §3/§8. Verified on a copy of this site: `/examples/syntax` badge = `badge-091823` (icon .75rem, accent), `/examples/components` badge = `badge-d4a5e7` (icon .5rem). Not for authors: selecting a `p-` component's `data-c` from outside its own `.css` (theme, tests) no longer matches; use a class, role or label.

## BUG-S-10 — Router prefetch evaluates the modules of every link in view
- **Found by:** council review (opus). On pages with many internal links (section nav, sidebars), the runtime prefetch imports and evaluates route modules for links merely visible in the viewport, not just fetches them.
- **Expected:** prefetch on intent (hover/focus/touchstart) or `<link rel="modulepreload">` / fetch-only for links in view; never evaluate a route module until navigation.
- **Suspected:** `core/runtime/router.js` (prefetch / IntersectionObserver path).
- **Note:** two bugs share the id BUG-S-10 (the one above is the CSS scope one); this is the router one.
- **Status:** FIXED — prefetch (link in view, hover, focus) adds `<link rel="modulepreload">` for the route modules (download + compile, no run; skips modules already preloaded) and still fetches the prebuilt HTML; `import()` runs only in `navigate`. Now matches SPEC §7. `core/runtime/router.js` (`preload`), client test `router: prefetch on hover and by viewport` (asserts the modulepreload link, the module not run before navigation, and run after) with a marker in `core/runtime/test/client/fixtures/assets/ssr.js`. `cargo xtask test client` 23/23. `site/core` synced with `pragma upgrade`.

## BUG-S-12 — `:style` is reported under the rule `html/on-inline`
- **Repro:** `pragma new /tmp/app`; `routes/x.html` = `<title>X</title>\n<p :style="$css">x</p>` (plus `routes/x.js` with `export const state = () => ({ css: '' })`); `pragma check`.
- **Expected:** `[html/style-inline]` (the rule `style="…"` already uses; cli.md lists `html/style-inline` for style), with a message that points to classes / `.name="path"`.
- **Actual:** `error [html/on-inline] :style is not allowed` — wrong rule id for a filter or an agent that keys on rule names, and no fix in the message (every other HTML error names the fix).
- **Suspected:** `core/cli/crates/pragma-compiler/src/validate.rs:353-354` (`n.starts_with("on") || n == "style"` share one branch).
- **Seen in:** the site's syntax page, "What the compiler rejects" (`wrong.html:7`).
- **Status:** FIXED — `:style` → `html/style-inline` ":style is not allowed: toggle a class with .name="path" (e.g. .hidden="$closed"); for a value with many options bind :data-x="path" and style [data-x=value] in CSS"; `:on<event>` stays `html/on-inline`, now with the fix ("use @click="action""). `core/cli/crates/pragma-compiler/src/validate.rs`, test `bound_style_and_handler_rules` in `tests/build.rs`. If the syntax page quotes the old output (`[html/on-inline] :style is not allowed`), update it.

## BUG-S-13 — Router view transitions ignore `prefers-reduced-motion`
- **Repro:** any app; OS/browser set to reduce motion; click an internal link in Chromium.
- **Expected:** no cross-fade (SPEC §7 promises view transitions as an enhancement; reduced-motion users should get an instant swap), or the default `::view-transition-*` animations disabled under `@media (prefers-reduced-motion: reduce)` in `core/base.css`.
- **Actual:** `document.startViewTransition(swap)` runs unconditionally, so every navigation animates the whole page (including the sticky header) for reduced-motion users. Apps cannot opt out without editing `core/`.
- **Suspected:** `core/runtime/router.js:135` (`if (document.startViewTransition) await document.startViewTransition(swap)…`). Fix: also check `matchMedia('(prefers-reduced-motion: reduce)').matches`.
- **Found by:** council review (astra P2-11, opus P2-20).
- **Status:** FIXED — the router calls `document.startViewTransition` only when `prefers-reduced-motion: reduce` does not match (checked on each navigation, so a live OS change applies); otherwise an instant swap. `core/runtime/router.js`; client test `router: view transition only without prefers-reduced-motion` (Playwright `emulateMedia`); `core/SPEC.md` §7. `site/core` synced with `pragma upgrade`.

## BUG-S-14 — `focus:not-sr-only` is emitted after `focus:fixed`, so the skip-link pattern renders static
- **Repro:** `pragma new /tmp/app`; add `<a href="#main" class="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-3 focus:z-50">Skip to content</a>` to `routes/index.html`; `pragma build`; Tab once in the browser.
- **Expected:** as in Tailwind v4 (whose subset classes.md promises: "same names, same effect"), `not-sr-only` sorts before the position utilities, so the focused link is `position: fixed` at top-left over the header.
- **Actual:** the built CSS emits `.focus\:fixed:focus{position:fixed}` … then `.focus\:not-sr-only:focus{…;position:static;…}` last, so the link becomes a static full-width bar that pushes the page down 24px. Seen on this site's layout (opus review P1-12); the site now uses its own `.skip-link` class in `lib/theme.css`.
- **Suspected:** utility ordering in `core/cli/crates/pragma-css/src/table.rs:383` (`not-sr-only` / `sr-only` are emitted after the layout utilities instead of before them).
- **Status:** FIXED — `sr-only` / `not-sr-only` are now the first rule in `RULES`, so within the same variant (`focus:`) every position, inset, z-index and spacing utility comes after `not-sr-only` and wins, as in Tailwind v4. `core/cli/crates/pragma-css/src/table.rs`, test `skip_link_not_sr_only_before_layout` in `src/tests.rs`; `core/docs/classes.md` regenerated (the `sr-only not-sr-only` line moves to the top of its section). The site can drop its `.skip-link` workaround and use `sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-3 focus:z-50` again.

## BUG-S-15 — An optional prop of a component with `.js` becomes `state.x` when no caller passes it
- **Repro:** `pragma new /tmp/app`; `lib/components/flagged/flagged.html` = `<p><span :text="$label"></span><if $flag><strong>flag on</strong></if></p>`, `flagged.js` = `export const state = () => ({ clicks: 0 })`; `routes/flag.html` = `<title>Flag</title>\n<c-flagged label="no flag passed"></c-flagged>`; `pragma check --strict` (passes), `pragma dev`, open `/flag`.
- **Expected:** components.md: "A missing prop, `""` and `"false"` are falsy". `<if $flag>` renders nothing, no error.
- **Actual:** the build emits `<template data-if="state.flag">` and the dev runtime throws `pragma: unknown path "state.flag"` on every render (production: silently `undefined`). As soon as any caller anywhere in the app passes `flag`, the same template compiles to a prop and works, so the error depends on unrelated files.
- **Suspected:** `core/cli/crates/pragma-compiler/src/expand.rs:191` (`is_state = sc.stateful && !sc.known_props…contains(&key)`): `known_props` is built only from call-site attributes (`collect_props`, expand.rs:59). A `$name` that is neither a state key nor a passed prop should be a prop (falsy), or `pragma check` should report it.
- **Seen in:** the site's `<c-frame height>`; worked around by styling the forwarded `data-height` root attribute instead of reading `$height`.
- **Status:** FIXED — the build reads the keys of the object `state()` returns (`() => ({ ... })`, a function ending in `return { ... }`, getters included); in such a component `$x` is state only for those keys, any other `$x` is a prop (missing = falsy), independent of other files. If the keys are not static (spread, computed key, early returns) the old rule stays. `core/cli/crates/pragma-jslint/src/lib.rs` (`exported_state_keys`, test `exported_state_keys_when_static`), `core/cli/crates/pragma-compiler/src/expand.rs` (`Scope::state_keys`), test `optional_prop_of_stateful_component_is_a_falsy_prop` (the repro) in `tests/build.rs`; `core/SPEC.md` §3, `core/docs/components.md`. This site's build output is byte-identical before/after. `<c-frame>` can read `$height` again.
