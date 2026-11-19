# `pragma` CLI

| Command | Does |
|---|---|
| `pragma new <dir>` | creates an app with the fixed structure |
| `pragma dev [--port 5180] [--host 127.0.0.1]` | dev server with live reload over a WebSocket (any number of tabs); a build error shows an overlay, and pages without errors keep working, `load` included (starts Node ≥ 22 when there is a `.server.js`) |
| `pragma check [--strict]` | checks JS, HTML, CSS, budgets and accessibility; `--strict` turns warnings into errors |
| `pragma build` | writes `dist/` |
| `pragma serve [--port 3000] [--host 127.0.0.1]` | serves `dist/` as in production |
| `pragma upgrade` | makes `core/` (runtime, docs, `base.css`) equal to this `pragma` version; prints each file updated, added or removed; never touches files outside `core/` |

Run `pragma check --strict` before finishing: zero errors, zero warnings. Output:
`file:line:column error|warning [rule] message`. Fix the cause; never work around the rule.

## Rules

| Rule | Fix |
|---|---|
| `js/syntax`, `html/syntax`, `css/syntax` | valid code; HTML: close every non-void tag |
| `js/html-in-js` | no `innerHTML`, `createElement`, `document.write`…: markup goes in `.html` |
| `js/console` | `log.*` (logs.md) |
| `js/jsdoc-types` | no TypeScript or JSDoc types |
| `js/import` | import only project files, `core/` and `package.json` dependencies (packages.md); `import()` only with a quoted specifier (`import('./chart.js')`), never a variable or `${}` |
| `js/comment`, `html/comment`, `css/comment` | delete the comment; name things clearly |
| `js/vague-name` | descriptive names. Flagged: generic words (`tmp`, `data`, `info`, `handler`, `util`…), names ending in a digit, names under 3 chars except `$ i j k id ok el ctx url log` and parameters of expression-body arrows |
| `js/legacy-api` | old API (`export const state`/`actions`, `export function mount`, `ctx.$`, `ctx.update`, `store()`): top-level `let` for state, functions for actions and derived values, `function mount()`, `export let` for props (components.md, state.md) |
| `js/prop-write` | props are read-only (also through `:model`): copy into a `let` (`let current = start`) |
| `js/const-state` | a top-level `const` changed in place and read in HTML does not update the page: declare it with `let` |
| `js/store-import` | reach stores through `shared` (`import { shared } from 'pragma'`, then `shared.cart`) |
| `js/component-script` | script shape (state.md): no `var`, no destructuring `let`, only `export let` props (stores: only `export const persist`), no top-level `await`, only `pragma`'s names, `$`/`$pragma` reserved |
| `html/legacy-syntax`, `html/unknown-directive` | syntax.md. Vue, Alpine, Svelte, Angular and React attributes (`v-if`, `x-show`, `v-bind:href`, `on:click`, `*ngIf`, `[disabled]`, `className`) do nothing here: the message names the pragma form (`<if path>`, `:show`, `:href`, `@click`) |
| `html/data-attribute` | no hand-written `data-*`: reserved ones name their short form (`data-text` → `{path}` inside the element); others suggest `:data-x="path"`, `.name="path"`, `aria-*` or role/label/id |
| `html/invalid-path`, `html/invalid-root` | only `root.a.b` with a valid root (syntax.md); in `{}` close the brace (`\{` for a literal one); event arguments are paths, `'text'`, numbers, `true`/`false`/`null` or `event`, never expressions |
| `html/text-removed` | `:text` was removed: write `{path}` inside the element (`<p>{$name}</p>`) |
| `html/block`, `html/block-single-root`, `html/for-without-key`, `html/misplaced-else`, `html/misplaced-empty`, `html/outside-template` | block rules (syntax.md) |
| `html/unknown-name` | declare it at the top level of the owner's script: `let x` (state), `export let x` (prop), `function x()` (action or derived); the layout has no script |
| `html/unknown-prop` | the component's script must declare the prop: `export let name = default` |
| `html/duplicate-module` | one place per component: `<script>`/`<style>` inside the `.html` or `x.js`/`x.css`, not both |
| `html/component`, `html/unknown-component` | components.md |
| `html/layout`, `html/title`, `html/head` | routes.md (SEO) |
| `html/class` | toggle classes with `.name="path"`, never `:class` or `{}` in `class` (syntax.md) |
| `html/interpolation` | no `{{ }}`/`${ }`: write `{path}` (syntax.md) |
| `html/script`, `html/style-inline`, `html/on-inline` | one `<script>` and one `<style>`, at the top level of a page or component `.html`, without attributes (`<script>` takes only `type="module"`); otherwise behavior in a script function, style in classes |
| `css/unknown-class`, `css/arbitrary-value` | classes.md, css.md |
| `app/structure`, `app/route-conflict` | structure.md, routes.md |
| `app/package` | packages.md |
| `app/pragma-json` | valid `pragma.json` (structure.md) |
| `app/budget` | shrink the code (performance.md) |
| `app/core-outdated` (warning) | `core/` differs from this `pragma` version: run `pragma upgrade`; never edit `core/` by hand |

## Accessibility (`a11y/*`, warnings; errors with `--strict`)

| Rule | Fix |
|---|---|
| `img-alt` | `alt` on every `<img>`; `alt=""` if decorative |
| `accessible-name` | `<button>`/`<a>` with text, `{path}`, `aria-label` or `title` |
| `label` | field with `<label for>`, wrapped in `<label>`, or `aria-label` |
| `dialog-name` | `<dialog>` with `aria-label` or `aria-labelledby` |
| `click-non-interactive` | `@click` on `<button>`/`<a>`; otherwise add `role` and `tabindex` |
| `heading` | do not skip heading levels |
| `single-h1` | one `<h1>` per page (`<if>`/`<else>` branches count separately) |
| `tabindex` | only `0` or `-1` |
| `duplicate-id` | unique `id` per page |
| `iframe-title` | `title` on `<iframe>` |
| `a-no-href` | every `<a>` has `href`; actions use `<button>` |

`:<attr>` and `{}` in the attribute count as the attribute (`:aria-label="task.title"`, `aria-label="Remove {task.title}"`).

## `dist/`

| Path | Use |
|---|---|
| `client/` | HTML per route and assets; without `.server.js`, deploy it to static hosting |
| `server/worker.js` + `wrangler.jsonc` | Cloudflare Workers (the worker resolves param pages via the manifest and answers `_404.html` with 404) |
| `server/node.mjs` | `node dist/server/node.mjs` (port from `PORT`, default 3000) |
