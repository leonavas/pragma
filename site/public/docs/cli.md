# `pragma` CLI

| Command | Does |
|---|---|
| `pragma new <dir>` | creates an app with the fixed structure (warns and keeps existing files if `<dir>` is not empty) |
| `pragma dev [--port 5180] [--host 127.0.0.1]` | dev server with live reload over a WebSocket (any number of tabs); a build error shows an overlay, and pages without errors keep working, `load` included (starts Node ≥ 22 when there is a `.server.js`); prints `app/core-outdated` first when `core/` is older than this `pragma` |
| `pragma check [--strict]` | checks JS, HTML, CSS, budgets, accessibility and WebMCP tools; `--strict` turns warnings into errors |
| `pragma build [--allow-outdated-core]` | writes `dist/`; stops with `app/core-outdated` when `core/` is older than this `pragma` (run `pragma upgrade`; `--allow-outdated-core` builds anyway) |
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
| `js/legacy-api` | old API (`export const state`/`actions`, `export function mount`, `ctx.$`, `ctx.update`, `store()`): top-level `let` for state, functions for actions and derived values, `function mount(el)`, `export let` for props (components.md, state.md) |
| `js/prop-write` | props are read-only (also through `:model`): copy into a `let` (`let current = start`) |
| `js/const-state` | a top-level `const` changed in place and read in HTML does not update the page: declare it with `let` |
| `js/store-import` | reach stores through `shared` (`import { shared } from 'pragma'`, then `shared.cart`) |
| `js/component-script` | script shape (state.md): no `var`, no destructuring `let`, only `export let` props (stores: only `export const persist`), no top-level `await`, only `pragma`'s names, `$`/`$pragma` reserved |
| `js/top-level-data` | `data` read when the script starts (before the loader is done): read it inside a function (`const products = () => data.products`) |
| `js/stale-derived` | a top-level `let`/`const` computed once from state or a prop and never assigned again: make it a function (`const doubled = () => count * 2`), or assign it in an action |
| `js/derived-call` | a derived function used as a value in JS (`if (empty)`, `` `${total}` ``): call it (`empty()`) |
| `js/dollar-in-script` | `$x` is HTML only: in a script write `x` |
| `html/legacy-syntax`, `html/unknown-directive` | syntax.md. Vue, Alpine, Svelte, Angular and React attributes (`v-if`, `x-show`, `v-bind:href`, `on:click`, `*ngIf`, `[disabled]`, `className`) do nothing here: the message names the pragma form (`<if path>`, `:show`, `:href`, `@click`) |
| `html/data-attribute` | no hand-written `data-*`: reserved ones name their short form (`data-text` → `{path}` inside the element); others suggest `:data-x="path"`, `.name="path"`, `aria-*` or role/label/id |
| `html/invalid-path`, `html/invalid-root` | only `root.a.b` with a valid root (syntax.md); in `{}` close the brace (`\{` for a literal one); event arguments are paths, `'text'`, numbers, `true`/`false`/`null`, `event` or `el`, never expressions; no `$` in an event name; `:attr` takes a bare path; no operators in `<if>`. Messages suggest the closest name ("did you mean `$count`?") |
| `html/text-removed` | `:text` was removed: write `{path}` inside the element (`<p>{$name}</p>`) |
| `html/block`, `html/block-single-root`, `html/for-without-key`, `html/misplaced-else`, `html/misplaced-empty`, `html/outside-template` | block rules (syntax.md) |
| `html/unknown-name` | declare it at the top level of the owner's script: `let x` (state), `export let x` (prop), `function x()` (action or derived); the layout has no script. For `@click="shared.cart.x(…)"`, `x` must be a function of the store |
| `html/unknown-prop` | the component's script must declare the prop: `export let name = default` |
| `html/duplicate-module` | one place per component: `<script>`/`<style>` inside the `.html` or `x.js`/`x.css`, not both |
| `html/component`, `html/unknown-component` | components.md |
| `html/layout`, `html/title`, `html/head` | routes.md (SEO) |
| `html/class` | toggle classes with `.name="path"`, never `:class` or `{}` in `class` (syntax.md) |
| `html/interpolation` | no `{{ }}`/`${ }`: write `{path}` (syntax.md) |
| `html/script`, `html/style-inline`, `html/on-inline` | one `<script>` and one `<style>`, at the top level of a page or component `.html`, without attributes (`<script>` takes only `type="module"`); otherwise behavior in a script function, style in classes. JSON-LD `<script type="application/ld+json">` only at the top level of a page (routes.md, SEO) |
| `html/json-ld` | the JSON-LD is not valid JSON, or a `{` in one of its strings is not closed: fix the JSON at the line shown; a literal `{` in a string is `{` |
| `css/unknown-class`, `css/arbitrary-value` | classes.md, css.md |
| `app/structure`, `app/route-conflict` | structure.md, routes.md |
| `app/package` | packages.md |
| `app/pragma-json` | valid `pragma.json` (structure.md) |
| `app/budget` | shrink the code (performance.md) |
| `app/core-outdated` (warning; an error in `pragma build`) | `core/` differs from this `pragma` version: run `pragma upgrade`; never edit `core/` by hand. An old runtime cannot render what a newer compiler emits (SSR "unknown op") |

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

## WebMCP (`webmcp/*`, only with `"webmcp": true`)

Every `<form>` is an agent tool (webmcp.md). Errors (the tool is broken): `webmcp/pair`,
`webmcp/name`, `webmcp/duplicate`, `webmcp/param-name`, `webmcp/dynamic`, `webmcp/notool`,
`webmcp/attribute`; with `"webmcp": false`, any tool attribute is `webmcp/disabled`. Warnings
(errors with `--strict`): `webmcp/form-tool`, `webmcp/name-style`, `webmcp/description`,
`webmcp/param-description`, `webmcp/autosubmit`. Each rule and its fix: webmcp.md.

## `dist/`

| Path | Use |
|---|---|
| `client/` | HTML per route and assets; without `.server.js`, deploy it to static hosting |
| `server/worker.js` + `wrangler.jsonc` | Cloudflare Workers (the worker resolves param pages via the manifest and answers `_404.html` with 404) |
| `server/node.mjs` | `node dist/server/node.mjs` (port from `PORT`, default 3000) |

Every server (`dev`, `serve`, Workers, Node) answers a request whose `Accept` prefers
`text/markdown` with the page's `.md` from `public/` when it exists (webmcp.md, Markdown for agents).
