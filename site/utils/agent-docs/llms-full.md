# pragma: full docs for agents

> pragma is a web framework built for coding agents. An app is native HTML, CSS and plain JavaScript (never TypeScript) with a closed set of directives; a Rust CLI compiles, lints and validates it and answers every error with the file, the line, the rule and the fix. Pages ship prebuilt with only the JavaScript they use, and every form is a WebMCP tool by default.

This file has everything needed to build a pragma app: install, the loop, the app rules (AGENTS.md) and every doc. Index: {{origin}}/llms.txt. Each part is also a separate file under {{origin}}/docs/.

## Install and create

pragma is one Rust binary. The installer supports Linux and macOS. Binaries for all platforms: https://github.com/leonavas/pragma/releases/latest. Node 22 or newer only runs `.server.js` files.

```sh
curl -fsSL https://raw.githubusercontent.com/leonavas/pragma/main/install.sh | sh
pragma new my-app
cd my-app
pragma dev
```

Or build from source with Cargo:

```sh
git clone https://github.com/leonavas/pragma && cd pragma && cargo install --path core/cli/crates/pragma-cli
```

## Validate version and update

Validate the pragma version before you change code:

1. Run `pragma -V` to check the installed CLI version.
2. Compare that version with `.pragma/version` inside the app.
3. Run `pragma self-update` to update the CLI binary from GitHub Releases if the CLI is old.
4. Run `pragma upgrade` inside the app to update `.pragma/` (runtime, docs, `base.css`, `version`) if `.pragma/` is outdated.

```sh
pragma -V
cat .pragma/version
pragma self-update
pragma upgrade
```

## The loop

1. Validate the pragma version (`pragma -V` vs `.pragma/version`); run `pragma self-update` and `pragma upgrade` if outdated.
2. Read `AGENTS.md` in the app (the rules below), then the doc for the task in `.pragma/docs/`.
3. Write the code: one `.html` per page or component, plain JS, utility classes.
4. Run `pragma check --strict` and fix every error and warning at its cause, never around the rule.
5. Finish when `pragma check --strict` prints no problems. `pragma build` writes `dist/`.

## Contents

{{docs}}

---

{{all-docs}}
