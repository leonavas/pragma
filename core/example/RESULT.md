# Minimart on pragma

The `tests_1/SPEC.md` app built with pragma. It is the canonical reference for agents: every
framework feature appears here at least once.

## Run

```sh
cd core/example
pragma dev            # http://localhost:5180, live reload
pragma check --strict # errors + accessibility warnings
pragma build && pragma serve   # dist/ as in production (http://127.0.0.1:3000)
```

End-to-end tests (Playwright, Chromium and Firefox), outside the app:

```sh
cd core/example/e2e && npm install && npx playwright install chromium firefox
PRAGMA=/path/to/pragma npm test     # runs pragma serve and pragma dev on copies of the app
```

`e2e/minimart.test.mjs` covers the app under `pragma serve`; `e2e/dev.test.mjs` covers `pragma dev`
(SSR, CSS live reload without a page reload). Variables: `PRAGMA`, `PLAYWRIGHT_DIR`, `BROWSERS`,
`PORT`, `DEV_PORT`.

## Routes

| Route | Files | Features |
|---|---|---|
| layout | `routes/_layout.html` | nav with automatic `aria-current`; cart badge (`shared.cart.count`) |
| `/` Dashboard | `routes/index.html` | `c-indicator` ×3 (`value`/`tone` aliases, tone functions in the page `<script>`); `c-counter` ×3 with typed props (`export let step = 1`, one inside `c-tabs`); `c-clock` (`mount` with cleanup, a timer assigns a `let`); `c-theme-toggle` (`core/theme.js`, no flash); `c-tabs` (named slots, ← →, `@click="select('first')"`); `c-modal` |
| `/tasks` | `routes/tasks/index.html` | persisted `tasks` store (`let items`, derived functions, actions); validated form; keyed list with inline edit (`autofocus` in a new block) and event arguments (`@change="toggle(task)"`); `c-search`; `c-filters` with `?filter=` in the URL; `c-dropdown` + `c-alert` with `confirm` |
| `/shop` | `routes/shop/index.html`, `index.server.js` | `load` (source `routes/shop/_products.json`, translated from `tests_1/shared/produtos.json`, 400 ms delay) with skeleton, error with `?fail=1` and server-side `?category=` filter; client-side search in a derived function; `_components/product-card` (no script), `add-to-cart` (object prop `export let product`), `cart-summary` (alert with `confirm`) |
| `/shop/:id` | `routes/shop/[id].html`, `[id].server.js` | `ssr = true`; reuses `p-add-to-cart`; server action `like` via `call()` with OTel logs on both sides |
| 404 | `routes/_404.html` | |

Stores: `lib/state/cart.js` and `lib/state/tasks.js` (persisted). Pure utilities:
`utils/money.js`, `utils/text.js`. Custom classes in `lib/theme.css`: `card`, `btn*`, `field`,
`nav-link`, `dialog`; everything else is utilities.

`pragma build` (prod): first-visit JS for `/` 10.3 kB gzip (sum per file), CSS 4.7 kB, build in
~20 ms. Browser measurements (FCP, slow profiles) are left to the author.

## Deviations from `tests_1/SPEC.md`

- **Counter inside ProductCard.** In pragma a parent does not read child state. The quantity
  picker with "Add" is the `add-to-cart` component (`quantity` state, limits via props), used in
  the card and the detail page. `counter` is reused 3× on the Dashboard.
- **Shop cache.** Returning to the shop refetches `GET /_data/shop` (data comes from the server per
  navigation; data caching is out of v0). The 400 ms delay lives in `load`.
- **Category filter** runs on the server (`load({ query })`); text search runs on the client.
- **"Try again"** is a link to `/shop` (without `?fail=1`).
- **Tabs** has 3 fixed tabs (props `first`, `second`, `third` and slots with the same names): props
  are text, not lists.
- **Like** (`/shop/:id`) is extra: it shows a server action; the counter lives in server memory.
- Initial tasks come from the store (no `localStorage`), as the SPEC asks.
- Prices are formatted as USD (`utils/money.js`); the SPEC data is in BRL.

## Framework integration bugs fixed

- Lexical slot owner (the compiler marks `data-slot`, the runtime evaluates in the writer's scope;
  action and `state` validation is lexical too).
- Scroll lock without shifting: `gutter` only when a scrollbar was present.
- Modulepreload: dynamic `import()` excluded; `overlay.js` lazy and preloaded only where an overlay
  exists.
- `state(ctx)` + `ctx.props` (`<c-x>` props become `data-*` on the root of a component with JS);
  derived getters in state and stores (kept out of `localStorage`).
- `data-with-x="state…"` on a component with JS is an error; `data-model` accepts an alias in a
  component.
- Automatic `aria-current` (build + router); theme without flash (`core/theme.js` + inline script).
- `data-attr-checked/selected` bind the property; a new block with `[autofocus]` gets focus.
- `self.__pragmaDev` was never set in dev; the dev build now injects it.
- `/shop/index.html` (HTML the router fetches) was treated as SSR of `/shop/:id`: `.html` paths are
  never SSR (Rust and handler).
- CSS live reload did not refetch `/assets/app.css` (same name): now with `?t=`.
- `pragma serve/dev` left node orphaned on SIGTERM.
- `a11y/*` warnings in `pragma check` and `--strict`.

## Pending

- Browser measurements (FCP on a slow profile, requests, comparison with `tests_1/COMPARACAO.md`).
- Rerun the e2e suite after the translation to English and the latest fixes (`dev.js` and
  SIGTERM). The last full run under `pragma serve` passed 24/24 in Chromium and Firefox; the
  Chromium CSS live reload test failed before the `dev.js` fix and was not rerun.
