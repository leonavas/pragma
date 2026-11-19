# Data: load, server actions, skeleton, SSR

| File | Export | Use |
|---|---|---|
| `x.server.js` | `load({ params, query, request, env, log })` | return value becomes `data` (HTML, and `data` from `pragma` in the page script) |
| `x.server.js` | `actions.name({ input, params, env, log })` | called by `call('name', input)` from `pragma` |
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

The `<script>` of `routes/shop/[id].html`:
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

`data` is live and reactive: changing `data.product.likes` updates the page.

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

Keep the title, back link and filters outside the loading chain. Shape the skeleton like the final content.

## Rules

- An error thrown in `load` sets `page.error` and returns 500. "Not found" is data: return `{ product: null }`.
- Every write and everything that uses a secret goes through a server action. Afterwards, update `data` or a `let`.
- `data` is undefined until the load ends (unless `ssr`): read it inside functions, never at the top level of the script.
- With `ssr`, page and component scripts also run on the server, once per request, so their initial `let`s, props and derived values are in the first HTML (see the table below). Keep browser-only work in `mount(el)` and actions.

| With `ssr`, in a page or component script | On the server |
|---|---|
| top-level `let`, props, derived functions, slot content | rendered into the first HTML |
| `data`, `params`, `query`, `page` | read-only |
| `shared`, `call`, `navigate`, `open`, `close`, `confirm` | throw: that script is left to the browser |
| reading `window`, `document`, `localStorage` at the top level | throws: left to the browser |
| a script that imports an npm package or `core/` | not run: left to the browser |
| a prop that comes from a store | left to the browser |
| `mount(el)`, events | never (browser only) |

"Left to the browser" means its `$x` bindings are filled at boot, as without `ssr`; the rest of the page still renders on the server.
- Sources: `fetch` with a secret from `env`, a database via `env`, or `import products from './_products.json' with { type: 'json' }`.
- Default is no SSR. The prebuilt HTML has the content outside `<if>`/`<for>`; the `page.loading` branch is an inert `<template>` and the skeleton appears as soon as `core/boot.js` runs (no skeleton without JS).
- With `ssr = true` the HTML is the same, rendered with data; the skeleton is never shown. Use it for pages that crawlers or link previews must see, and for head tags or JSON-LD structured data bound to `data` (routes.md, SEO).
- Never fetch data in `mount` or in a client action, never create `/api/...` routes, never return secrets from `load`. `env` exists only on the server.
