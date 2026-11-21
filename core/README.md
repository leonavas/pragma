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
