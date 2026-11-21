# Components

Search `lib/components/` and `_components/` before you create a component.

| Tag | Folder | Use when |
|---|---|---|
| `<c-name>` | `lib/components/name/` | Shared across multiple route folders. |
| `<p-name>` | `_components/name/` in the page folder or an ancestor | Used in one route folder only. |

To promote a `p-` component, move the folder to `lib/components/` and rename `p-` to `c-`.

```
name/
  name.html   required: optional <script> and <style> at top level, then 1 root element
              (not <template>, <slot>, or another component)
  name.js     optional: replaces <script>
  name.css    optional: replaces <style>; scoped automatically
```

- `<script>` defines state and behavior (`state.md`). `<style>` or `name.css` defines CSS.
- The build scopes every selector to the component root and its descendants. `button { … }` never matches outside.
- `:global(…)` opts out only its argument (`:global(html[data-theme="dark"]) .x` targets `.x` in dark mode; `:global(body.locked)` is global; see `css.md`).
- Selectors starting with `[data-c="name"]` stay as written.
- Using both inline `<script>`/`<style>` and external `name.js`/`name.css` triggers `html/duplicate-module`.
- `p-` names and styles are private to their folder. Two `<p-badge>` components in different folders get distinct `data-c` values and never share CSS.
- Do not select a `p-` component's `data-c` outside its `.css`. Select by class, role, or label.

## Props

Declare props in `<script>` with `export let name = default`:

| Caller writes | Script declares | Inside (`$x` in HTML, `x` in JS) |
|---|---|---|
| `label="Hi"` | `export let label = ''` | `'Hi'` |
| `start="10"` | `export let start = 0` | `10` (converted to the default's type) |
| `locked` / `locked="false"` | `export let locked = false` | `true` / `false` |
| `:product="p"` | `export let product = null` | The object itself, updated on each render |
| omitted | `export let step = 1` | `1` |

- Props are read-only (`js/prop-write`, including via `:model`). Copy a prop into a `let` to change it (`let count = start`).
- Object and array props are deeply read-only. In dev, mutating them throws `"props are read-only, also inside"`. Copy before changing (`let rows = files.map(file => ({ ...file }))`), or update through a store function or event.
- Passing an undeclared prop triggers `html/unknown-prop`.
- `class` is not a prop; it appends to the root element's classes.
- `id` is a normal prop (`export let id = ''`, then `:id="$id"`).
- Name props as one lowercase word (`label`, `step`); HTML attributes are case-insensitive.
- Components without a script use build-time props: `$label` replaces with caller text, `:x="path"` with caller path. Missing, `""`, and `"false"` are falsy.

## Example

`lib/components/counter/counter.html`:
```html
<script>
  export let label = ''
  export let start = 0
  export let step = 1

  let count = start

  const atMinimum = () => count - step < 0

  function decrease() { count -= step }
  function increase() { count += step }
</script>

<div class="inline-flex items-center gap-2" role="group" :aria-label="$label">
  <span class="text-sm text-muted">{$label}</span>
  <button type="button" class="btn" aria-label="Decrease" @click="decrease" :disabled="$atMinimum">−</button>
  <output class="w-8 text-center tabular-nums">{$count}</output>
  <button type="button" class="btn" aria-label="Increase" @click="increase">+</button>
</div>
```

Usage:
```html
<c-counter label="Steps of 5" start="10" step="5"></c-counter>
<for p in data.products key=p.id>
  <p-product-card :product="p"></p-product-card>
</for>
```

## Slots

```html
<c-modal id="about" title="About">
  <p>The body goes in the default slot.</p>
  <button slot="actions" type="button" class="btn" @close>Close</button>
</c-modal>
```

- `<slot></slot>` receives default children. `<slot name="actions">` receives `slot="actions"`.
- Filling a slot that does not exist is an error.
- Slot content belongs to its caller; `$` and `@` resolve in the caller's scope.

## Rules

- In a scriptless component, `@` calls the caller's function.
- State is per instance. A component script loads only on pages that use it.
- Do not put `.server.js` in components; load data in the page's `load`.
- Do not read or mutate a child's state. Pass props; let the child call its own action or a store function.
- Do not use `customElements.define` or shadow DOM. Build overlays with `c-modal`, `c-alert`, and `c-dropdown` (`overlays.md`). Use `c-select` for a styled select (`forms.md`).
