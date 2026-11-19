# pragma: full docs for agents

> pragma is a web framework built for coding agents. An app is native HTML, CSS and plain JavaScript (never TypeScript) with a closed set of directives; a Rust CLI compiles, lints and validates it and answers every error with the file, the line, the rule and the fix. Pages ship prebuilt with only the JavaScript they use, and every form is a WebMCP tool by default.

This file has everything needed to build a pragma app: install, the loop, the app rules (AGENTS.md) and every doc. Index: {{origin}}/llms.txt. Each part is also a separate file under {{origin}}/docs/.

## Install and create

pragma is one Rust binary. Building it needs Cargo; Node 22 or newer only runs `.server.js` files.

```sh
git clone https://github.com/leonavas/pragma
cd pragma
cargo install --path core/cli/crates/pragma-cli
cd ..
pragma new my-app
cd my-app
pragma dev
```

## The loop

1. Read `AGENTS.md` in the app (the rules below), then the doc for the task.
2. Write the code: one `.html` per page or component, plain JS, utility classes.
3. Run `pragma check --strict` and fix every error and warning at its cause, never around the rule.
4. Done when `pragma check --strict` prints no problems. `pragma build` writes `dist/`.

## Contents

{{docs}}

---

{{all-docs}}
