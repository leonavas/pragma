# pragma

Agentic, opinionated framework for web apps in native HTML, CSS and JS: prebuilt `.html` pages,
closed directives, minimal bundle, no UI framework, and a Rust CLI
(`pragma new|dev|build|check|serve`) with rules, docs and checks built for agents.

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
