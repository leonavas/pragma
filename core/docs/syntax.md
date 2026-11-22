# HTML syntax

Pragma HTML uses a closed set of directives and block tags. Any other syntax is an error.

- `{path}` inserts escaped text.
- `:` binds an attribute or property.
- `.` toggles a class.
- `@` binds an event.
- `$` references a name from the owner `<script>` (state, prop, or function).

| Syntax | Target | Effect |
|---|---|---|
| `{path}` | Text, attribute string | Inserts escaped text (`<p>Hi {$name}</p>`, `alt="Photo of {p.name}"`, `href="/shop/{p.id}"`). |
| `:<attr>="v"` | Element | Sets attribute (`:href`, `:disabled`, `:aria-current`). `false`/`null`/`undefined` removes it; `true` sets empty. `:checked` and `:selected` also set the DOM property. `v` is a path or a literal (see Values). Never `:class`, `:style`, `:on*`. |
| `:--<name>="v"` | Element, component tag | Sets the CSS variable `--name` (`:--left="bar.left"`). Numbers stay numbers; `false`/`null`/`undefined`/`""` removes it. Use it in CSS: `left: calc(var(--left) * 1%)` (`css.md`). |
| `.<class>="[!]v"` | Element | Toggles the class. |
| `:show="[!]v"` | Element | Sets `hidden` when falsy. |
| `:model="$v"` | Input, select, textarea | Two-way binding (`$` or `shared` only). |
| `@<event>="fn"` | Element | Calls `fn(event)` on the owner (`@input="search"`). |
| `@<event>="fn(a, b)"` | Element | Calls `fn` with listed arguments only: paths (`task`, `$x`, `data.x`), `'text'`, numbers, `true`/`false`/`null`, `event`, `el`. |
| `@<event>="shared.cart.add(item)"` | Element | Calls a store function with the same argument rules. |
| `@open="id"` / `@close` | Button | Opens or closes an overlay (`overlays.md`). |

## Text: `{path}`

```html
<p>Hello {$name}, you have {$count} items</p>
<li>{task.title}</li>
<img :src="p.photo" alt="Photo of {p.name}">
<a href="/shop/{p.id}">{p.name}</a>
<label><input type="checkbox" :model="$done"> {task.title}</label>
```

- Use paths only (`$x`, `data.x`, `shared.x.y`, `params.x`, `query.x`, `page.x`, `<for>` variable). No expressions, calls, indexes, or `!`.
- `{$total}` calls derived function `total()` on each render.
- Output is always escaped text, never HTML. `null` and `undefined` render empty.
- Use `{}` in attributes to build strings (`alt="Photo of {p.name}"`). Use `:attr` for whole or boolean values (`:disabled="$atMinimum"`).
- Direct text inside `<code>` or `<pre>` stays literal (`<code>{x}</code>` renders `{x}`). Child elements still interpolate (`<pre><span>{$path}</span></pre>`). Directives, blocks, and components work inside `<code>`/`<pre>`.
- Write `\{` for a literal brace (`pattern="\d\{3}"`) and `&#36;{path}` for a literal `$` before `{path}` (`${` is an error).
- Do not use `{}` in `class` (use `.name="path"`) or in component props (use `:title="$title"`).
- Without `ssr`, prebuilt HTML omits dynamic text until boot. With `ssr`, the server renders it.

## Blocks

| Tag | Effect |
|---|---|
| `<if [!]v>…</if>` | Inserts or removes its content. |
| `<else-if [!]v>…</else-if>` | Follows `</if>` or `</else-if>` (any count). |
| `<else>…</else>` | Follows `</if>` or `</else-if>`. |
| `<for x in v key=x.id>…</for>` | Repeats per item; `key` must be stable and unique. |
| `<empty>…</empty>` | Follows `</for>`; renders when the list is empty. |

Write block paths without quotes: `<if page.loading>`, `<if !$open>`, `<for p in data.products key=p.id>`.

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

## Enter and leave motion

Block roots animate with CSS only. The runtime sets two attributes on the root element of an `<if>`/`<else-if>`/`<else>`/`<for>`/`<empty>` instance:

| Attribute | When |
|---|---|
| `data-entering` | Inserted after the block's first render; removed when its CSS animations end. |
| `data-leaving` | Removed by the block; the element stays (inert) until its CSS animations end, then leaves the DOM. |

```css
@keyframes fade-in { from { opacity: 0; translate: 0 4px } }
@keyframes fade-out { to { opacity: 0 } }
.toast[data-entering] { animation: fade-in .2s ease-out }
.toast[data-leaving] { animation: fade-out .15s ease-in }
```

- No CSS rule, no wait: without an animation or transition the element leaves at once.
- Animations on descendants count (`.modal[data-leaving] .panel { animation: … }`). Infinite animations (spinners, `.skeleton`) are ignored.
- The first render (page load, SSR, navigation, content of a newly inserted block) gets no `data-entering`.
- If the value turns back on while the element is leaving, the same element returns (no new node).
- Under `prefers-reduced-motion: reduce` nothing waits and `data-entering` is not set.
- A leaving `<for>` item stays in place; the other items do not move. Use `position: absolute` in the leave rule if the gap must close at once.

## Values

Use `root.a.b` paths only. Do not use expressions, calls, indexes, or operators. Use `!` only in `<if>`, `<else-if>`, `:show`, and `.class`.

`:attr`, `:--name`, and component props (`:prop`) also take a literal: `true`, `false`, `null`, a number (`2`, `-0.5`), or a quoted string (`'text'`). The build writes it as a fixed value; a prop keeps its type (`:holiday="false"` gives `false`, not the text `"false"`).

```html
<p-hours :holiday="false" :step="15" :label="'Opening hours'"></p-hours>
<input aria-label="Name" :disabled="true" :maxlength="40">
<div class="bar" :--left="0"></div>
```

| Root | Value |
|---|---|
| `$x` | Owner state (`let`), prop (`export let`), constant, or function. |
| `data` | `load` result. |
| `shared` | Stores (`shared.cart.total`). |
| `params`, `query` | Path params and query strings. |
| `page` | `page.loading`, `page.error`. |
| `<for>` variable | Current loop item. |

Compute every derived value in a script or store function (`const total = () => …`). Value positions (`{}`, `:attr`, `.class`, `:show`, `<if>`, `<for>`) call the function with no arguments.

## Example

```html
<form class="flex gap-2" toolname="add_task" tooldescription="Add a task to the to-do list" @submit="add">
  <input name="title" class="flex-1 rounded-lg border border-line px-3 py-2" aria-label="New task" toolparamdescription="Task title, 3 to 80 characters" :model="$draft">
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

- The owner is the page or scripted component whose `.html` defines the element.
- `$`, `@`, and `:model` resolve against the owner. Slot content resolves against its caller.
- Root and nested `_layout.html` files have no owner; they read only `shared`, `params`, and `query`.

## Rules

- Each `<if>`, `<else-if>`, `<else>`, `<for>`, and `<empty>` must contain exactly 1 root element (not another block). Put directives on that element, not on the block tag.
- Write strict HTML and close every non-void tag. Do not use `<html>`, `<body>`, or `<template>`. Put at most one `<script>` and one `<style>` per file (`state.md`, `css.md`).
- `@submit` calls `preventDefault()` automatically. Every `<form>` is an agent tool (`webmcp.md`); read values with `new FormData(event.target)`.
- Pass `<for>` items as arguments (`@click="remove(task)"`). Without parentheses, the handler receives `event` (`@input="search"`). With parentheses, it receives only listed arguments (`@click="reset()"` receives none). Use `el` instead of `event.currentTarget` (events are delegated).
- Omit `$` in event names (`@click="increase"`). `:attr` takes a bare path (`:disabled="$atMinimum"`) or a literal (`:maxlength="40"`); use `title="Count {$count}"` without `:` for interpolated strings.
- Pass one bare path to block tags (`<if $hasTasks>` with `const hasTasks = () => tasks.length > 0`).
- Return `'true'` or `'false'` strings for `aria-*` states (`aria-pressed`, `aria-expanded`), because boolean `false` removes the attribute.
- `:show` hides an element in the DOM; `<if>` adds or removes it. `autofocus` inside a newly inserted block receives focus.
- In dev, missing paths throw (except under `query`).
- Allowed markers: `data-page` (root layout `<main>`), `data-reload` (full-load `<a>`). Never write `data-c`, `data-tid`, `data-of`, `data-slot`, or `data-prop-*`. `data-entering` and `data-leaving` are set by the runtime: use them only in CSS selectors.
- Do not write `data-*` attributes by hand (`html/data-attribute`). Bind `:data-x="path"` for multi-value style hooks (`[data-x=value]`), `.name="path"` for on/off states, and role/label/id for tests.
- `:text` is removed (`html/text-removed`); use `{path}` inside the element.
- Computed sizes, positions, colors, and counts go in CSS variables: `:--x="path"` on the element, `var(--x)` in CSS (`css.md`). Write the name in lowercase kebab-case (`:--bar-left`, `html/css-var`). A value with `;`, `{`, `}`, or `<` is refused.
- Forbidden: inline `style` and `on*`, `:style`, `:class`, `{{ }}`/`${ }` outside direct `<code>`/`<pre>` text (`html/interpolation`), legacy `state.`/`props.`, and foreign directives (`v-if`, `x-show`, `on:click`, `*ngIf`, `[disabled]`: `html/unknown-directive`).
