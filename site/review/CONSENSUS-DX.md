# Council round 3: DX of the new component API (astra · fable · opus)

Sources: `astra-dx.md`, `fable-dx.md`, `opus-dx.md`. Decisions by @claude-109.

## Agreed by all 3
1. **`data` read at the top level of a script passes check** (`let products = data.products`; TypeError or stale value at runtime). → `check` error `js/top-level-data` with the fix ("read it inside a function: `const products = () => data.products`"). Owner: sfc-script.
2. **Error messages:** misleading texts, typo hint suggests creating the wrong prop, errors masked by earlier ones, false "pragma only for page scripts" cascade. → report every script error in one pass, "did you mean `x`?" hints (Levenshtein over declared names/props), fix the five messages listed in `fable-dx.md`. Owners: sfc-script (script), sfc-template (template).
3. **AGENTS.md / docs stale:** `site/AGENTS.md` teaches the old API; starter/example AGENTS.md say `mount()`; docs miss "call derived values in JS", object-prop mutability, `.map({...x})` copies. Owners: mig-example (starter, example), polish-a (site/AGENTS.md), sfc-script (core/docs).

## Agreed by 2
4. **Stale derived values:** `let doubled = count * 2` used in HTML (computed once), and a derived function used without `()` in JS (`if (empty)`). → `js/stale-derived` (a top-level `let` initialised from another state `let`/prop and never reassigned → "make it a function") and `js/derived-call` (a top-level function referenced as a value in JS without calling it). Owner: sfc-script.
5. **`$x` inside a script** (ReferenceError). → `js/dollar-in-script` ("in <script> write `x`, `$x` is HTML only"). Owner: sfc-script.

## Single reviewer, accepted
6. **`<style>` is not scoped** (fable, P0): `button { color: red }` in a component ships as a global rule. → component `<style>` and `.css` are auto-scoped: every selector is prefixed with the component scope unless it already starts with it; `:global(...)` escapes. Pages: scoped to the page root. Owner: sfc-template.
7. **Event arity** (fable): `@input="search"` with `function search(event)` gets no args. → with no parentheses, the handler receives `event`; explicit args replace it. New allowed arg name `el` (the element with the directive), since `event.currentTarget` is the delegation root (opus). Owner: sfc-template.
8. **Store functions as handlers** (opus): `@click="shared.cart.remove(item.id)"`. → allowed. Owner: sfc-template.
9. **`async mount()` cleanup fails** (astra, P1). → if `mount` returns a promise, await it and use the resolved function as cleanup (also when unmounted before it resolves). Owner: sfc-script.
10. **Object props mutability** (astra). → props are read-only, deeply: dev runtime wraps prop objects in a read-only proxy that throws a clear error on write; docs say so. Owner: sfc-script.

## After core
- example/starter and site: drop store-action wrappers (cart-summary has 4), use `@input="search"` with event, remove manual `[data-c="x"]` prefixes only where it simplifies (auto-scope keeps them valid). Owners: mig-example (example/starter), polish-a/b/c (site, their scopes).
