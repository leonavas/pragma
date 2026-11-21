# App structure

The folder structure is fixed. Do not create other top-level folders. Do not edit `.pragma/`.

```
<app>/
  AGENTS.md  CLAUDE.md
  pragma.json              { "name": "app", "lang": "en", "webmcp": true, "budgets": { "initialJsKb": 15, "routeJsKb": 10, "cssKb": 15 } }
  package.json             optional: npm packages in dependencies (packages.md)
  routes/
    _layout.html  _404.html
    index.html             page / (optional <script> and <style> at top)
    index.js               optional page script file instead of <script>
    index.server.js        optional server data and actions
    _middleware.server.js  optional; runs before every request
    files/
      [name].server.js     endpoint /files/:name (no .html; returns a Response)
    shop/
      _layout.html         optional nested layout for /shop/*
      [id].html            /shop/:id
      _catalog.server.js   private server helper
      _products.json       private data
      _components/<name>/  <p-name>
  lib/
    components/<name>/     <c-name>; starter ships modal, alert, dropdown, select
    state/<name>.js        stores (access through `shared`, never import)
    theme.css              tokens and global custom classes
  utils/                   pure functions
  public/                  static files (images, fonts, robots.txt, <page>.md for agents)
  .pragma/                 framework (runtime, docs, base.css, version); do not edit
```

## `pragma.json`

- `name` is required.
- `lang` sets `<html lang>` (default `"en"`).
- `webmcp` (default `true`) turns every `<form>` into an agent tool and enables `webmcp/*` rules (`webmcp.md`).
- `budgets` sets gzip kB limits: `initialJsKb` (all JS loaded by a page), `routeJsKb` (page JS outside `.pragma/`), `cssKb` (global CSS).

## `.pragma/` versioning

- `.pragma/version` records the `.pragma/` version.
- Compare `pragma -V` with `.pragma/version`.
- Run `pragma upgrade` to update `.pragma/` to match the CLI (`cli.md`).
- Run `pragma self-update [version]` to update the CLI binary (`cli.md`).

## File locations

| Item | Location |
|---|---|
| Page | `routes/<path>.html` (with `<script>`/`<style>`, or `.js`) + `.server.js` (same base name) |
| Root layout | `routes/_layout.html` |
| Nested layout | `routes/<folder>/_layout.html` |
| Server-only helper or JSON | `routes/<folder>/_name.server.js`, `_name.json` |
| Middleware (auth, headers) | `routes/_middleware.server.js` |
| Endpoint (file, stream, webhook, WebSocket) | `routes/<path>.server.js` without `.html` |
| Route-local component | `routes/<folder>/_components/<name>/` |
| Shared component | `lib/components/<name>/` |
| Shared state store | `lib/state/<name>.js` |
| Token, color, global class | `lib/theme.css` |
| Pure function | `utils/<name>.js` |
| Agent markdown page | `public/<path>.md` or `public/<path>/index.md` (`/` -> `public/index.md`; `webmcp.md`) |

## Rules

- A leading `_` makes a file or folder private (not a route). Only `_layout.html`, `_404.html`, and `routes/_middleware.server.js` are special.
- Put at most one `<script>` and one `<style>` at the top level of a page or component `.html` (never in layouts). Or use `.js`/`.css` with the same base name, not both.
- Do not create `src/`, `pages/`, `styles/`, or `helpers/`.
- Put CSS in a component or page `<style>`, a component `.css`, or `lib/theme.css`.
- Naming: component folder uses lowercase letters, digits, hyphens; route param `[name]` uses letters, digits, `_`; store file uses a valid JS identifier.
