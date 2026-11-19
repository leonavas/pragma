# Components

Search `lib/components/` and `_components/` before creating one.

| Tag | Folder | Use when |
|---|---|---|
| `<c-name>` | `lib/components/name/` | used by more than one route folder |
| `<p-name>` | `_components/name/` in the page's folder or an ancestor | used by one route folder only |

To promote: move the folder to `lib/components/` and rename `p-` to `c-`.

```
name/
  name.html   required; 1 root element (not <template>, <slot> or a component)
  name.js     optional: state, actions, mount
  name.css    optional; scope selectors with [data-c="name"]
```

## Props

| Caller writes | Inside the component |
|---|---|
| `label="Hi"` (static) | `$label` in HTML, `ctx.$.label` in JS |
| `:product="p"` (dynamic, updates on each render) | `$product` in HTML; in JS `ctx.$.product` is the attribute string |
| `locked` (bare) / `locked="false"` | `<if $locked>` true / false; in JS `ctx.$.locked` is `true` / `false` |

- Props are read-only. A missing prop, `""` and `"false"` are falsy; a bare attribute and any other value are truthy, in HTML and JS alike.
- Name props in one lowercase word (`label`, `step`): HTML attributes are case-insensitive.
- Never give a state key or getter the same name as a prop: the state key hides the prop, and a getter reading `ctx.$.<same name>` recurses forever.
- In JS every prop is a string, except a bare attribute or `"true"` (`true`) and `"false"` (`false`). Use objects only in HTML; to use one in JS, pass an id and look it up in `ctx.data` or a store.
- A component with a `.js` does not accept the caller's `$`: pass `data`, `shared` or a `<for>` item, or use a store.
- `class` is appended to the root's classes. `id` is a prop: apply it with `:id="$id"`.

## Example

`lib/components/counter/counter.html`:
```html
<div class="inline-flex items-center gap-2" role="group" :aria-label="$label">
  <span class="text-sm text-muted" :text="$label"></span>
  <button type="button" class="btn" aria-label="Decrease" @click="decrease" :disabled="$atMinimum">−</button>
  <output class="w-8 text-center tabular-nums" :text="$count"></output>
  <button type="button" class="btn" aria-label="Increase" @click="increase">+</button>
</div>
```

`counter.js`:
```js
export const state = ctx => ({
  count: Number(ctx.$.start || 0),
  get increment() { return Number(ctx.$.step || 1) },
  get atMinimum() { return this.count - this.increment < 0 },
})

export const actions = {
  decrease(ctx) { ctx.$.count -= ctx.$.increment },
  increase(ctx) { ctx.$.count += ctx.$.increment },
}
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

- In a component without `.js`, `@` calls the caller's action.
- A page cannot use two components with the same folder name (`<c-card>` and `<p-card>`).
- State is per instance. A component's `.js` loads only on pages that use it.
- No `.server.js` in components: data comes from the page's `load`.
- Never read or change a child's state: pass props; the child changes a store or calls its own action.
- No `customElements.define`, no shadow DOM. Overlays: use `c-modal`, `c-alert`, `c-dropdown` (overlays.md).
