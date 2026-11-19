# `pragma` CLI

| Command | Does |
|---|---|
| `pragma new <dir>` | creates an app with the fixed structure |
| `pragma dev [--port 5180] [--host 127.0.0.1]` | dev server with live reload (starts Node ≥ 22 when there is a `.server.js`) |
| `pragma check [--strict]` | checks JS, HTML, CSS, budgets and accessibility; `--strict` turns warnings into errors |
| `pragma build` | writes `dist/` |
| `pragma serve [--port 3000] [--host 127.0.0.1]` | serves `dist/` as in production |

Run `pragma check --strict` before finishing: zero errors, zero warnings. Output:
`file:line:column error|warning [rule] message`. Fix the cause; never work around the rule.

## Rules

| Rule | Fix |
|---|---|
| `js/syntax`, `html/syntax`, `css/syntax` | valid code; HTML: close every non-void tag |
| `js/html-in-js` | no `innerHTML`, `createElement`, `document.write`…: markup goes in `.html` |
| `js/console` | `log.*` (logs.md) |
| `js/jsdoc-types` | no TypeScript or JSDoc types |
| `js/import` | import only project files, `core/` and `package.json` dependencies (packages.md) |
| `js/comment`, `html/comment`, `css/comment` | delete the comment; name things clearly |
| `js/vague-name` | descriptive names. Flagged: generic words (`tmp`, `data`, `info`, `handler`, `util`…), names ending in a digit, names under 3 chars except `$ i j k id ok el ctx url log` and parameters of expression-body arrows |
| `js/ctx` | `ctx.$`, not `ctx.state`/`ctx.props` |
| `html/legacy-syntax`, `html/unknown-directive` | syntax.md |
| `html/invalid-path`, `html/invalid-root` | only `root.a.b` with a valid root (syntax.md) |
| `html/block`, `html/block-single-root`, `html/for-without-key`, `html/misplaced-else`, `html/misplaced-empty`, `html/outside-template` | block rules (syntax.md) |
| `html/unknown-action` | export the action from the owner's `.js` |
| `html/component`, `html/unknown-component` | components.md |
| `html/layout`, `html/title` | routes.md |
| `html/script`, `html/style-inline`, `html/on-inline` | behavior in `.js`, style in classes |
| `css/unknown-class`, `css/arbitrary-value` | classes.md, css.md |
| `app/structure`, `app/route-conflict` | structure.md, routes.md |
| `app/package` | packages.md |
| `app/pragma-json` | valid `pragma.json` (structure.md) |
| `app/budget` | shrink the code (performance.md) |

## Accessibility (`a11y/*`, warnings; errors with `--strict`)

| Rule | Fix |
|---|---|
| `img-alt` | `alt` on every `<img>`; `alt=""` if decorative |
| `accessible-name` | `<button>`/`<a>` with text, `:text`, `aria-label` or `title` |
| `label` | field with `<label for>`, wrapped in `<label>`, or `aria-label` |
| `dialog-name` | `<dialog>` with `aria-label` or `aria-labelledby` |
| `click-non-interactive` | `@click` on `<button>`/`<a>`; otherwise add `role` and `tabindex` |
| `heading` | do not skip heading levels |
| `single-h1` | one `<h1>` per page (`<if>`/`<else>` branches count separately) |
| `tabindex` | only `0` or `-1` |
| `duplicate-id` | unique `id` per page |
| `iframe-title` | `title` on `<iframe>` |
| `a-no-href` | every `<a>` has `href`; actions use `<button>` |

`:<attr>` counts as the attribute (`:aria-label="task.title"`).

## `dist/`

| Path | Use |
|---|---|
| `client/` | HTML per route and assets; without `.server.js`, deploy it to static hosting |
| `server/worker.js` + `wrangler.jsonc` | Cloudflare Workers |
| `server/node.mjs` | `node dist/server/node.mjs` (port from `PORT`, default 3000) |
