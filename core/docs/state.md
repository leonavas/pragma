# State: script, actions, derived values, stores

The top-level `<script>` of a page or component defines its state and behavior. Assign a `let` to update HTML (batched once per microtask). Do not use signals or manual renders.

| Top-level declaration | Role |
|---|---|
| `let draft = ''` | Instance state (`$draft` in HTML). |
| `export let step = 1` | Prop (`components.md`). |
| `function add() {…}` used as `@submit="add"` | Action. |
| `const saveLater = debounce(save, 300)` used as `@input="saveLater"` | Action (any top-level `const` holding a function works as a handler). |
| `const visible = () => …` used in `{}`, `<for>`, `:attr` | Derived value, recomputed on each render (call `visible()` in JS). |
| `function mount(el) { …; return cleanup }` | Runs external effects after mount (`el` is the component root or `<main data-page>`). |
| `import { shared, data, … } from 'pragma'` | Runtime services. |

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

## Wrapped handlers

Bind functions returned by helpers (`debounce`, `throttle`, `once`) directly as top-level `const` handlers:

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

If a top-level `const` handler does not hold a function, dev throws when the event fires.

## `mount(el)`

Use `mount(el)` for timers, observers, global listeners, measurements, and focus. Return a cleanup function.

```js
let now = Date.now()

function mount() {
  const timer = setInterval(() => { now = Date.now() }, 1000)
  return () => clearInterval(timer)
}
```

```js
let lines = 0

function mount(el) {
  const observer = new ResizeObserver(() => { lines = Math.round(el.clientHeight / 24) })
  observer.observe(el)
  return () => observer.disconnect()
}
```

- `mount` runs only in the browser and may be `async` (resolved function is the cleanup).
- Read from `el`; do not mutate `el`.
- Declare values used only inside `mount` (timer IDs, observers, DOM nodes) as `const` inside `mount`, not top-level `let`.

## Derived values

Compute derived values in functions: `$total` in HTML, `total()` in JS. Computing from state once in a top-level `let`/`const` triggers `js/stale-derived`. Reading a derived function without `()` in JS triggers `js/derived-call`.

```js
let count = 0
let doubled = count * 2            // js/stale-derived
const doubled = () => count * 2    // valid: `$doubled` in HTML, `doubled()` in JS

function check() {
  if (empty) return                // js/derived-call
  if (empty()) return              // valid
}
```

`let count = start` (from a prop) is valid when an action reassigns `count` later.

For per-item derived values, return row objects from a derived function (`{}` accepts paths only):

```js
const rows = () => items.map(item => ({ item, label: formatPrice(item.price) }))
```
```html
<for row in $rows key=row.item.id>
  <li>{row.item.name}: {row.label} <button @click="remove(row.item)">x</button></li>
</for>
```

Keep the original `item` reference (`row.item`) when mutating it. Spreading `{ ...item, label }` creates a copy that does not update the source.

## Per-frame updates

Assign state from `frame` (scroll, drag, a progress animation) and bind it with `:--name`, `{}`, `:attr`, `:class`, or `:show`. When a change is read only by such bindings (directly or through derived functions), the runtime updates only those bindings, not the page. A value read by `<if>`, `<for>`, `:model`, or a component prop renders the whole page.

`frame(fn)` is `requestAnimationFrame` for all components together: every `frame` callback of the page runs in one animation frame, then the runtime writes all their state changes to the DOM in one pass. It returns a function that cancels the callback.

```js
import { frame } from 'pragma'

let progress = 0

function mount() {
  const start = performance.now()
  let cancel = frame(function step(now) {
    progress = Math.min(1, (now - start) / 520)
    if (progress < 1) cancel = frame(step)
  })
  return () => cancel()
}
```
```html
<div class="send" :--progress="$progress"></div>
```

Scroll and pointer handlers: read nothing in the event, ask for a frame, then read layout and assign state in the callback.

```js
function mount(el) {
  let cancel = null
  const update = () => {
    cancel = null
    const box = el.getBoundingClientRect()
    progress = Math.min(1, Math.max(0, -box.top / box.height))
  }
  const onScroll = () => { cancel ??= frame(update) }
  addEventListener('scroll', onScroll, { passive: true })
  return () => { removeEventListener('scroll', onScroll); cancel?.() }
}
```

- Read layout (`getBoundingClientRect`, `offsetHeight`, `scrollHeight`) before you assign state in a `frame` callback. Assigning state does not touch the DOM until all callbacks of the frame finish, so the reads of the next callbacks stay fast.
- With `requestAnimationFrame`, each callback writes its changes when it returns; a later callback that reads layout forces the browser to compute layout again (about 10x slower with 10 callbacks on a 1000-row page).
- Outside `frame`, the DOM updates in the next microtask after a change (code that waits with `queueMicrotask` sees the new DOM).
- Animate `transform` and `opacity` with the variable, not layout properties (`css.md`, "Values that change every frame").

Derived functions recompute only when state they read changes. Keep values from outside state (`Date.now()`, module variables) in a `let` you assign (`now = Date.now()`).

## Services (`import { … } from 'pragma'`)

| Name | Use |
|---|---|
| `shared` | Stores (`shared.tasks.items`, `shared.tasks.add(title)`). |
| `data` | `load` result (`data.md`); undefined until `load` finishes. |
| `params`, `query`, `page` | Route params, query strings, `page.loading`, `page.error`. |
| `call(name, input)` | Calls a server action; throws `ActionError` (`status`, `body`) on failure (`data.md`). |
| `stream(name, input, { signal })` | Calls a server action that streams NDJSON; `for await` each line (`data.md`). |
| `navigate(url, { replace })` | Navigates via the router; `replace: true` replaces the history entry. |
| `open(id)`, `close(id)`, `confirm(id)` | Controls overlays (`overlays.md`). |
| `frame(fn)` | Runs `fn(now)` in the next animation frame with the other `frame` callbacks, then updates the DOM once; returns a cancel function ("Per-frame updates"). |
| `log` | Emits logs (`logs.md`). |

## Stores

Create stores in `lib/state/<name>.js`. Access them as `shared.<name>` in HTML and scripts.

`lib/state/tasks.js`:
```js
let items = []
export const persist = ['items']

const pending = () => items.filter(task => !task.done).length

function add(title) {
  items.push({ id: crypto.randomUUID(), title, done: false })
}
```

## Rules

- State uses a deep Proxy: `items.push(x)`, `task.done = true`, and reassignment all trigger renders (including from `await`, timers, and promises). Maps, Sets, Dates, and class instances are not tracked; reassign a new value.
- Define derived values as functions and call them in JS (`total()`). Do not copy derived values into a `let` (`js/stale-derived`, `js/derived-call`).
- Read `data` inside functions only (`js/top-level-data`). Wrap elements reading `data` inside the `<else>` of `page.loading`.
- Write `count` in JS, never `$count` (`js/dollar-in-script`).
- Keep page-only state in the page script. Use a store for shared or persisted state, and put actions shared by 2+ pages in the store.
- Store filters, tabs, and pagination in `query`, read by a derived function.
- `persist` saves listed `let` names to `localStorage`; use it only for preferences, drafts, and local-only data.
- Declare mutated top-level state with `let`, not `const` (`js/const-state`).
- Do not use `var`, destructuring `let`, or top-level `await`. Use `export` only for props (or `persist` in stores).
- Access stores only through `shared`; direct store imports trigger `js/store-import`.
- Do not mutate the DOM (`textContent`, `classList`, `setAttribute`). Call `addEventListener` only on `window` or `document` inside `mount`. Do not fetch data in `mount`. Do not build custom pub/sub.
- Do not use the legacy API (`export const state`/`actions`, `ctx.$`, `ctx.update`, `store()`): triggers `js/legacy-api`.
