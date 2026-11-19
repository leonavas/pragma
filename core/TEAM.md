# v0 team: rules for agents

Contract: `core/SPEC.md` (read it fully before starting; it is the source of truth).
Context: `core/DECISIONS.md` (may be under rewrite; if it disagrees, SPEC wins).

## Owners (edit ONLY your files; others are editing the rest now)

| Role | Files |
|---|---|
| css | `core/cli/crates/pragma-css/**`, `core/docs/classes.md` |
| jslint | `core/cli/crates/pragma-jslint/**` |
| compiler | `core/cli/crates/pragma-compiler/**` |
| cli | `core/cli/crates/pragma-cli/**` |
| client-runtime | `core/runtime/{boot,router,directives,state,page,loader}.js`, `core/runtime/test/client/**` |
| server-runtime | `core/runtime/log.js`, `core/runtime/server/**`, `core/runtime/test/server/**` |
| overlays | `core/runtime/overlay.js`, `core/runtime/test/overlay/**`, `core/starter/**` (except AGENTS.md and CLAUDE.md), `core/docs/overlays.md` |
| docs | `core/DECISIONS.md`, `core/README.md`, `core/docs/**` (except classes.md and overlays.md), `core/starter/AGENTS.md`, `core/starter/CLAUDE.md` |

Nobody edits `SPEC.md`, `TEAM.md`, `pragma-diag` or the root `core/cli/Cargo.toml`; ask the
coordinator. You may add dependencies to your crate's `Cargo.toml`.

## Rules

- Need something from another owner? Implement against the contract and note it in your final
  answer. Don't change existing public signatures; adding functions is fine.
- Rust: `CARGO_TARGET_DIR=/tmp/pragma-target-<role>` (avoids blocking others' builds).
  Edition 2024, few dependencies, current versions.
- JS: plain ES modules; no TypeScript, no JSDoc types, no runtime dependencies.
  Test tools (e.g. Playwright) only in `/tmp/pragma-tools-<role>`, never in the repo.
- No comments anywhere (JS, HTML, CSS, Rust, tests). Code is the documentation: names say
  what a thing is or does. Small, direct code; no speculative abstraction.
- Everything in English: user-facing messages (errors, CLI, docs) and identifiers.
- Tests: write them for what you deliver; run them only per `TESTING.md` (recommend, author accepts).
- Final answer: max 15 lines: what you delivered, how to test, what is missing, and where you
  interpreted or diverged from SPEC.
