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
| `client/` | HTML per route and assets. Deploy to static hosting when no `.server.js` exists. |
| `server/worker.js` + `wrangler.jsonc` | Cloudflare Workers (resolves param pages via manifest and serves `_404.html` with 404). |
| `server/node.mjs` | Run `node dist/server/node.mjs` (reads `PORT`, default 3000). |

Every server (`dev`, `serve`, Workers, Node) returns the matching `.md` from `public/` when `Accept` prefers `text/markdown` (`webmcp.md`).
