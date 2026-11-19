# AGENTS.md

pragma app: native HTML, CSS and JS, no framework. Docs are in `core/docs/`: read the relevant one before changing code.

## Syntax

`:` binds a value (`:text`, `:href`), `.` binds a class, `@` binds an event, `$` is state or prop.
Blocks are tags: `<if v>`, `<else-if v>`, `<else>`, `<for x in v key=x.id>`, `<empty>`.
`<c-x>` = `lib/components/x/`; `<p-x>` = `_components/x/` in the page's folder.

## Rules

- Plain JS (ES modules). No TypeScript, no JSDoc types, no `console.*` (use `log.*`).
- npm packages only when the platform cannot do it; plain JS (ESM preferred), chosen per `packages.md`.
- No HTML in JS. Never write to the DOM: change state.
- Strict HTML: close every non-void tag.
- Derived values are getters on the state or store, never in HTML.
- Never write comments. The code is the documentation.
- Names say what a thing is or does. No vague (`tmp`, `data`, `info`) or short (`t`, `e`) names,
  except `i`, `j`, `k`, `id`, `ok`, `el`, `ctx`, `url`, `log` and parameters of one-expression arrows.
- Fixed structure; never edit `core/`. Reuse components before creating new ones.
- Overlays only with `c-modal`, `c-alert`, `c-dropdown`.
- Data comes from `load` in `.server.js`, with a skeleton.
- CSS: utilities from `classes.md`; a custom class only when a combination repeats 3+ times. No inline `style`.
- Accessible: a name on every control, `alt`, labels, one `<h1>`.
- Finish with `pragma check --strict`: zero errors, zero warnings.

## To do X, read Y

| Task | Read |
|---|---|
| HTML syntax (`:`, `.`, `@`, `$`, blocks) | `syntax.md` |
| where each file goes, `pragma.json` | `structure.md` |
| pages, links, params, query, layout | `routes.md` |
| page title, description, canonical, Open Graph | `routes.md` (SEO) |
| components, props, slots | `components.md` |
| state, actions, mount, stores, `ctx` | `state.md` |
| load, server actions, skeleton, SSR | `data.md` |
| modal, alert, dropdown | `overlays.md` |
| styles, theme | `css.md`, `classes.md` |
| performance | `performance.md` |
| logs | `logs.md` |
| npm packages (install, which to use) | `packages.md` |
| commands, `check` rules, deploy | `cli.md` |

If unsure, copy the doc's example. If no doc covers it, do the simplest thing that passes `check`.
