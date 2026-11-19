# Routes and router

| File | Route |
|---|---|
| `routes/index.html` | `/` |
| `routes/about.html` | `/about` |
| `routes/shop/index.html` | `/shop` |
| `routes/shop/[id].html` | `/shop/:id` |

- Every `.html` in `routes/` is a route, except names starting with `_`. `x.html` plus `x/index.html` = error.
- `[name]` (file or folder) = path param; a static segment beats a param. No `[...rest]`, no nested layouts.
- A page is a fragment: no `<html>`, `<head>`, `<body>`. `<title>`, `<meta>` and `<link>` at the top level of the page go to `<head>` (see SEO).
- Read params as `params.id`, `query.filter` in HTML and in scripts (`import { params, query } from 'pragma'`). Always strings.

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
    <span>{shared.cart.count}</span>
  </nav>
</header>
<main data-page><slot></slot></main>
```

- `<head>` is optional and must come first. Its tags go on every page; it is static (no `:` bindings, no `{}`). Exactly one `<main data-page>` containing only `<slot></slot>`.
- The layout has no script: it reads only `shared`, `params`, `query`. For behavior, use a component with a `<script>`.
- `routes/_404.html` renders unknown routes. Neither it nor the layout has a `.server.js`.

## SEO and link previews

Site-wide tags go in the layout `<head>`. Each page writes its own `<title>`, `<meta>` and `<link>` at the top level; a page tag replaces the layout tag with the same `name`, `property`, `http-equiv` or `rel` (+ `hreflang`); new ones are added.

`routes/about.html`:
```html
<title>About · Shop</title>
<meta name="description" content="Who we are">
<link rel="canonical" href="https://shop.example/about">
<meta property="og:title" content="About">
<meta property="og:url" content="https://shop.example/about">
<h1>About</h1>
```

From server data: write `{path}` in `<title>` and `:<attr>` or `{path}` in a `<meta>`/`<link>` attribute, with `data`, `params` or `query`, and `export const ssr = true` in the page's `.server.js`. The server renders them into the first HTML, so crawlers and link-preview bots (no JS) see them.

`routes/blog/[slug].html`:
```html
<title>{data.post.title} · Shop</title>
<meta name="description" :content="data.post.summary">
<link rel="canonical" href="https://shop.example/blog/{params.slug}">
<meta property="og:type" content="article">
<meta property="og:title" :content="data.post.title">
<meta property="og:image" :content="data.post.image">
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "Article",
  "headline": "{data.post.title}",
  "image": "{data.post.image}",
  "datePublished": "{data.post.date}",
  "author": { "@type": "Person", "name": "{data.post.author}" }
}
</script>
<h1>{data.post.title}</h1>
```
`routes/blog/[slug].server.js`:
```js
export const ssr = true

export async function load({ params, request }) {
  return { post: await findPost(params.slug), url: new URL(request.url).href }
}
```

- A missing value keeps the tag's static text or attribute; a `<meta>` without `content` or a `<link>` without `href` is dropped.
- Structured data (JSON-LD): one `<script type="application/ld+json">` per schema, at the top level of the page (never in a component); it goes to `<head>`. It must be valid JSON (`html/json-ld`). `{path}` works only inside its strings (`"headline": "{data.post.title}"`), with `data`, `params` or `query` and `ssr`; the server writes each value JSON-escaped. A literal `{` inside a string is `{`. Without `{}` it is static and needs no `ssr`. Any other `<script type>` is an error (`html/script`).
- Never put `canonical` or `og:url` in the layout: every page would point to the same URL.
- `<meta>` and `<link>` anywhere else in a page or component are an error (`html/head`).
- The router updates `<title>`, `<meta>` and `<link>` on every navigation.

## Active link

- An `<a href="/path">` (no query or hash) pointing to the current route gets `aria-current="page"` automatically. Style it in `lib/theme.css`: `.nav-link[aria-current="page"]`.
- For query-based links (filters, tabs), bind `:aria-current` to a derived function returning `'true'` or `false`:

```html
<script>
  import { query } from 'pragma'

  const pendingLink = () => query.filter === 'pending' && 'true'
</script>
<a href="?filter=pending" :aria-current="$pendingLink">Pending</a>
```

## Navigation

- Use `<a href>`. The router swaps only `<main data-page>`, prefetches links and keeps the layout.
- Every navigation, including query-only changes, recreates the page state.
- From JS after an action: `navigate('/shop?filter=sale')` (`import { navigate } from 'pragma'`).
- Full page load: `<a href="/logout" data-reload>`. Links with `target` or `download` also bypass the router.
- Never use `<button>` + `navigate` instead of `<a>`, `location.href`, `history.pushState`, or `location` to read params.
