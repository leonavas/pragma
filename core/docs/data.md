# Data: load, server actions, skeleton, SSR

| File | Export | Use |
|---|---|---|
| `x.server.js` | `load({ params, query, request, env, log })` | return value becomes `data` / `ctx.data` |
| `x.server.js` | `actions.name({ input, params, env, log })` | called by `ctx.call('name', input)` |
| `x.server.js` | `ssr = true` | data in the first HTML (SEO, link previews, LCP) |
| `_name.server.js` | helpers | private to the folder; only `.server.js` files import it |

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

`routes/shop/[id].js`:
```js
export const state = () => ({ sending: false, error: '' })

export const actions = {
  async like(ctx) {
    ctx.$.sending = true
    try {
      ctx.data.product.likes = (await ctx.call('like')).likes
    } catch (failure) {
      ctx.log.error('product like failed', { 'product.id': ctx.params.id, 'exception.message': failure.message })
      ctx.$.error = 'Could not save right now.'
    } finally {
      ctx.$.sending = false
    }
  },
}
```

## Skeleton (page with `load`, without `ssr`)

```html
<h1 class="text-3xl font-semibold">Shop</h1>
<if page.loading>
  <ul aria-busy="true" aria-label="Loading products">
    <li class="skeleton h-48 rounded-2xl"></li>
  </ul>
</if>
<else-if page.error>
  <p role="alert" :text="page.error.message"></p>
</else-if>
<else>
  <ul>
    <for p in data.products key=p.id>
      <li class="card" :text="p.name"></li>
    </for>
  </ul>
</else>
```

Keep the title, back link and filters outside the loading chain. Shape the skeleton like the final content.

## Rules

- An error thrown in `load` sets `page.error` and returns 500. "Not found" is data: return `{ product: null }`.
- Every write and everything that uses a secret goes through a server action. Afterwards, update `ctx.data` or `$`.
- Sources: `fetch` with a secret from `env`, a database via `env`, or `import products from './_products.json' with { type: 'json' }`.
- Default is no SSR. With `ssr = true` the HTML is the same, the skeleton is never shown.
- Never fetch data in `mount` or in a client action, never create `/api/...` routes, never return secrets from `load`. `env` exists only on the server.
