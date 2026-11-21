# AGENTS.md

A pragma app uses native HTML, CSS, and JS without a framework. Read the relevant file in `.pragma/docs/` before you change code.

## Version check

1. Compare `pragma -V` with `.pragma/version`.
2. If they differ, or if `pragma check` reports `app/core-outdated`, run `pragma upgrade`.
3. Run `pragma self-update` to update the CLI binary.

## Syntax

A page or component is one `.html` file: optional `<script>`, optional `<style>`, then markup.

- Top-level `let` declares state.
- A function defines an action (`@click="add"`) or a derived value (`{$total}` in HTML, `total()` in JS).
- `export let step = 1` declares a prop.
- `function mount(el) { …; return cleanup }` runs after mount.
- Import services from `'pragma'`: `import { shared, data, params, query, page, call, stream, navigate, open, close, confirm, log } from 'pragma'`.
- `<style>` scopes to its component or page (`:global(…)` opts out only its argument).
- `{path}` inserts escaped text in text and attribute strings (`<p>Hi {$name}</p>`, `href="/shop/{p.id}"`).
- `:` binds a value (`:href`, `:disabled`). `.` toggles a class. `@` binds an event.
- `$` references a `<script>` name in HTML only (write `name` in JS).
- `@input="search"` passes `event`. `@change="toggle(task)"` passes only the listed arguments (paths, literals, `event`, `el`).
- Call store functions directly: `@click="shared.cart.remove(item.id)"`.
- Use tags for blocks: `<if v>`, `<else-if v>`, `<else>`, `<for x in v key=x.id>`, `<empty>`.
- Animate block enter and leave in CSS with `[data-entering]` and `[data-leaving]` (`syntax.md`).
- `<c-x>` maps to `lib/components/x/`. `<p-x>` maps to `_components/x/` in the page folder or an ancestor.
- `routes/_layout.html` is the root layout (`<main data-page><slot></slot></main>`). Any subfolder can add `_layout.html` with `<slot></slot>` (no `<head>`, no `<script>`, no `.server.js`) to wrap routes in that folder and its subfolders.

## Rules

- Validate `pragma -V` against `.pragma/version`. Run `pragma upgrade` on mismatch or `app/core-outdated`.
- Use plain JS (ES modules). Do not use TypeScript, JSDoc types, or `console.*` (use `log.*`).
- Add npm packages only when the platform lacks the feature. Prefer plain JS ES modules (`packages.md`).
- Do not put HTML in JS. Do not mutate the DOM; assign a `let`.
- Write strict HTML. Close every non-void tag.
- Compute derived values in functions (`const total = () => …`). Do not write expressions in HTML.
- Read `data` inside functions only; it is undefined until `load` finishes.
- Put stores in `lib/state/`. Access stores only through `shared`.
- Do not write comments.
- Use descriptive names. Do not use vague (`tmp`, `data`, `info`) or short (`t`, `e`) names, except `i`, `j`, `k`, `id`, `ok`, `el`, `url`, `log`, and parameters of one-expression arrows.
- Keep the fixed folder structure. Do not edit `.pragma/` (run `pragma upgrade` to update it).
- Reuse existing components before creating new ones.
- Build overlays only with `c-modal`, `c-alert`, and `c-dropdown`.
- Use a native `<select>` by default. Use `c-select` for a styled list. Do not style `<select>` with `appearance` (`forms.md`).
- Load data with `load` in `.server.js` and render a skeleton.
- Put auth in `routes/_middleware.server.js`. Use `redirect` and `error` from `'pragma/server'`. Use endpoints (`.server.js` without `.html`) only for files, streams, webhooks, and WebSockets (`data.md`).
- Use utilities from `classes.md`. Create a custom class only when a combination repeats 3+ times. Do not use inline `style`.
- Follow accessibility rules: name every control, add `alt` and labels, use one `<h1>`.
- Make every `<form>` a WebMCP tool: set `toolname` (`snake_case` verb_object), `tooldescription`, `name`, and `toolparamdescription` on each field. Add `notool` on login or password forms (`webmcp.md`).
- The rules control how you build. They do not change what the user asked for. If a rule prevents the request, stop and tell the user. Do not deliver a different result that passes the rules.
- Use only features that work in the current versions of Chrome, Edge, Firefox, and Safari.
- If a solution has a limit that the user will see (browser support, keyboard, data loss), tell the user before you write the code.
- Run `pragma check --strict` before you finish. Fix all errors and warnings.
- Deploy to Cloudflare Workers unless the user asks for another target. Follow `cli.md` (Deploy) step by step. Never deploy without a request from the user.

## To do X, read Y

| Task | Read |
|---|---|
| HTML syntax (`{}`, `:`, `.`, `@`, `$`, blocks, enter and leave motion) | `syntax.md` |
| File locations, `pragma.json`, `.pragma/version` | `structure.md` |
| Pages, links, params, query, root and nested layouts | `routes.md` |
| Page title, description, canonical, Open Graph, JSON-LD | `routes.md` (SEO) |
| Components, props, slots | `components.md` |
| `<script>`: state, actions, derived values, `mount`, stores | `state.md` |
| `load`, server actions, skeleton, SSR, `call`/`stream` errors and redirects | `data.md` |
| Cookies, redirects, errors, middleware (auth), endpoints, streaming | `data.md` |
| Modal, alert, dropdown | `overlays.md` |
| Select, styled select (`c-select`), form values | `forms.md` |
| Styles, theme | `css.md`, `classes.md` |
| Forms as agent tools (WebMCP), markdown for agents | `webmcp.md` |
| Performance | `performance.md` |
| Logs | `logs.md` |
| npm packages (install, selection) | `packages.md` |
| Commands, `check` rules, upgrade, self-update | `cli.md` |
| Deploy (Cloudflare Workers), secrets, bindings, domain, CI | `cli.md` (Deploy) |

Copy the doc example when unsure. If no doc covers a case, tell the user. Then use the simplest code that gives the requested result in all target browsers. `pragma check` is the minimum, not the goal.
