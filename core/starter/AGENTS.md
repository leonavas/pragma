# AGENTS.md

pragma app: native HTML, CSS and JS, no framework. Docs are in `core/docs/`: read the relevant one before changing code.

## Syntax

A page or component is one `.html`: an optional `<script>` and `<style>`, then the markup.
In the `<script>`, a top-level `let` is state, a function is an action (`@click="add"`) or a
derived value (`{$total}` in HTML, `total()` in JS), `export let step = 1` is a prop and
`function mount(el) { …; return cleanup }` runs after mount. Services:
`import { shared, data, params, query, page, call, navigate, open, close, confirm, log } from 'pragma'`.
`<style>` is scoped to its component or page (`:global(…)` opts out).

`{path}` puts a value in text and attribute strings (`<p>Hi {$name}</p>`, `href="/shop/{p.id}"`),
`:` binds a whole value (`:href`, `:disabled`), `.` binds a class, `@` binds an event, `$` is a
name from the `<script>` (HTML only: in JS write `name`).
`@input="search"` passes the event; `@change="toggle(task)"` passes only what it lists (paths,
literals, `event`, `el`); store functions work directly: `@click="shared.cart.remove(item.id)"`.
Blocks are tags: `<if v>`, `<else-if v>`, `<else>`, `<for x in v key=x.id>`, `<empty>`.
`<c-x>` = `lib/components/x/`; `<p-x>` = `_components/x/` in the page's folder.

## Rules

- Plain JS (ES modules). No TypeScript, no JSDoc types, no `console.*` (use `log.*`).
- npm packages only when the platform cannot do it; plain JS (ESM preferred), chosen per `packages.md`.
- No HTML in JS. Never write to the DOM: assign a `let`.
- Strict HTML: close every non-void tag.
- Derived values are functions in the script or store (`const total = () => …`), never expressions in HTML.
- Read `data` inside functions only; it is undefined until the load ends.
- Stores live in `lib/state/` and are reached only through `shared`.
- Never write comments. The code is the documentation.
- Names say what a thing is or does. No vague (`tmp`, `data`, `info`) or short (`t`, `e`) names,
  except `i`, `j`, `k`, `id`, `ok`, `el`, `url`, `log` and parameters of one-expression arrows.
- Fixed structure; never edit `core/` (`pragma upgrade` updates it). Reuse components before creating new ones.
- Overlays only with `c-modal`, `c-alert`, `c-dropdown`.
- Data comes from `load` in `.server.js`, with a skeleton.
- CSS: utilities from `classes.md`; a custom class only when a combination repeats 3+ times. No inline `style`.
- Accessible: a name on every control, `alt`, labels, one `<h1>`.
- Finish with `pragma check --strict`: zero errors, zero warnings.

## To do X, read Y

| Task | Read |
|---|---|
| HTML syntax (`{}`, `:`, `.`, `@`, `$`, blocks) | `syntax.md` |
| where each file goes, `pragma.json` | `structure.md` |
| pages, links, params, query, layout | `routes.md` |
| page title, description, canonical, Open Graph | `routes.md` (SEO) |
| components, props, slots | `components.md` |
| `<script>`: state, actions, derived values, `mount`, stores | `state.md` |
| load, server actions, skeleton, SSR | `data.md` |
| modal, alert, dropdown | `overlays.md` |
| styles, theme | `css.md`, `classes.md` |
| performance | `performance.md` |
| logs | `logs.md` |
| npm packages (install, which to use) | `packages.md` |
| commands, `check` rules, deploy | `cli.md` |

If unsure, copy the doc's example. If no doc covers it, do the simplest thing that passes `check`.
