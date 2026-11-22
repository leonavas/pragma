# pragma

Agentic, opinionated framework for web apps in native HTML, CSS and JS: prebuilt `.html` pages,
closed directives, minimal bundle, no UI framework, and a Rust CLI
(`pragma new|dev|build|check|serve`) with rules, docs and checks built for agents.

Agent-first both ways: agents build with it, and the apps work for agents. Every `<form>` is a
WebMCP tool by default (checked by `pragma check`), and pages can serve markdown to fetch agents
([docs/webmcp.md](docs/webmcp.md)).

- v0 contract: [SPEC.md](SPEC.md). Decisions and rationale: [DECISIONS.md](DECISIONS.md).
- Tests: [TESTING.md](TESTING.md). Agent team rules: [TEAM.md](TEAM.md).

## Repository layout

```
pragma/
  core/
    SPEC.md  DECISIONS.md  TEAM.md  TESTING.md  README.md
    docs/        LLM docs; copied to <app>/core/docs/
    runtime/     framework JS; copied to <app>/core/
      server/    fetch handler, SSR, Node adapter, server logs
    starter/     app skeleton used by `pragma new` (AGENTS.md, routes/, lib/…)
    cli/         Rust workspace
      crates/pragma-diag      shared diagnostics
      crates/pragma-css       Tailwind v4 subset + base CSS
      crates/pragma-jslint    JS rules (oxc)
      crates/pragma-compiler  build, analysis, directives
      crates/pragma-cli       `pragma` binary
    example/     example app generated with `pragma new`
```

The SvelteKit × astra × fable experiment moved out of the repo to `../pragma-tests_1/`.

## Pending validation: agent-first and WebMCP

- Static-only apps (no `*.server.js`) get no worker, so Cloudflare has no markdown negotiation.
- WebMCP tools on the site (`get_pragma_docs`, `get_setup_instructions`, `add_task`,
  `sign_guestbook`) not tried in a browser with WebMCP (Chrome origin trial).
- Site: placeholder origin `https://pragma.dev` in the prompt and md links; landing copy lives in
  `site/routes/*.html` and `site/utils/agent-docs/*.md` (drift check compares headings only).

## Performance work from escalei-pragma (found 2026-11-22, shipped in v0.2.1)

Lighthouse on a large real app page (escalei-pragma `/app/[id]`, see that repo's README "Status and next session") showed pragma faster on FCP/LCP than the SvelteKit original but worse on TBT, Speed Index and payload. Planned for v0.2.1:

1. Done: route CSS and dev/unbundled JS asset names use the bundler's URL-safe names (`_id_`). Was: route CSS asset paths keep brackets (`/assets/css/app/[id]-<hash>.css`); Workers answers 307 to `%5Bid%5D`, a redirect on a render-blocking stylesheet. Emit URL-safe names like the JS chunks (`_id_`) for every asset and preload.
2. Done (opt/templates): each block `<template>` is an empty anchor; its body is emitted once per page in `<template data-tpl="<tid>">` (end of `<main data-page>` content, end of `<body>` for the layout), the tid is a hash of the body (equal bodies shared), and the runtime and SSR clone by tid. `/app/[id]`: 365.6 → 317.1 kB HTML (gzip 60.9 → 56.1 kB, brotli 47.7 → 48.0 kB), 858 → 564 block bodies, template nesting 12 → 2 levels; one chat line clones 220 B / 2 empty anchors (was 66 kB / 234 nested templates), one markdown node 2.1 kB (was 7.4 kB). Was: prebuilt HTML repeats `<template data-if … data-tid>` inside every list item and recursive `<template data-def>`; that page had 6277 elements / 365 kB HTML vs 1838 / 1.8 kB.
3. Done: viewport prefetch is skipped with Save-Data and requests the clean page URL (navigation too). Was: viewport/MutationObserver link prefetch fires during page load (it fetched `/app/index.html` → 307 → `/app`). Wait for `load` + idle, respect `saveData`, request the clean URL.
4. Done (opt/boot): `boot.js` 216 ms eval and ~55 ms forced reflow. An SSR or no-loader page renders once at boot (was twice); entering block roots read animations after the next frame, not in the render; leave reads stay batched before the removals; `autofocus` focuses once at the end of the render.
5. Done (opt/boot): `performance.md` "Lazy-load heavy code" documents `await import()` of a `utils/` module inside an action (separate chunk, no modulepreload, packages tree-shaken). Auto code-split of components is not planned.
6. Done (opt/anchors, v0.2.2): block anchors are comments (`<!--p:if t3 state.open-->`), and block and self-call bodies leave the HTML for a per-page template module (`/assets/tpl/<route>-<hash>.js`, modulepreloaded, imported with the route modules, each body parsed on first use); tids are short base36 per page. `/app/[id]`: HTML 317.1 → 14.1 kB (gzip 56.1 → 4.0 kB, brotli 48.0 → 3.8 kB), `<template>` in the HTML 1227 → 0 (35 anchors left); the module is 259.7 kB (gzip 47.3 kB, brotli 41.1 kB, 566 bodies), so HTML + module brotli is 44.9 kB (was 48.0). Example app: no `<template>` left in the live DOM (`/shop` 238 → 227 elements, `/tasks` 94 → 84). Was: 878 empty `<template>` anchors and 564 hoisted bodies (~300 kB) in the page HTML and DOM.
7. Done (opt/slim, v0.2.2): the `data-s` scope token goes only on elements whose ancestors on the page can also match a selector of the owner (was: element name and classes only). `/app/[id]`: 1641 to 1375 tokens, 28.0 to 23.5 kB raw.
8. Done (opt/css-assets, v0.2.3): CSS `url(/path)` to a `public/` file (fonts, CSS images) points to a hashed copy in `/assets/` (`/assets/fonts/<name>-<hash>.woff2`), font preloads too, and the Workers handler serves `/assets/*` as `immutable` (Workers answered `max-age=0, must-revalidate` for every asset). Was: `/fonts/*.woff2` revalidated (304) on every visit, so the hero painted the fallback font and swapped.

## Untangle refactor (2026-11-22, v0.2.3)

One pass of the "untangle-codebase" method over core/ (audits of ~280 problems, fixes in three areas plus a second round). Behavior is the same except the bugs below, each with a regression test.

- Runtime: `contract.js` owns routes (match, order, page URL, `/_data` `/_action` `/_otel`), DOM ids and binding reads for client and server; `loader.js` only fetches; no loader/router/page cycles; overlays load from the `overlays` flag in `#pragma-route`.
- Compiler: one expanded page tree (no separate SSR copy), no "no layout" branches, `run()` split into analyze / emit client / emit server, framework path and block grammar decided once, leaf `hash.rs`.
- Tooling: one import-specifier model (lint, bundling, minify), one CSS scanner, legacy `core/` layout knowledge in `legacy.rs`, contract tests for lists copied across Rust, JS and docs, release targets in one list.
- Bugs fixed: SSR `{query.x}` without the param and null in the middle of a path (500, now empty like the client); overlays only inside blocks; client route order vs server; store named `theme`; number literals like `.5`/`1e3`; layout `aria-current` stuck on the first page; duplicate `:model` error; byte columns in diagnostics; template-literal `import()`; `pragma upgrade` deleting all of `core/` (data loss) and not resumable; ANSI codes in redirected stdout; self-update without checksum; quoted `)` in a selector moving the scope; non-UTF-8 import panic; ignored write errors (new rule `app/output`); dev/serve decoding `%2F` before splitting; loop variable named `constructor`.
- Deferred: Rust dev/serve router still mirrors `handler.js`; one parse per JS file in the compiler; `check` without a full prod build; `let _` in Writer prune and the chunk cache.
