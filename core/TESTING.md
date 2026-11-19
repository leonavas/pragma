# pragma tests

> **When to run:** automated tests do not run on every change. Agents run them only when they
> recommend it and the author accepts. Exception: tests the request itself asked to create or fix.

Run from `core/cli/` with `cargo xtask test [target]`:

| Target | Runs |
|---|---|
| `quick` (default) | `rust` + `server` |
| `rust` | crates: css, jslint, compiler (incl. a11y), cli |
| `server` | handler, SSR, OTel logs, Node adapter |
| `client` | directives, router, loader, stores (Chromium) |
| `overlays` | modal, alert, dropdown, back, scroll lock (Chromium and Firefox) |
| `e2e` | full example app under `pragma serve` and `pragma dev` |
| `all` | all of the above except `packages` |
| `packages` | **opt-in, not in `all`**: compiler with real npm packages (date-fns, @date-fns/tz, es-toolkit, valibot, fuse.js, chart.js, dayjs, lodash). Prod: `check --strict`, deterministic `build` (two clean builds identical), 1 entry per package in `assets/npm/`, exact per-page `modulepreload`, gzip caps per page, tree-shaking, lodash shared once. Chromium and Firefox under `pragma serve` and `pragma dev`: values, interactions (chart update, valibot, fuse, debounce, pt-BR + time zone, dayjs plugin), zero errors. Dev live reload: new import, `npm install` via `package.json`, error overlay and recovery. Rejections (`js/import`, `app/package`). Needs network the first time |

- Implemented in `core/cli/xtask/` (standard `cargo xtask`; alias in `core/cli/.cargo/config.toml`).
- First browser test run installs Playwright in `~/.cache/pragma-tools` (outside the repo) and
  browsers in `~/.cache/ms-playwright`. `PRAGMA_TOOLS_DIR` overrides the folder.
- `e2e` and `packages` build `pragma` to `core/cli/target/release/pragma` and use that binary.
- `packages` lives in `core/cli/xtask/packages/`: fixture app (`app/`, versions pinned in
  `package-lock.json`) and `packages.test.mjs`. `node_modules` is fetched with `npm ci` the first
  time into `~/.cache/pragma-tools/packages/<lockfile hash>/`, with a private npm cache (`npm-cache/`)
  that also holds `ms`, installed with `npm install --offline` by the live reload test; changing the
  lockfile refetches. `BROWSERS=chromium` limits the browsers.

Manual testing:

```sh
cd core/example
pragma dev            # http://127.0.0.1:5180, live reload
pragma check --strict # errors + accessibility warnings
pragma build && pragma serve
```
