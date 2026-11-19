# npm packages

Add a package only when the platform cannot do the job. First check `Intl`, `fetch`,
`crypto.randomUUID`, `URLSearchParams`, `structuredClone`.

## Install

```sh
npm i date-fns --ignore-scripts
```

- The package goes in `dependencies` of `package.json`. Importing a package not listed there = error.
- Any package manager works (npm, bun, pnpm): pragma reads `package.json` and `node_modules/`.
- Keep `node_modules/` out of git.

## Import

```js
import { format } from 'date-fns'

let today = format(new Date(), 'PPP')
```

- Use a quoted specifier, never a template literal.
- Import from the index or a subpath: the build bundles each package with tree-shaking; only what the app uses ships.
- Each package becomes `/assets/npm/<package>-<hash>.js` (`@date-fns/tz` → `date-fns__tz`), with `modulepreload`. Code shared by several packages goes to `/assets/npm/_*.js`. Package size counts toward the page budget.
- `import * as x from 'pkg'` and `import('pkg')` ship the whole package: import names.
- `process.env.NODE_ENV` becomes `"production"` in build and `"development"` in dev.

## What works

JavaScript packages: ES modules or CommonJS (`require`, `module.exports`), files `.js`, `.mjs`,
`.cjs` (plus `.json` imported by the package). CommonJS has no tree-shaking: it ships whole.
Between two options, choose the ES module one.

| Does not work | Error |
|---|---|
| TypeScript, JSX, `.vue`, `.svelte`, imported CSS or assets | `app/package` |
| `node:*` or Node built-ins (`fs`, `path`…) on the client | `app/package` |
| Importing a name the package does not export | `js/import` |
| UI frameworks (React, Preact, Vue, Svelte, Solid, lit, jQuery, Alpine…) | `js/import` |

The error points at the import in your file. In `.server.js`, any package from `package.json` works, plus Node built-ins (`fs`, `node:*`) without listing. A built-in name listed in `package.json` (a polyfill such as `buffer`) is bundled as a package.

## Recommended

| For | Use |
|---|---|
| Formatting dates, numbers, currency, plurals, relative time | native `Intl.DateTimeFormat`, `Intl.NumberFormat`, `Intl.PluralRules`, `Intl.RelativeTimeFormat` |
| Date math (add, difference, start of month) | `date-fns`; time zones: `@date-fns/tz` |
| Form or payload validation | `valibot` |
| Fuzzy search in a list | `fuse.js` |
| Charts | `chart.js` (draws into a `<canvas>` in the `.html`) |
| Debounce, throttle, group, sort | `es-toolkit` |
| Unique ID | native `crypto.randomUUID()` |
| HTTP | native `fetch` |

## Do not use

| Package | Reason | Use |
|---|---|---|
| `moment` | heavy, no tree-shaking | `date-fns`, `Intl` |
| `lodash` | CommonJS: ships whole (~25 kB gzip) even for one function | `es-toolkit` |
| `axios` | heavy; `fetch` suffices | `fetch` |
| `uuid`, `nanoid` | native suffices | `crypto.randomUUID()` |
| libraries that build HTML (`marked`, template engines, UI kits) | HTML in JS is forbidden | markup in the `.html` |
