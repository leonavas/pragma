# Routes and router

| File | Route |
|---|---|
| `routes/index.html` | `/` |
| `routes/about.html` | `/about` |
| `routes/shop/index.html` | `/shop` |
| `routes/shop/[id].html` | `/shop/:id` |

- Every `.html` in `routes/` is a route, except names starting with `_`.
- A `.server.js` without a sibling `.html` is an endpoint that returns a `Response` (`routes/files/[name].server.js` -> `/files/:name`; `data.md`, Endpoints). A page and an endpoint cannot serve the same path (`app/route-conflict`).
- `routes/_middleware.server.js` runs before every request (`data.md`, Middleware).
- Having both `x.html` and `x/index.html` is an error (`app/route-conflict`).
- `[name]` (file or folder) defines a path param. Static segments take priority over params. `[...rest]` is not supported.
- A page is an HTML fragment without `<html>`, `<head>`, or `<body>`. Top-level `<title>`, `<meta>`, and `<link>` move to `<head>` (see SEO).
- Read params as `params.id` and `query.filter` in HTML and scripts (`import { params, query } from 'pragma'`). Both are always strings.

## Root layout

`routes/_layout.html` wraps all routes:

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

- `<head>` is optional and must come first. Its tags apply to every page and must be static (no `:` bindings, no `{}`).
- Include exactly one `<main data-page>` containing only `<slot></slot>`.
- The layout has no `<script>`. It reads only `layout`, `shared`, `params`, and `query`. Use a component with `<script>` for behavior.
- The layout and the components in it never read `data`: `data` belongs to each page and is empty during every navigation (`layout/data-in-layout`). What the layout shows on every page (the signed-in user, a counter) comes from `layout`.

## Layout data

`routes/_layout.server.js` exports only `load`. It gets the same context as a page `load` (`request`, `url`, `cookies`, `locals` from the middleware) and returns what the layout shows. Read it as `layout` in any HTML (layout, pages, components) and as `import { layout } from 'pragma'` in scripts.

`routes/_layout.server.js`:
```js
export async function load({ locals }) {
  return { user: locals.user }
}
```

`routes/_layout.html`:
```html
<span>{layout.user.name}</span>
```

- `load` runs on the full page load (with the page `load`, in parallel) and again after each action (`call`, `stream`). A client navigation keeps the value, so the layout does not change while the next page loads.
- Before the first answer, `layout` is `null`; with `ssr`, the value comes in the page.
- Actions that change it (sign in, sign out, edit the profile) need nothing extra: `layout` updates after them.
- `routes/_404.html` renders unknown routes. It has no `.server.js`.

## Nested layouts

Any subfolder in `routes/` can add `_layout.html` with `<slot></slot>` to wrap routes in that folder and its subfolders.

`routes/shop/_layout.html`:
```html
<div class="grid gap-6 md:grid-cols-4">
  <aside aria-label="Shop menu">
    <a href="/shop" class="nav-link">All products</a>
    <a href="/shop/deals" class="nav-link">Deals</a>
  </aside>
  <section class="md:col-span-3">
    <slot></slot>
  </section>
</div>
```

- A nested `_layout.html` wraps the page inside the root `<main data-page>`.
- A nested `_layout.html` has no `<head>`, no `<script>`, and no `.server.js`.
- It reads only `layout`, `shared`, `params`, and `query`.
- Include one `<slot></slot>` for the child route or inner layout.

## SEO and link previews

Put site-wide tags in `routes/_layout.html` `<head>`. Put page `<title>`, `<meta>`, and `<link>` at the top level of each page. A page tag replaces the layout tag with the same `name`, `property`, `http-equiv`, or `rel` (+ `hreflang`); new tags are added.

Every page needs a `<meta name="description">`, its own or the layout's: without one, `pragma check` warns `html/meta-description`. Write what the page shows in 1–2 sentences; a private app can keep one in the layout.

`routes/about.html`:
```html
<title>About · Shop</title>
<meta name="description" content="Who we are">
<link rel="canonical" href="https://shop.example/about">
<meta property="og:title" content="About">
<meta property="og:url" content="https://shop.example/about">
<h1>About</h1>
```

For server data, use `{path}` in `<title>` and `:<attr>` or `{path}` in `<meta>`/`<link>` with `data`, `params`, or `query`. Add `export const ssr = true` in `.server.js` so crawlers and link previews receive them in the first HTML.

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

- Missing values keep the tag's static text or attribute. A `<meta>` without `content` or `<link>` without `href` is dropped.
- Put JSON-LD `<script type="application/ld+json">` at the top level of a page (never in a component); it moves to `<head>`.
- JSON-LD must be valid JSON (`html/json-ld`). `{path}` works inside strings with `data`, `params`, or `query` and `ssr`. The server JSON-escapes each value. A literal `{` in a string is `{`. Static JSON-LD needs no `ssr`. Other `<script type>` values trigger `html/script`.
- Do not put `canonical` or `og:url` in the layout.
- Do not place `<meta>` or `<link>` outside the page top level or root layout `<head>` (`html/head`).
- The router updates `<title>`, `<meta>`, and `<link>` on every navigation.

## Active link

- `<a href="/path">` (no query or hash) automatically gets `aria-current="page"` on the current route. The router sets it on the clicked link at the click, before the new page loads. Style `.nav-link[aria-current="page"]` in `lib/theme.css`.
- For query links (filters, tabs), bind `:aria-current` to a derived function that returns `'true'` or `false`:

```html
<script>
  import { query } from 'pragma'

  const pendingLink = () => query.filter === 'pending' && 'true'
</script>
<a href="?filter=pending" :aria-current="$pendingLink">Pending</a>
```

## Navigation

- Use `<a href>`. The router swaps `<main data-page>`, prefetches links, and keeps the root layout.
- Every navigation, including query-only changes, recreates page state.
- Navigate from JS after an action with `navigate('/shop?filter=sale')` (`import { navigate } from 'pragma'`). `navigate(url, { replace: true })` replaces the current history entry (wizard steps, fixing a URL).
- A redirect from `load` replaces the history entry; a redirect from an action adds one (`data.md`). A URL on another origin is a full page load.
- Force a full page load with `<a href="/logout" data-reload>`. Links with `target` or `download` also bypass the router.
- Do not use `<button>` + `navigate` instead of `<a>`. Do not use `location.href`, `history.pushState`, or `location` to read params.
