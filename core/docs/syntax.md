# HTML syntax

`{path}` puts a value in text, `:` binds a value, `.` binds a class, `@` binds an event, `$` is a
name from the owner's `<script>` (state, prop or function). Conditions and loops are tags. Closed
list: anything else is an error.

| Write | On | Effect |
|---|---|---|
| `{path}` | text, attribute value | the value as text (escaped): `<p>Hi {$name}</p>`, `alt="Photo of {p.name}"`, `href="/shop/{p.id}"` |
| `:<attr>="v"` | element | whole attribute (`:href`, `:disabled`, `:aria-current`); `false`/`null`/`undefined` removes it, `true` sets it empty. `:checked`, `:selected` also set the property. Never `:class`, `:style`, `:on*` |
| `.<class>="[!]v"` | element | toggles the class |
| `:show="[!]v"` | element | `hidden` when falsy |
| `:model="$v"` | input, select, textarea | two-way binding; `$` or `shared` only |
| `@<event>="fn"` | element | calls the owner's function `fn` |
| `@<event>="fn(a, b)"` | element | calls `fn` with arguments: paths (`task`, `$x`, `data.x`), `'text'`, numbers, `true`/`false`/`null`, `event` |
| `@open="id"` / `@close` | button | opens/closes an overlay (overlays.md) |

## Text: `{path}`

```html
<p>Hello {$name}, you have {$count} items</p>
<li>{task.title}</li>
<img :src="p.photo" alt="Photo of {p.name}">
<a href="/shop/{p.id}">{p.name}</a>
<label><input type="checkbox" :model="$done"> {task.title}</label>
```

- Only paths (`$x`, `data.x`, `shared.x.y`, `params.x`, `query.x`, `page.x`, a `<for>` variable). No expressions, calls, indexes or `!`.
- A function from the script is a derived value: `{$total}` calls `total()` on each render.
- Always escaped text, never HTML. `null`/`undefined` is empty.
- In an attribute, `{}` builds a string (`alt="Photo of {p.name}"`); for a whole or boolean value use `:attr` (`:disabled="$atMinimum"`).
- Literal in text whose nearest parent element is `<code>` or `<pre>` (`<code>{x}</code>` shows `{x}`); a child element interpolates (`<pre><span>{$path}</span></pre>`). Directives, blocks and components work everywhere, `<pre>`/`<code>` included (`<pre><code><for line in $lines key=line.id><span>{line.text}</span></for></code></pre>`).
- Elsewhere write `\{` for a literal brace (`pattern="\d\{3}"`) and `&#36;{path}` for a literal `$` right before a value (`${` is an error).
- No `{}` in `class` (use `.name="path"`) or in a component prop (pass a path: `:title="$title"`).
- Without `ssr` the prebuilt HTML shows no value (never raw `{…}`): the runtime fills it at boot. With `ssr` the server renders it.

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
  <p role="alert">{page.error.message}</p>
</else-if>
<else>
  <h1>{data.product.name}</h1>
</else>
```

## Values

Only paths `root.a.b`: no expressions, calls, indexes or operators. `!` only in `<if>`,
`<else-if>`, `:show` and `.class`.

| Root | Value |
|---|---|
| `$x` | the owner's state (`let`), prop (`export let`), constant or function |
| `data` | `load` result |
| `shared` | stores (`shared.cart.total`) |
| `params` `query` | path params and query (strings) |
| `page` | `page.loading`, `page.error` |
| `<for>` variable | current item |

Compute every derived value (total, label, filter, flag) in a function of the script or store
(`const total = () => …`), never in HTML. In a value position (`{}`, `:attr`, `.class`, `:show`,
`<if>`, `<for>`) the function is called with no arguments.

## Example

```html
<form class="flex gap-2" @submit="add">
  <input class="flex-1 rounded-lg border border-line px-3 py-2" aria-label="New task" :model="$draft">
  <button class="btn btn-primary">Add</button>
</form>
<p class="text-sm text-danger-600" role="status" :show="$error">{$error}</p>
<ul>
  <for task in $visible key=task.id>
    <li .line-through="task.done">
      <label><input type="checkbox" :checked="task.done" @change="toggle(task)"> {task.title}</label>
      <button type="button" aria-label="Remove {task.title}" @click="remove(task)">×</button>
    </li>
  </for>
  <empty>
    <li class="text-muted">No tasks.</li>
  </empty>
</ul>
```

## Owner

- The owner is whoever wrote the element: the page, or the component (with a script) whose `.html` contains it.
- `$`, `@` and `:model` resolve against the owner. Slot content (and its `{}`) belongs to whoever wrote it.
- `_layout.html` has no owner: only `shared`, `params` and `query`.

## Rules

- Each `<if>`, `<else-if>`, `<else>`, `<for>`, `<empty>` contains exactly 1 root element, not another block. Put other directives on that element, not on the block tag.
- Strict HTML: close every non-void tag. No `<html>`, `<body>`, `<template>`; one `<script>` and one `<style>` per file (state.md, css.md).
- `@submit` calls `preventDefault()`. Pass the `<for>` item as an argument: `@click="remove(task)"`; `event` is the DOM event (`@input="search(event)"`).
- For `aria-*` states (`aria-pressed`, `aria-expanded`), return the strings `'true'`/`'false'`: boolean `false` removes the attribute.
- `:show` hides an element that stays in the DOM; `<if>` creates/removes it. An element with `autofocus` inside a newly inserted block gets focus.
- In dev, a path that does not exist throws (except under `query`).
- Markers: `data-page` (layout `<main>`), `data-reload` (`<a>` that does a full page load). Never write `data-c`, `data-tid`, `data-of`, `data-slot`, `data-prop-*`: the build generates them.
- Never write `data-*` by hand (`html/data-attribute`): `data-*` is the compiled form of the short syntax (`data-text` is `{path}`, `data-on-click` is `@click`). For a dynamic style hook with many values, bind `:data-x="path"` and select it in CSS with `[data-x=value]`; for on/off, use `.name="path"`; for tests, select by role, label or id.
- `:text` was removed (`html/text-removed`): write `{path}` inside the element.
- Forbidden: inline `style` and `on*`, `:class` (use `.name`), `{{ }}` and `${ }` outside text directly in `<code>`/`<pre>` (`html/interpolation`), the legacy syntax `state.`, `props.`, and attributes of other frameworks (`v-if`, `x-show`, `on:click`, `*ngIf`, `[disabled]`: `html/unknown-directive`).
