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

## Pending: performance (found 2026-11-22, not started)

Lighthouse on a large real app page (escalei-pragma `/app/[id]`, see that repo's README "Status and next session") showed pragma faster on FCP/LCP than the SvelteKit original but worse on TBT, Speed Index and payload. Planned for v0.2.1:

1. Route CSS asset paths keep brackets (`/assets/css/app/[id]-<hash>.css`); Workers answers 307 to `%5Bid%5D`, a redirect on a render-blocking stylesheet. Emit URL-safe names like the JS chunks (`_id_`) for every asset and preload.
2. DOM/HTML size: prebuilt HTML repeats `<template data-if … data-tid>` inside every list item and recursive `<template data-def>`; that page had 6277 elements / 365 kB HTML vs 1838 / 1.8 kB. Emit each block template once per page and clone by id.
3. Viewport/MutationObserver link prefetch fires during page load (it fetched `/app/index.html` → 307 → `/app`). Wait for `load` + idle, respect `saveData`, request the clean URL.
4. Done (opt/boot): `boot.js` 216 ms eval and ~55 ms forced reflow. An SSR or no-loader page renders once at boot (was twice); entering block roots read animations after the next frame, not in the render; leave reads stay batched before the removals; `autofocus` focuses once at the end of the render.
5. Done (opt/boot): `performance.md` "Lazy-load heavy code" documents `await import()` of a `utils/` module inside an action (separate chunk, no modulepreload, packages tree-shaken). Auto code-split of components is not planned.
