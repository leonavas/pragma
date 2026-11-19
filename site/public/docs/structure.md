# App structure

Fixed. Create no other top-level folders. Never edit `core/`.

```
<app>/
  AGENTS.md  CLAUDE.md
  pragma.json              { "name": "app", "lang": "en", "webmcp": true, "budgets": { "initialJsKb": 15, "routeJsKb": 10, "cssKb": 15 } }
  package.json             optional: npm packages in dependencies (packages.md)
  routes/
    _layout.html  _404.html
    index.html             page / (optional <script> and <style> at the top)
    index.js               the page script as a file, instead of the <script> (optional)
    index.server.js        data (optional)
    shop/
      [id].html            /shop/:id
      _catalog.server.js   private server helper
      _products.json       private data
      _components/<name>/  <p-name>
  lib/
    components/<name>/     <c-name>; modal, alert, dropdown ship with the starter
    state/<name>.js        stores (reached through `shared`, never imported)
    theme.css              tokens and custom classes
  utils/                   pure functions
  public/                  static files (images, fonts, robots.txt, <page>.md for agents), served as-is
  core/                    framework; do not edit
```

`pragma.json`: `name` required; `lang` sets `<html lang>` (default `"en"`); `webmcp` (default
`true`) makes every `<form>` an agent tool and turns on the `webmcp/*` rules (webmcp.md). Budgets, gzip kB:
`initialJsKb` all JS a page loads, `routeJsKb` the page's JS outside `core/`, `cssKb` global CSS.

| Thing | Location |
|---|---|
| Page | `routes/<path>.html` (with its `<script>`/`<style>`, or `.js`) + `.server.js` (same base name) |
| Server-only helper or JSON | `routes/<folder>/_name.server.js`, `_name.json` |
| Component for one route folder | `routes/<folder>/_components/<name>/` |
| Shared component | `lib/components/<name>/` |
| State shared across pages | `lib/state/<name>.js` |
| Token, color, global class | `lib/theme.css` |
| Pure function | `utils/<name>.js` |
| Markdown of a page for agents | `public/<path>.md` or `public/<path>/index.md` (`/` → `public/index.md`; webmcp.md) |

- A leading `_` makes a file or folder private (not a route). Only `_layout.html` and `_404.html` are special.
- `<script>` and `<style>` only once each, at the top level of a page or component `.html` (never in the layout); or the same base name as `.js`/`.css`, not both. No `src/`, `pages/`, `styles/`, `helpers/`.
- CSS lives in a component's or page's `<style>` (or the component's `.css`), or `lib/theme.css`.
- Names: component folder = lowercase letters, digits, hyphens; route param `[name]` = letters, digits, `_`; store file = JS identifier.
