# State: state, actions, mount, stores

Change state; the runtime reapplies the HTML. No signals, no manual render.

| Export | Purpose |
|---|---|
| `state(ctx)` | initial state, one per instance; getters for derived values |
| `actions` | functions `(ctx)`, sync or async, called by `@event` |
| `mount(ctx)` | external effects only (timer, global listener, observer); returns the cleanup |

```js
export const state = ctx => ({
  draft: '',
  error: '',
  get visible() {
    const pendingOnly = ctx.query.filter === 'pending'
    return ctx.shared.tasks.items.filter(task => !(pendingOnly && task.done))
  },
})

export const actions = {
  add(ctx) {
    const title = ctx.$.draft.trim()
    ctx.$.error = title.length < 3 ? 'Use at least 3 characters.' : ''
    if (ctx.$.error) return
    ctx.shared.tasks.add(title)
    ctx.$.draft = ''
  },
  toggle(ctx) {
    const task = ctx.shared.tasks.find(ctx.scope.task.id)
    task.done = !task.done
  },
}

export function mount(ctx) {
  const timer = setInterval(() => ctx.update(current => { current.now = Date.now() }), 1000)
  return () => clearInterval(timer)
}
```

## `ctx`

| Field | Use |
|---|---|
| `$` | owner state + props (props read-only) |
| `data` `shared` | `load` result, stores |
| `params` `query` `page` | route params, query, `page.loading`/`page.error` |
| `event` `el` `scope` | event, element, `<for>` item (actions only) |
| `update(fn)` | change state outside an action |
| `call(name, input)` | server action (data.md) |
| `navigate(url)` | navigate through the router |
| `open(id)` `close(id)` `confirm(id)` | overlays (overlays.md) |
| `log` | logs (logs.md) |

`ctx.state` and `ctx.props` do not exist (`js/ctx`): use `ctx.$`.

## Store

`lib/state/tasks.js`, read as `shared.tasks` in HTML and `ctx.shared.tasks` in JS:
```js
import { store } from '../../core/state.js'

export default store({
  items: [],
  get pending() { return this.items.filter(task => !task.done).length },
  add(title) { this.items.push({ id: crypto.randomUUID(), title, done: false }) },
  find(id) { return this.items.find(task => task.id === id) },
}, { persist: 'tasks' })
```

## Rules

- Getters run on every render: use `this.x` for own state and `ctx` for the rest. Never store a derived copy from an action.
- `ctx.data` exists only after the load: put elements reading such getters inside the `<else>` of `page.loading`.
- Outside an action (timer, detached promise), change state only through `ctx.update(fn)`.
- Page-only state: `state`. Shared across pages or persisted: store. A mutation used by 2+ pages: a store method.
- Filters, tabs, pagination: in the `query`, read by a getter.
- `persist` (saved to `localStorage`, getters excluded) only for preferences, drafts and local-only data.
- Never write to the DOM (`textContent`, `classList`, `setAttribute`). `addEventListener` only on `window`/`document`, inside `mount`. Never fetch data in `mount`. No custom pub/sub.
