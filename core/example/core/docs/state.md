# State: script, actions, derived values, stores

The top-level `<script>` of a page or component is its state and behavior. Change a `let`; the
runtime reapplies the HTML (one render per microtask). No signals, no manual render.

| At the top level of the script | It is |
|---|---|
| `let draft = ''` | state, one per instance; `$draft` in HTML |
| `export let step = 1` | a prop (components.md) |
| `function add() {…}`, used as `@submit="add"` | an action |
| `const visible = () => …`, used in `{}`, `<for>`, `:attr` | a derived value, recomputed on each render (in JS call it: `visible()`) |
| `function mount() { …; return cleanup }` | external effects after mount (timer, global listener, observer) |
| `import { shared, data, … } from 'pragma'` | services (below) |

`routes/tasks/index.html`:
```html
<script>
  import { shared, query, log } from 'pragma'

  let draft = ''
  let error = ''

  const visible = () => shared.tasks.items.filter(task => query.filter !== 'pending' || !task.done)

  function add() {
    const title = draft.trim()
    error = title.length < 3 ? 'Use at least 3 characters.' : ''
    if (error) return
    shared.tasks.add(title)
    draft = ''
    log.info('task added')
  }

  function toggle(task) {
    task.done = !task.done
  }
</script>

<form class="flex gap-2" @submit="add">
  <input class="flex-1" aria-label="New task" :model="$draft">
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

A timer just assigns the `let`:
```js
let now = Date.now()

function mount() {
  const timer = setInterval(() => { now = Date.now() }, 1000)
  return () => clearInterval(timer)
}
```

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
- Derived values are functions. Never keep a derived copy in a `let` updated by actions.
- `data` exists only after the load: read it in functions, and put the elements that read it inside the `<else>` of `page.loading`.
- Page-only state: the page script. Shared across pages or persisted: a store. A change used by 2+ pages: a store function.
- Filters, tabs, pagination: in the `query`, read by a derived function.
- `persist` lists `let` names saved to `localStorage`; only for preferences, drafts and local-only data.
- A top-level `const` changed in place (`items.push`) does not update the page (`js/const-state`): use `let`.
- No `var`, no destructuring `let`, no top-level `await` (do it in `mount()` or an action); `export` only declares props (a store exports only `persist`).
- Reach stores only through `shared`; importing a store file is `js/store-import`.
- Never write to the DOM (`textContent`, `classList`, `setAttribute`). `addEventListener` only on `window`/`document`, inside `mount`. Never fetch data in `mount`. No custom pub/sub.
- The old API (`export const state`/`actions`, `ctx.$`, `ctx.update`, `store()`) is an error (`js/legacy-api`).
