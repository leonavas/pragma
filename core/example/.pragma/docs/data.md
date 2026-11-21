# Data: load, server actions, skeleton, SSR

| File | Export | Purpose |
|---|---|---|
| `x.server.js` | `load({ params, query, request, env, log })` | Return value becomes `data` in HTML and page `<script>`. |
| `x.server.js` | `actions.name({ input, params, env, log })` | Invoked via `call('name', input)` from `'pragma'`. |
| `x.server.js` | `ssr = true` | Renders `data` into the first HTML (SEO, link previews, LCP). |
| `_name.server.js` | helpers | Private to the folder; imported only by `.server.js` files. |

`routes/shop/[id].server.js`:
```js
import { findProduct } from './_catalog.server.js'

const likes = new Map()

export async function load({ params }) {
  return { product: findProduct(params.id) }
}

export const actions = {
  async like({ params, log }) {
    const total = (likes.get(params.id) ?? 0) + 1
    likes.set(params.id, total)
    log.info('product liked', { 'product.id': params.id })
    return { likes: total }
  },
}
```

`<script>` of `routes/shop/[id].html`:
```html
<script>
  import { data, params, call, log } from 'pragma'

  let sending = false
  let error = ''

  async function like() {
    sending = true
    try {
      data.product.likes = (await call('like')).likes
    } catch (failure) {
      log.error('product like failed', { 'product.id': params.id, 'exception.message': failure.message })
      error = 'Could not save right now.'
    } finally {
      sending = false
    }
  }
</script>
```

`data` is reactive: assigning `data.product.likes` updates the page.

## Skeleton (page with `load`, without `ssr`)

```html
<h1 class="text-3xl font-semibold">Shop</h1>
<if page.loading>
  <ul aria-busy="true" aria-label="Loading products">
    <li class="skeleton h-48 rounded-2xl"></li>
  </ul>
</if>
<else-if page.error>
  <p role="alert">{page.error.message}</p>
</else-if>
<else>
  <ul>
    <for p in data.products key=p.id>
      <li class="card">{p.name}</li>
    </for>
  </ul>
</else>
```

Keep titles, back links, and filters outside `<if page.loading>`. Match the skeleton shape to the final content.

## SSR behavior

With `ssr = true`, page and component scripts run once per request on the server so initial `let`s, props, and derived values appear in the first HTML. Keep browser-only code in `mount(el)` and actions.

| With `ssr`, in a page or component script | Server behavior |
|---|---|
| Top-level `let`, props, derived functions, slot content | Rendered into the first HTML |
| `data`, `params`, `query`, `page` | Read-only |
| `shared`, `call`, `navigate`, `open`, `close`, `confirm` | Throws; script deferred to browser |
| Top-level `window`, `document`, `localStorage` | Throws; script deferred to browser |
| Imports of npm packages or `.pragma/` | Skipped; script deferred to browser |
| Prop passed from a store | Deferred to browser |
| `mount(el)`, events | Browser only |

When a script is deferred to the browser, its `$x` bindings fill at boot while the rest of the page renders on the server.

## Rules

- Throwing in `load` sets `page.error` and returns HTTP 500. Return `{ product: null }` for not-found states.
- Put every write and every secret-using operation in a server action. Update `data` or a `let` after the call.
- Read `data` inside functions only (`js/top-level-data`). Without `ssr`, `data` is undefined until `load` finishes.
- Load data from `fetch` with secrets from `env`, a database via `env`, or `import products from './_products.json' with { type: 'json' }`.
- Default mode is no SSR: prebuilt HTML holds static content outside `<if>`/`<for>`, and `.pragma/boot.js` renders the skeleton.
- Enable `ssr = true` when crawlers, link previews, or data-bound `<head>`/JSON-LD tags need server data (`routes.md`).
- Do not fetch data in `mount` or client actions. Do not create `/api/...` routes. Do not return secrets from `load`. `env` exists only on the server.
