# App structure

Fixed. Create no other top-level folders. Never edit `core/`.

```
<app>/
  AGENTS.md  CLAUDE.md
  pragma.json              { "name": "app", "lang": "en", "budgets": { "initialJsKb": 15, "routeJsKb": 10, "cssKb": 15 } }
  package.json             optional: npm packages in dependencies (packages.md)
  routes/
    _layout.html  _404.html
    index.html             page /
    index.js               behavior (optional)
    index.server.js        data (optional)
    shop/
      [id].html            /shop/:id
      _catalog.server.js   private server helper
      _products.json       private data
      _components/<name>/  <p-name>
  lib/
    components/<name>/     <c-name>; modal, alert, dropdown ship with the starter
    state/<name>.js        stores
    theme.css              tokens and custom classes
  utils/                   pure functions
  public/                  static files (images, fonts, robots.txt), served as-is
  core/                    framework; do not edit
```

`pragma.json`: `name` required; `lang` sets `<html lang>` (default `"en"`). Budgets, gzip kB:
`initialJsKb` all JS a page loads, `routeJsKb` the page's JS outside `core/`, `cssKb` global CSS.

| Thing | Location |
|---|---|
| Page | `routes/<path>.html` + `.js` + `.server.js` (same base name) |
| Server-only helper or JSON | `routes/<folder>/_name.server.js`, `_name.json` |
| Component for one route folder | `routes/<folder>/_components/<name>/` |
| Shared component | `lib/components/<name>/` |
| State shared across pages | `lib/state/<name>.js` |
| Token, color, global class | `lib/theme.css` |
| Pure function | `utils/<name>.js` |

- A leading `_` makes a file or folder private (not a route). Only `_layout.html` and `_404.html` are special.
- No `<script>` in `routes/` or `lib/`. No `src/`, `pages/`, `styles/`, `helpers/`.
- CSS lives in a component's `.css` or `lib/theme.css`.
- Names: component folder = lowercase letters, digits, hyphens; route param `[name]` = letters, digits, `_`; store file = JS identifier.
