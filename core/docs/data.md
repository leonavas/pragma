# Data: load, server actions, skeleton, SSR, middleware, endpoints

| File | Export | Purpose |
|---|---|---|
| `x.server.js` | `load(context)` | Return value becomes `data` in HTML and page `<script>`. |
| `x.server.js` | `actions.name({ input, ...context })` | Invoked via `call('name', input)` from `'pragma'`. |
| `x.server.js` | `ssr = true` | Renders `data` into the first HTML (SEO, link previews, LCP). |
| `x.server.js` without `x.html` | `GET`, `POST`, `PUT`, `PATCH`, `DELETE` | Endpoint: returns a `Response` (files, streams, webhooks, WebSockets). |
| `routes/_middleware.server.js` | `handle({ ...context, next })` | Runs before every request. |
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

## Server context

`load`, actions, endpoints, and middleware receive one context object:

| Key | Value |
|---|---|
| `request` | The `Request`. |
| `url` | Page URL as a `URL`. For `/_data` and `/_action` requests it is the page path, not the internal path. |
| `params`, `query` | Route params and query, as strings. |
| `env`, `db`, `kv`, `files` | Bindings and secrets (`cli.md`, Deploy). |
| `log` | Server logger (`logs.md`). |
| `cookies` | `get(name)`, `getAll()`, `set(name, value, options)`, `delete(name, options)`. |
| `locals` | Object for the request. Middleware fills it (`locals.user`); `load` and actions read it. |
| `setHeaders({ name: value })` | Adds headers to the response. Use `cookies.set` for cookies. |
| `waitUntil(promise)` | Work after the response (email, audit). Workers: `ctx.waitUntil`. Node: runs in the background and logs a rejection. |

`cookies.set` defaults: `path: '/'`, `httpOnly: true`, `sameSite: 'lax'`, `secure: true` (not on plain `http://localhost`). Options: `maxAge` (seconds), `expires`, `path`, `domain`, `httpOnly`, `secure`, `sameSite`, `partitioned`. `__Host-` names always get `Secure` and `Path=/` and reject `domain`. Cookies and headers apply to the response that goes out, also a `Response` you return.

## Redirects and errors

```js
import { redirect, error } from 'pragma/server'

export async function load({ locals, params }) {
  if (!locals.user) redirect(303, '/login')
  const order = await findOrder(params.id)
  if (!order) error(404, 'Order not found')
  return { order }
}
```

- `redirect(status, location)`: status 301, 302, 303, 307, or 308. Use 303 after an action.
- `error(status, body)`: status 400-599. `body` is a message string or an object with `message`.
- Both throw. Use them in `load`, actions, endpoints, and middleware. `'pragma/server'` works only in `.server.js` files.

| Request | `redirect` | `error` |
|---|---|---|
| Page load (`/_data`) or `call()` (`/_action`) | HTTP 200 `{"redirect":{"status":303,"location":"/x"}}`; the browser navigates. | Its status with `{"error": body}`; `page.error` or `call()` throws. |
| Full page (SSR or prebuilt page behind middleware) | Real 30x with `Location`. | SSR page: its status with `page.error`. Otherwise 404 serves `_404.html`, other statuses a plain text response. |
| Endpoint | Real 30x with `Location`. | Its status with `{"error": body}`. |

## Middleware

`routes/_middleware.server.js` (one per app) runs before every request: pages (also prebuilt ones), `/_data`, `/_action`, and endpoints. It does not run for `/assets/*`, `/_pragma/*`, and `/_otel`.

```js
import { redirect } from 'pragma/server'
import { findSession } from './_session.server.js'

export async function handle({ url, cookies, locals, env, next }) {
  locals.user = await findSession(env, cookies.get('__Host-session'))
  if (url.pathname.startsWith('/app') && !locals.user) redirect(303, '/login')
  const response = await next()
  response.headers.set('x-frame-options', 'DENY')
  return response
}
```

- `next()` returns the `Response`. Return it, change it, or return another `Response`.
- Check `url.pathname` to choose the protected paths. `url` is the page path also for `/_data` and `/_action`.

## Endpoints

A `.server.js` without a sibling `.html` is an endpoint. `routes/files/[name].server.js` serves `/files/:name` with the same param rules as pages.

```js
import { error } from 'pragma/server'

export async function GET({ params, files, request }) {
  const file = await files.get(params.name, { range: request.headers })
  if (!file) error(404, 'File not found')
  const headers = new Headers({ 'content-type': file.httpMetadata?.contentType ?? 'application/octet-stream' })
  if (request.headers.has('range') && file.range) {
    const end = file.range.offset + file.range.length - 1
    headers.set('content-range', `bytes ${file.range.offset}-${end}/${file.size}`)
    return new Response(file.body, { status: 206, headers })
  }
  return new Response(file.body, { headers })
}
```

- Export only uppercase method handlers. `HEAD` uses `GET` without a body. Other methods get 405 with `Allow`.
- Every handler returns a `Response`: binary bodies, `206` ranges, `ReadableStream` streams, redirects.
- WebSocket: return `new Response(null, { status: 101, webSocket: client })` from `GET`. This works only on Workers (`wrangler dev`, deploy), not on Node (`pragma dev`, `pragma serve`).
- Do not use endpoints for page data or writes from your pages. Use `load` and actions. `Response.json(...)` in an endpoint gives the warning `app/endpoint-json`.

## Streaming from an action

An action can return a `Response`. Its body and headers reach the client untouched, so a `ReadableStream` (for example NDJSON lines) streams chunk by chunk:

```js
export const actions = {
  async ask({ input, env }) {
    const lines = await startAnswer(env, input.question)
    return new Response(lines, { headers: { 'content-type': 'application/x-ndjson' } })
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

- Throwing in `load` sets `page.error` and returns HTTP 500. Use `error(404, 'message')` for another status, or return `{ product: null }` for not-found states.
- Put every write and every secret-using operation in a server action. Update `data` or a `let` after the call.
- Read `data` inside functions only (`js/top-level-data`). Without `ssr`, `data` is undefined until `load` finishes.
- Load data from `fetch` with secrets from `env`, a database via `env`, or `import products from './_products.json' with { type: 'json' }`.
- Default mode is no SSR: prebuilt HTML holds static content outside `<if>`/`<for>`, and `.pragma/boot.js` renders the skeleton.
- Enable `ssr = true` when crawlers, link previews, or data-bound `<head>`/JSON-LD tags need server data (`routes.md`).
- Do not fetch data in `mount` or client actions. Do not create `/api/...` routes or endpoints for page data. Do not return secrets from `load`. `env` exists only on the server.
- Put auth checks in `routes/_middleware.server.js` and keep the user in `locals`. Store sessions in `__Host-` cookies.
