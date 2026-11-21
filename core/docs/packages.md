# npm packages

Add a package only when the platform cannot do the task. Check `Intl`, `fetch`, `crypto.randomUUID`, `URLSearchParams`, and `structuredClone` first.

## Install

```sh
npm i date-fns --ignore-scripts
```

- List every imported package in `package.json` `dependencies`. Importing an unlisted package is an error.
- Use any package manager (npm, bun, pnpm); pragma reads `package.json` and `node_modules/`.
- Exclude `node_modules/` from git.

## Import

```js
import { format } from 'date-fns'

let today = format(new Date(), 'PPP')
```

- Use a quoted module specifier; do not use template literals.
- Import named exports from the index or a subpath. The build tree-shakes each ES module package.
- Each package outputs `/assets/npm/<package>-<hash>.js` (`@date-fns/tz` -> `date-fns__tz`) with `modulepreload`. Shared chunks go to `/assets/npm/_*.js`. Package size counts in the page JS size that `pragma build` prints.
- Avoid `import * as x from 'pkg'` and `import('pkg')` when possible; they bundle the whole package.
- `process.env.NODE_ENV` evaluates to `"production"` in build and `"development"` in dev.

## Supported packages

Client code supports plain JS packages (`.js`, `.mjs`, `.cjs`, plus imported `.json`) using ES modules or CommonJS. CommonJS packages cannot be tree-shaken and ship whole; prefer ES modules.

| Unsupported on client | Error |
|---|---|
| TypeScript, JSX, `.vue`, `.svelte`, imported CSS or assets | `app/package` |
| `node:*` or Node built-ins (`fs`, `path`) | `app/package` |
| Importing an export the package does not provide | `js/import` |
| UI frameworks (React, Preact, Vue, Svelte, Solid, lit, jQuery, Alpine) | `js/import` |

In `.server.js`, all `package.json` dependencies and Node built-ins (`fs`, `node:*`) work without extra listing. A built-in name listed in `package.json` (such as `buffer`) bundles as a package.

## WebAssembly (server only)

Import a `.wasm` file, from the project (`./_solver.wasm`) or from a package export (`highs/runtime`), in server code. The import gives a compiled `WebAssembly.Module` on Workers and on Node (`pragma dev`, `pragma serve`, `node.mjs`). The build copies the file to `dist/server/` and adds a `CompiledWasm` rule to `dist/server/wrangler.jsonc`. A root `wrangler.jsonc` needs no rule (wrangler treats `*.wasm` the same way by default).

```js
import loadHighs from 'highs'
import highsWasm from 'highs/runtime'

let loading

export function solver() {
  loading ??= loadHighs({
    instantiateWasm(imports, done) {
      WebAssembly.instantiate(highsWasm, imports).then(done)
      return {}
    },
  })
  return loading
}
```

- Workers cannot compile WebAssembly from bytes. Pass the imported module to the package (Emscripten packages: `instantiateWasm`).
- Importing `.wasm` in client code is a `js/import` error.

## Workers modules

`cloudflare:workers`, `cloudflare:sockets` and other `cloudflare:*` modules work in `.server.js` on Workers only. They do not exist on Node (`pragma dev`, `pragma serve`, `node.mjs`). In client code they are an `app/package` error. Prefer the `env` that `load` and actions receive.

## Recommended

| Need | Use |
|---|---|
| Format dates, numbers, currency, plurals, relative time | Native `Intl.DateTimeFormat`, `Intl.NumberFormat`, `Intl.PluralRules`, `Intl.RelativeTimeFormat` |
| Date math (add, diff, start of month) | `date-fns`; time zones: `@date-fns/tz` |
| Form or payload validation | `valibot` |
| Fuzzy list search | `fuse.js` |
| Charts | `chart.js` (render into `<canvas>` in `.html`) |
| Debounce, throttle, group, sort | `es-toolkit` |
| Unique ID | Native `crypto.randomUUID()` |
| HTTP requests | Native `fetch` |

## Do not use

| Package | Reason | Use instead |
|---|---|---|
| `moment` | Large, no tree-shaking | `date-fns`, `Intl` |
| `lodash` | CommonJS; ships whole (~25 kB gzip) | `es-toolkit` |
| `axios` | Large; redundant | `fetch` |
| `uuid`, `nanoid` | Redundant | `crypto.randomUUID()` |
| HTML-building libraries (`marked`, template engines, UI kits) | HTML in JS is forbidden | Markup in `.html` |
