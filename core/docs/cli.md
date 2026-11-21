# `pragma` CLI

| Command | Action |
|---|---|
| `pragma -V` | Prints the installed CLI version. |
| `pragma new <dir>` | Creates an app with the fixed structure. Warns and keeps existing files if `<dir>` is not empty. |
| `pragma dev [--port 5180] [--host 127.0.0.1]` | Runs the dev server with WebSocket live reload. Shows an overlay on build errors while valid pages keep working. Starts Node >= 22 when `.server.js` exists. Prints `app/core-outdated` first if `.pragma/` is older than the CLI. |
| `pragma check [--strict]` | Checks JS, HTML, CSS, budgets, accessibility, and WebMCP tools. `--strict` turns warnings into errors. |
| `pragma build [--allow-outdated-core]` | Builds `dist/`. Stops with `app/core-outdated` if `.pragma/` is older than the CLI (`--allow-outdated-core` overrides this). |
| `pragma serve [--port 3000] [--host 127.0.0.1]` | Serves `dist/` as in production. |
| `pragma upgrade` | Syncs `.pragma/` (runtime, docs, `base.css`, `version`) with the installed CLI version. Prints updated, added, or removed files. Never edits files outside `.pragma/`. |
| `pragma self-update [version]` | Downloads the latest or specified `pragma` binary from GitHub Releases, verifies `SHA256SUMS.txt`, and replaces the executable. |

Run `pragma check --strict` before you finish. Fix all errors and warnings.
Output format: `file:line:column error|warning [rule] message`.
Fix the root cause; do not work around a rule.

## Versioning and updates

- `.pragma/version` stores the version of `.pragma/` in the app.
- Compare `pragma -V` with `.pragma/version`.
- If they differ, or if `pragma check` reports `app/core-outdated`, run `pragma upgrade`.
- Run `pragma self-update [version]` to update the CLI binary.

## Rules

| Rule | Fix |
|---|---|
| `js/syntax`, `html/syntax`, `css/syntax` | Write valid syntax. Close every non-void HTML tag. |
| `js/html-in-js` | Remove `innerHTML`, `createElement`, `document.write`. Put markup in `.html`. |
| `js/console` | Use `log.*` (`logs.md`). |
| `js/jsdoc-types` | Remove TypeScript and JSDoc types. |
| `js/import` | Import only project files, `.pragma/`, and `package.json` dependencies (`packages.md`). Pass a quoted specifier to `import()` (`import('./chart.js')`), never a variable or `${}`. |
| `js/comment`, `html/comment`, `css/comment` | Delete the comment. Use clear names. |
| `js/vague-name` | Use descriptive names. Avoid generic words (`tmp`, `data`, `info`, `handler`, `util`), names ending in a digit, and names under 3 chars except `$ i j k id ok el ctx url log` and parameters of expression-body arrows. |
| `js/legacy-api` | Replace old API (`export const state`/`actions`, `export function mount`, `ctx.$`, `ctx.update`, `store()`). Use top-level `let`, functions, `function mount(el)`, and `export let` (`components.md`, `state.md`). |
| `js/prop-write` | Do not mutate props (including via `:model`). Copy into a `let` (`let current = start`). |
| `js/const-state` | Declare mutated state with `let`, not `const`. |
| `js/store-import` | Access stores through `shared` (`import { shared } from 'pragma'`, then `shared.cart`). |
| `js/component-script` | Follow script rules (`state.md`): no `var`, no destructuring `let`, only `export let` props (stores export only `persist`), no top-level `await`, only valid `pragma` imports, no `$`/`$pragma` declarations. |
| `js/top-level-data` | Read `data` inside functions (`const products = () => data.products`), not at top level. |
| `js/stale-derived` | Convert static derived values to functions (`const doubled = () => count * 2`), or update them in actions. |
| `js/derived-call` | Call derived functions in JS (`empty()`), not as bare values (`if (empty)`). |
| `js/dollar-in-script` | Write `x` in JS; `$x` is HTML only. |
| `html/legacy-syntax`, `html/unknown-directive` | Use pragma syntax (`syntax.md`). Replace foreign attributes (`v-if`, `x-show`, `v-bind:href`, `on:click`, `*ngIf`, `[disabled]`, `className`) with pragma forms (`<if path>`, `:show`, `:href`, `@click`). |
| `html/data-attribute` | Do not write `data-*` by hand. Use short forms (`{path}` instead of `data-text`), `:data-x="path"`, `.name="path"`, `aria-*`, or role/label/id. |
| `html/invalid-path`, `html/invalid-root` | Use `root.a.b` with a valid root (`syntax.md`). Close `{}` (`\{` for literal). Pass only paths, `'text'`, numbers, `true`/`false`/`null`, `event`, or `el` to events. Omit `$` in event names. Use bare paths in `:attr` and `<if>`. |
| `html/text-removed` | Replace `:text` with `{path}` inside the element (`<p>{$name}</p>`). |
| `html/block`, `html/block-single-root`, `html/for-without-key`, `html/misplaced-else`, `html/misplaced-empty`, `html/outside-template` | Follow block rules (`syntax.md`). |
| `html/unknown-name` | Declare the name at the top level of the owner script (`let x`, `export let x`, `function x()`). Layouts have no script. In `@click="shared.cart.x(…)"`, `x` must be a store function. |
| `html/unknown-prop` | Declare the prop in the component script (`export let name = default`). |
| `html/duplicate-module` | Use inline `<script>`/`<style>` or external `x.js`/`x.css`, not both. |
| `html/component`, `html/unknown-component` | Follow component rules (`components.md`). |
| `html/layout`, `html/title`, `html/head` | Follow layout and SEO rules (`routes.md`). |
| `html/class` | Toggle classes with `.name="path"`. Do not use `:class` or `{}` in `class` (`syntax.md`). |
| `html/interpolation` | Replace `{{ }}` and `${ }` with `{path}` (`syntax.md`). |
| `html/script`, `html/style-inline`, `html/on-inline` | Put at most one `<script>` (optional `type="module"`) and one `<style>` at the top level of a page or component `.html`. Put JSON-LD `<script type="application/ld+json">` only at the top level of a page (`routes.md`). |
| `html/json-ld` | Fix invalid JSON or unclosed `{` in JSON-LD strings. Use `{` for a literal brace. |
| `css/unknown-class`, `css/arbitrary-value` | Use classes from `classes.md` or custom classes per `css.md`. |
| `app/structure`, `app/route-conflict` | Follow structure and route rules (`structure.md`, `routes.md`). |
| `app/endpoint`, `app/middleware` | An endpoint exports only `GET`/`POST`/`PUT`/`PATCH`/`DELETE`; the middleware exports `handle` (`data.md`). |
| `app/endpoint-json` (warning) | An endpoint answers JSON. Use `load` and actions for page data (`data.md`). |
| `app/package` | Follow package rules (`packages.md`). |
| `app/pragma-json` | Fix `pragma.json` (`structure.md`). |
| `app/budget` | Reduce bundle size (`performance.md`). |
| `app/core-outdated` (warning; error in `pragma build`) | `.pragma/` differs from the CLI version. Run `pragma upgrade`. Do not edit `.pragma/` by hand. |

## Accessibility (`a11y/*`, warnings; errors with `--strict`)

| Rule | Fix |
|---|---|
| `img-alt` | Add `alt` to every `<img>`. Use `alt=""` for decorative images. |
| `accessible-name` | Give `<button>` and `<a>` text, `{path}`, `aria-label`, or `title`. |
| `label` | Associate fields with `<label for>`, wrap in `<label>`, or add `aria-label`. |
| `dialog-name` | Add `aria-label` or `aria-labelledby` to `<dialog>`. |
| `click-non-interactive` | Put `@click` on `<button>` or `<a>`, or add `role` and `tabindex`. |
| `heading` | Do not skip heading levels. |
| `single-h1` | Use one `<h1>` per page (`<if>`/`<else>` branches count separately). |
| `tabindex` | Use only `0` or `-1`. |
| `duplicate-id` | Use a unique `id` per page. |
| `iframe-title` | Add `title` to `<iframe>`. |
| `a-no-href` | Add `href` to every `<a>`. Use `<button>` for actions. |

`:<attr>` and `{}` count as setting the attribute (`:aria-label="task.title"`, `aria-label="Remove {task.title}"`).

## WebMCP (`webmcp/*`, active when `"webmcp": true`)

Every `<form>` is an agent tool (`webmcp.md`).
- Errors: `webmcp/pair`, `webmcp/name`, `webmcp/duplicate`, `webmcp/param-name`, `webmcp/dynamic`, `webmcp/notool`, `webmcp/attribute`. With `"webmcp": false`, any tool attribute triggers `webmcp/disabled`.
- Warnings (errors with `--strict`): `webmcp/form-tool`, `webmcp/name-style`, `webmcp/description`, `webmcp/param-description`, `webmcp/autosubmit`.

## `dist/`

| Path | Use |
|---|---|
| `client/` | HTML per route and assets. |
| `server/worker.js` + `wrangler.jsonc` | Cloudflare Workers (resolves param pages via manifest and serves `_404.html` with 404). Emitted when the app has a `*.server.js` or a `public/**/*.md`. `run_worker_first` sends every request, also prebuilt pages, through the worker, so middleware runs for them. |
| `server/node.mjs` | Run `node dist/server/node.mjs` (reads `PORT`, default 3000). Emitted when the app has a `*.server.js`. |

Every server (`dev`, `serve`, Workers, Node) returns the matching `.md` from `public/` when `Accept` prefers `text/markdown` (`webmcp.md`).

## Deploy

Cloudflare Workers is the default target. Deploy only when the user asks. Use another target (Node, static hosting) only when the user asks for it.

### Before the first deploy

1. Set `name` in `pragma.json` to a unique Worker name (lowercase letters, digits, hyphens). The name becomes the Worker name and the `<name>.<account>.workers.dev` URL. A deploy replaces any Worker with the same name in the account, so do not keep the starter name `app`. If the user gave no name, ask.
2. Run `pragma check --strict`, then `pragma build`. Fix all errors.
3. If `dist/server/wrangler.jsonc` does not exist (no `*.server.js` and no `public/**/*.md`), add `public/index.md` (`webmcp.md`) and build again. Without the worker, Workers cannot serve param pages (`[id].html`) or `_404.html`. Tell the user about the new file.
4. Log in to Cloudflare. Local: `npx wrangler@4 login`. CI: set `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`. Do not write tokens into files.

### Deploy

```sh
pragma build
npx wrangler@4 deploy --config dist/server/wrangler.jsonc
```

Wrangler prints the URL. Then check:
- `curl -I <url>` returns 200 and `curl -I <url>/missing-page` returns 404.
- `curl -H 'Accept: text/markdown' <url>` returns markdown when `public/index.md` exists.
- `npx wrangler@4 tail <name>` shows the live logs (`logs.md`).

### Secrets, variables, bindings, domain

`pragma build` rewrites `dist/`. Do not edit `dist/server/wrangler.jsonc`. For more configuration, create `wrangler.jsonc` in the app root. Use the same `name` as `pragma.json` and point to `dist/`. Keep only the keys the app uses:

```jsonc
{
  "name": "my-app",
  "main": "dist/server/worker.js",
  "compatibility_date": "2025-11-01",
  "assets": { "directory": "dist/client", "binding": "ASSETS", "run_worker_first": true },
  "vars": { "PRAGMA_LOG_LEVEL": "info" },
  "d1_databases": [{ "binding": "DB", "database_name": "my-app", "database_id": "<id>" }],
  "kv_namespaces": [{ "binding": "KV", "id": "<id>" }],
  "r2_buckets": [{ "binding": "FILES", "bucket_name": "my-app" }],
  "routes": [{ "pattern": "app.example.com", "custom_domain": true }]
}
```

Then deploy with `npx wrangler@4 deploy` (no `--config`).

| Need | Do | Read in `.server.js` |
|---|---|---|
| Secret (API key, token) | `npx wrangler@4 secret put KEY_NAME --name <name>`. Never put a secret in `vars` or in a file. | `env.KEY_NAME` |
| Plain variable | `vars` in the root `wrangler.jsonc`. | `env.NAME` |
| SQL database (D1) | `npx wrangler@4 d1 create <db>`, then binding `DB`. | `db.get`, `db.all`, `db.run` |
| Key-value store (KV) | `npx wrangler@4 kv namespace create <ns>`, then binding `KV`. | `kv.get`, `kv.put`, `kv.delete` |
| File storage (R2) | `npx wrangler@4 r2 bucket create <bucket>`, then binding `FILES`. | `files.get`, `files.put`, `files.delete` |
| Custom domain | `routes` with `custom_domain: true`. The zone must be in the same Cloudflare account. | |
| Log export (OTLP) | `OTEL_*` in `vars`, headers with a token as a secret (`logs.md`). | |

`load` and actions receive `env`, `db`, `kv`, and `files`. In `pragma dev` and `pragma serve`, `env` is the process environment (`KEY_NAME=value pragma dev`), and `db`, `kv`, and `files` are `undefined`. Check them before use.

### Workers runtime limits

- There is no filesystem. Do not use `node:fs`, `process.cwd()`, or paths on disk in `.server.js`. Import JSON with `import products from './_products.json' with { type: 'json' }`.
- Read configuration from `env`, not `process.env`.
- The generated config has no `nodejs_compat`. If a package needs `node:*` modules, add `"compatibility_flags": ["nodejs_compat"]` to the root `wrangler.jsonc` and tell the user.
- If a feature cannot work on Workers, stop and tell the user before you change the target.
- WebSocket endpoints (`status: 101`) work only on Workers. `pragma dev` and `pragma serve` run Node and cannot upgrade; test them with `npx wrangler@4 dev --config dist/server/wrangler.jsonc`.

### CI (GitHub Actions)

Install the CLI version from `.pragma/version`, so `pragma build` does not stop with `app/core-outdated`:

```yaml
name: Deploy
on:
  push:
    branches: [main]
concurrency:
  group: deploy
  cancel-in-progress: true
jobs:
  deploy:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - name: Install pragma
        run: |
          curl -fsSL https://raw.githubusercontent.com/leonavas/pragma/main/install.sh | PRAGMA_VERSION="$(cat .pragma/version)" sh
          echo "$HOME/.local/bin" >> "$GITHUB_PATH"
      - run: npm ci --ignore-scripts
        if: hashFiles('package-lock.json') != ''
      - run: pragma check --strict
      - run: pragma build
      - run: npx --yes wrangler@4 deploy --config dist/server/wrangler.jsonc
        env:
          CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          CLOUDFLARE_ACCOUNT_ID: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
```

With a root `wrangler.jsonc`, remove `--config dist/server/wrangler.jsonc`. The API token needs the "Edit Cloudflare Workers" template permissions.
