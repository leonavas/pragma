# Proposal: single-file components, top-level script, `{}` interpolation

Status: implemented (2026-11-19). The decisions are now in `SPEC.md` (§3, §4, §10, §14) and
`DECISIONS.md` (Reactivity and state; HTML, directives and components), including why the "plain
JS, no compile magic" rule changed. Replaces the `state(ctx)` / `actions` / `ctx.$` API entirely
(old API = `js/legacy-api`). Next: plan steps 3 and 4 (codemod, migrate starter, example + e2e
and `site/`; council review).

## Decisions

1. **`<script>` and `<style>` inside the `.html`** of pages and components.
   - One `<script>` (always an ES module; no `src`, no classic script) and one `<style>` per file.
   - `<style>` is scoped exactly like today's component `.css`.
   - Separate `.js` / `.css` files stay valid; having both for the same component is a
     `check` error (`html/duplicate-module`). `.server.js` stays a separate file.
2. **Top-level script is the component** (no `export default`, no `this`, no `ctx`):
   - top-level `let` = reactive state (the compiler rewrites reads/writes; deep Proxy, so
     `items.push()` works; renders batched in a microtask);
   - top-level function used in `@event` = action;
   - top-level function used in a value binding or `{}` = derived value, called on each render
     (in JS you call it: `total()`); no getters, no `derived()`, no `$:`;
   - `function mount() { …; return cleanup }` runs after mount; no `ctx.update` (assigning a
     `let` from a timer just works);
   - imports are hoisted out of the per-instance factory.
3. **Props, Svelte 4 style:** `export let step = 1`.
   - The default's type drives coercion (`"5"` → 5; bare / `"true"` / `"false"` → boolean).
   - Props are read-only and live. Undeclared prop at a call site → `html/unknown-prop`.
     `export` means "prop" and nothing else.
4. **Services from a virtual module:**
   `import { shared, data, params, query, page, log, call, navigate, open, close, confirm } from 'pragma'`.
5. **Calls with arguments in events:** `@change="toggle(task)"`, `@input="search(event, task)"`.
   Arguments are paths (`$x`, `data.x`, `shared.x`, `params.x`, `<for>` item), simple literals or
   `event`. No expressions.
6. **Stores use the same rule** (`lib/state/x.js`): top-level `let` = state, functions = actions /
   derived, everything exposed as `shared.x.*`; `export const persist = ['items']`. Stores are
   reached only through `shared` (direct `import` of a store is a `check` error).
7. **Text interpolation `{path}`**, the only way to put a value in text:
   - `<p>Hello {$name}, you have {$count} items</p>`, `<li>{task.title}</li>`, `{data.product.name}`;
   - also inside attribute strings: `alt="Photo of {$name}"`, `href="/shop/{product.id}"`;
     `:attr="path"` stays for whole and boolean values (`:disabled="$atMinimum"`);
   - only paths (and derived functions), no expressions; always text-escaped;
   - literal inside `<code>` / `<pre>`; elsewhere escape with `\{`; an invalid path is a `check` error;
   - filled by the server with `ssr`, otherwise by the runtime at boot (invisible markers).
   - `:text` is removed (`check`: "write `{$name}` inside the element").
8. HTML keeps the `$` prefix for state/props (`$count`).

## Example

```html
<script>
  export let start = 0
  export let step = 1

  let count = start

  const atMinimum = () => count - step < 0
  const atStart = () => count === start

  function decrease() { count -= step }
  function increase() { count += step }
  function reset() { count = start }
</script>

<div class="counter">
  <button @click="decrease" :disabled="$atMinimum" aria-label="Decrease">−</button>
  <output>{$count}</output>
  <button @click="increase" aria-label="Increase">+</button>
  <button @click="reset" :disabled="$atStart">Reset</button>
</div>

<style>
  .counter { display: flex; gap: .5rem; align-items: center }
</style>
```

## New `check` rules

| Case | Rule |
|---|---|
| old API (`ctx.$`, `ctx.update`, `export const state` / `actions`) | `js/legacy-api` (with migration hint) |
| `<script>` and `x.js` for the same component | `html/duplicate-module` |
| `<script src>`, classic script, two `<script>` | `html/script` |
| assignment to a prop | `js/prop-write` |
| mutated top-level `const` used in HTML | `js/const-state` ("use let") |
| `{}` / `$x` / `@fn` / `fn(arg)` not declared in the script | `html/unknown-name` |
| undeclared prop at a call site | `html/unknown-prop` |
| `:text` | `html/text-removed` |
| direct import of a store | `js/store-import` |

## Plan

1. Move this into `SPEC.md` / `DECISIONS.md` (review).
2. In parallel: runtime (`page.js`, `state.js`: Proxy, factory, virtual `pragma` module) and
   compiler (`<script>`/`<style>` extraction, top-level rewrite with oxc, `{}` interpolation,
   arguments in events, new rules in `validate` / `pragma-jslint`).
3. Codemod for the old API, then migrate starter, example (+ e2e) and `site/`.
4. Council review of the new DX (docs + site).
