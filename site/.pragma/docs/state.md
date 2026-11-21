# State: script, actions, derived values, stores

The top-level `<script>` of a page or component is its state and behavior. Change a `let`; the
runtime reapplies the HTML (one render per microtask). No signals, no manual render.

| At the top level of the script | It is |
|---|---|
| `let draft = ''` | state, one per instance; `$draft` in HTML |
| `export let step = 1` | a prop (components.md) |
| `function add() {…}`, used as `@submit="add"` | an action |
| `const saveLater = debounce(save, 300)`, used as `@input="saveLater"` | an action too: any top-level `const` can be a handler; the runtime calls the function it holds |
| `const visible = () => …`, used in `{}`, `<for>`, `:attr` | a derived value, recomputed on each render (in JS call it: `visible()`) |
| `function mount(el) { …; return cleanup }` | external effects after mount (timer, global listener, observer); `el` is the component root (a page: its `<main data-page>`) |
| `import { shared, data, … } from 'pragma'` | services (below) |

`routes/tasks/index.html`:
```html
<script>
  import { shared, query, log } from 'pragma'

  let draft = ''
  let error = ''

  const visible = () => shared.tasks.items.filter(task => query.filter !== 'pending' || !task.done)

  function add(event) {
    const title = String(new FormData(event.target).get('title') ?? '').trim()
    error = title.length < 3 ? 'Use at least 3 characters.' : ''
    if (error) {
      if (event.agentInvoked) event.respondWith(Promise.resolve({ error }))
      return
    }
    shared.tasks.add(title)
    draft = ''
    log.info('task added')
    if (event.agentInvoked) event.respondWith(Promise.resolve({ ok: true, title }))
  }

  function toggle(task) {
    task.done = !task.done
  }
</script>

<form class="flex gap-2" toolname="add_task" tooldescription="Add a task to the to-do list" @submit="add">
  <input name="title" class="flex-1" aria-label="New task" toolparamdescription="Task title, at least 3 characters" :model="$draft">
  <button class="btn btn-primary">Add</button>
</form>
<p role="alert" :show="$error">{$error}</p>
<ul>
  <for task in $visible key=task.id>
    <li>
      <input type="checkbox" :checked="task.done" :aria-label="task.title" @change="toggle(task)">
      <span>{task.title}</span>
    </li>
  </for>
</ul>
```

A function made by a call (debounce, throttle, once) is a top-level `const`, used directly as the
handler; no wrapper function:
```html
<script>
  import { debounce } from 'es-toolkit/function'

  let draft = ''
  let saved = ''

  const saveLater = debounce(() => { saved = draft }, 300)
</script>

<input aria-label="Draft" :model="$draft" @input="saveLater">
<p>Saved: {$saved}</p>
```
The build accepts any top-level `const` as a handler; if its value is not a function, the event
throws in dev (`@input="saveLater" needs a function, but "saveLater" holds …`).

A timer just assigns the `let`:
```js
let now = Date.now()

function mount() {
  const timer = setInterval(() => { now = Date.now() }, 1000)
  return () => clearInterval(timer)
}
```

`mount(el)` gets the component's root element, for what needs its own DOM (an observer, a
measurement, focus):
```js
let lines = 0

function mount(el) {
  const observer = new ResizeObserver(() => { lines = Math.round(el.clientHeight / 24) })
  observer.observe(el)
  return () => observer.disconnect()
}
```
`mount` runs only in the browser; read from `el`, never write to it (change state instead). It
may be `async`: the function it resolves to is the cleanup.

What only `mount` needs (a timer id, an observer, a DOM node) is a `const` inside `mount`, not a
top-level `let`: a top-level `let` is state, renders on every change and keeps a DOM node in the
instance scope.

## Derived values

A value computed from state is a function, called on each render: `$total` in HTML, `total()` in
JS. A top-level `let` or `const` computed from state once is stale (`js/stale-derived`), and a
function used in JS without `()` is always truthy (`js/derived-call`):
```js
let count = 0
let doubled = count * 2            // js/stale-derived: computed once
const doubled = () => count * 2    // right: `$doubled` in HTML, `doubled()` in JS

function check() {
  if (empty) return                // js/derived-call: always true
  if (empty()) return              // right
}
```
`let count = start` (a prop) is fine when an action assigns `count` later: it is state that starts
from the prop.

A value per item is a field of the rows a derived function returns (`{}` takes paths, not calls):
```js
const rows = () => items.map(item => ({ item, label: formatPrice(item.price) }))
```
```html
<for row in $rows key=row.item.id>
  <li>{row.item.name}: {row.label} <button @click="remove(row.item)">x</button></li>
</for>
```
Keep the original item in the row (`row.item`) and change that one: `{ ...item, label }` is a copy,
so `item.done = true` on it changes nothing.

## Services (`import { … } from 'pragma'`)

| Name | Use |
|---|---|
| `shared` | stores: `shared.tasks.items`, `shared.tasks.add(title)` |
| `data` | `load` result (data.md); undefined until the load ends |
| `params` `query` `page` | route params, query, `page.loading`/`page.error` |
| `call(name, input)` | server action (data.md) |
| `navigate(url)` | navigate through the router |
| `open(id)` `close(id)` `confirm(id)` | overlays (overlays.md) |
| `log` | logs (logs.md) |

## Store

`lib/state/tasks.js`, read as `shared.tasks` in HTML and in scripts:
```js
let items = []
export const persist = ['items']

const pending = () => items.filter(task => !task.done).length

function add(title) {
  items.push({ id: crypto.randomUUID(), title, done: false })
}
```

## Rules

- State is a deep Proxy: `items.push(x)`, `task.done = true` and reassignment all render, from actions, `await`, timers or promises. Maps, Sets, Dates and class instances are not tracked: assign a new value.
- Derived values are functions; call them in JS (`total()`). Never keep a derived copy in a `let` updated by actions (`js/stale-derived`, `js/derived-call`).
- `data` exists only after the load: read it in functions (`js/top-level-data` at the top level), and put the elements that read it inside the `<else>` of `page.loading`.
- In a script write `count`, never `$count` (`js/dollar-in-script`): `$` is HTML only.
- Page-only state: the page script. Shared across pages or persisted: a store. A change used by 2+ pages: a store function.
- Filters, tabs, pagination: in the `query`, read by a derived function.
- `persist` lists `let` names saved to `localStorage`; only for preferences, drafts and local-only data.
- A top-level `const` changed in place (`items.push`) does not update the page (`js/const-state`): use `let`.
- No `var`, no destructuring `let`, no top-level `await` (do it in `mount()` or an action); `export` only declares props (a store exports only `persist`).
- Reach stores only through `shared`; importing a store file is `js/store-import`.
- Never write to the DOM (`textContent`, `classList`, `setAttribute`). `addEventListener` only on `window`/`document`, inside `mount`. Never fetch data in `mount`. No custom pub/sub.
- The old API (`export const state`/`actions`, `ctx.$`, `ctx.update`, `store()`) is an error (`js/legacy-api`).
