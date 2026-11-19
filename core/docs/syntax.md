# HTML syntax

`:` binds a value, `.` binds a class, `@` binds an event, `$` is owner state or prop. Conditions
and loops are tags. Closed list: anything else is an error.

| Write | On | Effect |
|---|---|---|
| `:text="v"` | element | text content (escaped) |
| `:<attr>="v"` | element | attribute (`:href`, `:disabled`, `:aria-current`); `false`/`null`/`undefined` removes it, `true` sets it empty. `:checked`, `:selected` also set the property. Never `:class`, `:style`, `:on*` |
| `.<class>="[!]v"` | element | toggles the class |
| `:show="[!]v"` | element | `hidden` when falsy |
| `:model="$v"` | input, select, textarea | two-way binding; `$` or `shared` only |
| `@<event>="name"` | element | calls the owner's `actions.name` |
| `@open="id"` / `@close` | button | opens/closes an overlay (overlays.md) |

## Blocks

| Write | Effect |
|---|---|
| `<if [!]v>…</if>` | creates/removes its content |
| `<else-if [!]v>…</else-if>` | right after `</if>` or `</else-if>`; any number |
| `<else>…</else>` | right after `</if>` or `</else-if>` |
| `<for x in v key=x.id>…</for>` | repeats; `key` required, stable and unique |
| `<empty>…</empty>` | right after `</for>`; shown when the list is empty |

Paths in block tags are unquoted: `<if page.loading>`, `<if !$open>`, `<for p in data.products key=p.id>`.

```html
<if page.loading>
  <p class="skeleton h-8"></p>
</if>
<else-if page.error>
  <p role="alert" :text="page.error.message"></p>
</else-if>
<else>
  <h1 :text="data.product.name"></h1>
</else>
```

## Values

Only paths `root.a.b`: no expressions, calls, indexes or operators. `!` only in `<if>`,
`<else-if>`, `:show` and `.class`.

| Root | Value |
|---|---|
| `$x` | owner state or prop |
| `data` | `load` result |
| `shared` | stores (`shared.cart.total`) |
| `params` `query` | path params and query (strings) |
| `page` | `page.loading`, `page.error` |
| `<for>` variable | current item |

Compute every derived value (total, label, filter, flag) in a getter on the state or store, never in HTML.

## Example

```html
<form class="flex gap-2" @submit="add">
  <input class="flex-1 rounded-lg border border-line px-3 py-2" aria-label="New task" :model="$draft">
  <button class="btn btn-primary">Add</button>
</form>
<p class="text-sm text-danger-600" role="status" :show="$error" :text="$error"></p>
<ul>
  <for task in $visible key=task.id>
    <li .line-through="task.done">
      <input type="checkbox" :checked="task.done" :aria-label="task.title" @change="toggle">
      <span :text="task.title"></span>
    </li>
  </for>
  <empty>
    <li class="text-muted">No tasks.</li>
  </empty>
</ul>
```

## Owner

- The owner is whoever wrote the element: the page, or the component (with a `.js`) whose `.html` contains it.
- `$`, `@` and `:model` resolve against the owner. Slot content belongs to whoever wrote it.
- `_layout.html` has no owner: only `shared`, `params` and `query`.

## Rules

- Each `<if>`, `<else-if>`, `<else>`, `<for>`, `<empty>` contains exactly 1 root element, not another block. Put other directives on that element, not on the block tag.
- Strict HTML: close every non-void tag. No `<html>`, `<body>`, `<script>`, `<style>`, `<template>`.
- `@submit` calls `preventDefault()`. In the action, the `<for>` item is `ctx.scope.<var>` (`ctx.scope.task`).
- For `aria-*` states (`aria-pressed`, `aria-expanded`), return the strings `'true'`/`'false'`: boolean `false` removes the attribute.
- `:show` hides an element that stays in the DOM; `<if>` creates/removes it. An element with `autofocus` inside a newly inserted block gets focus.
- In dev, a path that does not exist throws (except under `query`).
- Markers: `data-page` (layout `<main>`), `data-reload` (`<a>` that does a full page load). Never write `data-c`, `data-tid`, `data-of`, `data-slot`: the build generates them.
- Never write `data-*` by hand (`html/data-attribute`): `data-*` is the compiled form of the short syntax (`data-text` is `:text`, `data-on-click` is `@click`). For a dynamic style hook with many values, bind `:data-x="path"` and select it in CSS with `[data-x=value]`; for on/off, use `.name="path"`; for tests, select by role, label or id.
- Forbidden: inline `style` and `on*`, `:class` (use `.name`), `{{ }}` and `${ }` outside `<code>`/`<pre>` (`html/interpolation`), and the legacy syntax `state.`, `props.`.
