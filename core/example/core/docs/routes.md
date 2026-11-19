# Routes and router

| File | Route |
|---|---|
| `routes/index.html` | `/` |
| `routes/about.html` | `/about` |
| `routes/shop/index.html` | `/shop` |
| `routes/shop/[id].html` | `/shop/:id` |

- Every `.html` in `routes/` is a route, except names starting with `_`. `x.html` plus `x/index.html` = error.
- `[name]` (file or folder) = path param; a static segment beats a param. No `[...rest]`, no nested layouts.
- A page is a fragment: no `<html>`, `<head>`, `<body>`. A `<title>` at the top becomes the page title.
- Read params as `params.id`, `query.filter` in HTML and `ctx.params`, `ctx.query` in JS. Always strings.

## Layout

`routes/_layout.html`:
```html
<head>
  <link rel="icon" href="/favicon.svg" type="image/svg+xml">
</head>
<header>
  <nav aria-label="Main">
    <a href="/" class="nav-link">Home</a>
    <a href="/shop" class="nav-link">Shop</a>
    <span :text="shared.cart.count"></span>
  </nav>
</header>
<main data-page><slot></slot></main>
```

- `<head>` is optional and must come first. Exactly one `<main data-page>` containing only `<slot></slot>`.
- The layout has no `.js`: it reads only `shared`, `params`, `query`. For behavior, use a component with a `.js`.
- `routes/_404.html` renders unknown routes. Neither it nor the layout has a `.server.js`.

## Active link

- An `<a href="/path">` (no query or hash) pointing to the current route gets `aria-current="page"` automatically. Style it in `lib/theme.css`: `.nav-link[aria-current="page"]`.
- For query-based links (filters, tabs), bind `:aria-current` to a getter returning `'true'` or `false`:

```html
<a href="?filter=pending" :aria-current="$filterLink.pending">Pending</a>
```
```js
export const state = ctx => ({
  get filterLink() {
    return { all: !ctx.query.filter && 'true', pending: ctx.query.filter === 'pending' && 'true' }
  },
})
```

## Navigation

- Use `<a href>`. The router swaps only `<main data-page>`, prefetches links and keeps the layout.
- Every navigation, including query-only changes, recreates the page state.
- From JS after an action: `ctx.navigate('/shop?filter=sale')`.
- Full page load: `<a href="/logout" data-reload>`. Links with `target` or `download` also bypass the router.
- Never use `<button>` + `navigate` instead of `<a>`, `location.href`, `history.pushState`, or `location` to read params.
