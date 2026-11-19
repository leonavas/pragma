# Components

Search `lib/components/` and `_components/` before creating one.

| Tag | Folder | Use when |
|---|---|---|
| `<c-name>` | `lib/components/name/` | used by more than one route folder |
| `<p-name>` | `_components/name/` in the page's folder or an ancestor | used by one route folder only |

To promote: move the folder to `lib/components/` and rename `p-` to `c-`.

```
name/
  name.html   required: optional <script> and <style> at the top level, then 1 root element
              (not <template>, <slot> or a component)
  name.js     optional, instead of the <script>
  name.css    optional, instead of the <style>; scope selectors with [data-c="name"]
```

The `<script>` is the component's state and behavior (state.md); `<style>` (or `name.css`) is
its CSS, scoped with `[data-c="name"]`. Both inline and as a file = `html/duplicate-module`.

`p-` names are private to their folder, styles included: two `<p-badge>` in different folders
never share CSS (the build gives each its own `data-c` and rewrites `[data-c="badge"]` in its own
`.css`). Do not select a `p-` component's `data-c` from outside its `.css` (theme, tests): use a
class, role or label.

## Props

A component with a script declares its props with `export let name = default`:

| Caller writes | Script declares | Inside (`$x` in HTML, `x` in JS) |
|---|---|---|
| `label="Hi"` | `export let label = ''` | `'Hi'` |
| `start="10"` | `export let start = 0` | the number `10`: the default's type converts the text |
| `locked` / `locked="false"` | `export let locked = false` | `true` / `false` |
| `:product="p"` | `export let product = null` | the object itself, updated on each render |
| nothing | `export let step = 1` | the default, `1` |

- Props are read-only (`js/prop-write`, also via `:model`): copy one into a `let` to change it (`let count = start`).
- A prop the script does not declare is an error at the call site (`html/unknown-prop`). `class` is not a prop: it is appended to the root's classes. `id` is a prop like any other (`export let id = ''`, then `:id="$id"`).
- Name props in one lowercase word (`label`, `step`): HTML attributes are case-insensitive.
- A component **without** a script has build-time props: `$label` is replaced by the caller's text, `:x="path"` by the caller's path; missing, `""` and `"false"` are falsy.

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

- `<slot></slot>` receives the children; `<slot name="actions">` receives `slot="actions"`. Filling a slot the component lacks = error.
- Slot content belongs to whoever wrote it: its `$` and `@` resolve there.

## Rules

- In a component without a script, `@` calls the caller's function.
- State is per instance. A component's script loads only on pages that use it.
- No `.server.js` in components: data comes from the page's `load`.
- Never read or change a child's state: pass props; the child changes a store or calls its own action.
- No `customElements.define`, no shadow DOM. Overlays: use `c-modal`, `c-alert`, `c-dropdown` (overlays.md).
