# DX review: new component API (opus)

Method: read proposal, syntax/components/state/data docs, starter and AGENTS.md. Wrote 12 wrong pages, components and stores in a scratch app (`pragma new`) and ran `check`. Built and ran a probe app, re-shot the site at 1440/390 and re-read my own migration (starter, core/example, packages app). The site: 0 console errors, no overflow, CLS ≤0.003. e2e 126/126.

## P0
1. **Compiler and runtime can drift apart with no guard.** After BUG-S-17 the compiler emits a `comp` op. With a `core/` vendored one hour earlier, `/shop/:id` answered **500 "unknown op: comp"**: `pragma build` and `dev` passed, only `check` says `app/core-outdated` (cli/src/lib.rs:151). Every app breaks this way after a CLI upgrade. Fix: `build` and `dev` fail (or `dev` auto-upgrades) when `core/` differs; put the op-set version in the manifest and make render.js fail with "run `pragma upgrade`".

## P1
2. **A derived function used as a value in JS passes check.** `if (empty) return` is always truthy (scratch n2). This is the main trap of "`$empty` in HTML, `empty()` in JS". Fix: a jslint rule `js/derived-call` for a top-level derived function in a test, operator or template without a call: "call it: `empty()`".
3. **`$draft = ''` inside `<script>` passes check**, then fails at runtime with ReferenceError. Agents copy `$` from the HTML. Fix: flag a `$name` identifier when the script declares `name`: "in JS write `draft`".
4. **Reading `data` at the top level passes check** (`let products = data.products`, n1). On a page without `ssr` this throws a TypeError, and state.md:113 only says not to. Fix: a rule for any read of `data` outside a function.
5. **Store actions need wrapper functions to be called from events.** `@click="shared.cart.clear"` is rejected ("write a function name"). Evidence: core/example cart-summary.html:4-18 (4 wrappers), tasks/index.html:44,66 (mine), site stores basket.html:4-10. Fix: accept a store-function path as the handler (`@click="shared.cart.remove(item.id)"`), or say in state.md that the wrapper is the rule.
6. **Events are delegated, so `event.currentTarget` is `document`** and the element with the handler cannot be reached. My tabs workaround is `event.target.closest('[role="tablist"]')` (core/example/lib/components/tabs/tabs.html:27). Fix: an `el` argument keyword like `event` (`@keydown="move(event, el)"`), and one line in syntax.md:110.
7. **Errors mask each other in one file.** `js/const-state` showed only after `js/store-import` and `js/prop-write` were fixed. In m2, `pick` (unknown-name) was hidden behind `js/legacy-api`. Also a false cascade: `export function mount` makes `import { data } from 'pragma'` in a page report "'pragma' is only for page, component and store scripts" (compiler lib.rs:204). Fix: report all script rules in one pass, and drop the import error when legacy-api fires.
8. **Derived lists of copies make the documented action pattern a silent no-op.** My tasks page maps `{ ...task, editing }` (core/example/routes/tasks/index.html:16-21), so `toggle(task) { task.done = !task.done }` (the doc pattern) would change a copy. I had to use `shared.tasks.find(task.id)` (:39-42). Fix: return `{ task, editing }` rows (my code), and add a rule in state.md: "items from `.map(x => ({ ...x }))` are copies: change the source".
9. **AGENTS.md (mine) is stale or thin.** core/starter/AGENTS.md:9 and core/example/AGENTS.md:9 say `function mount()`, but the API is now `mount(el)`. They miss the most needed lines: "in JS call derived values: `total()`", "`{}` is literal in text directly inside `<code>`/`<pre>`; escape with `\{`". Fix these 3 lines.

## P2
10. The invalid-path hint always suggests `const total`/`$total` (compiler directive.rs:213), also for `{$items.length > 0}`. For `{$total()}` say "drop the `()`: `{$total}` calls it". The column points at the text start, not at the `{`.
11. Add "did you mean …?" hints: `valeu` → `value` (expand.rs:406 now suggests declaring the typo); `{count}` → `{$count}` when the script declares `count`; `{curly}` in prose → hint `\{`.
12. Duplicate errors: `class="btn {$x}"` gives css/unknown-class plus html/class; `:model="name"` is reported twice (validate.rs:415 and expand.rs:740).
13. There are 2 ways to bind a whole attribute: `aria-label="{$label}"` (site components/_components/counter/counter.html:24) and `:aria-label="$label"` (example counter.html). Pick one in syntax.md and lint the other.
14. These pass check silently: `let tags = new Set(); tags.add()` never re-renders (n6), and a `mount` that adds a listener with no cleanup (n3). Warn for both.
15. Store internals become public: `shared.cart.money` (site lib/state/cart.js:1) and `shared.tasks.createTask` (example tasks.js:1). Expose only `let`s and functions, or use `_` for private names.
16. The new starter page is 10.0 KB JS gzip (was 8.8). The landing, guide, OG image and principles still say 8.8 KB.

## What works well
`js/legacy-api`, `js/prop-write`, `js/store-import`, `html/unknown-prop` (lists the props), `html/text-removed`, `html/duplicate-module`, the `${}` message and the persist typo are precise and actionable. Typed props (`export let min = -Infinity`), deep-proxy `let`, event args and `{}` in attributes made my migrated code about 30% shorter and clearer.

## Top 5
1. Guard compiler/runtime drift: `build`/`dev` fail on an outdated `core/` (now a silent SSR 500 "unknown op: comp").
2. Catch the 3 silent JS traps in check: a derived function without `()`, `$x` in a script, `data` at the top level.
3. Accept store functions as event handlers (`@click="shared.cart.remove(item.id)"`) and remove the wrapper boilerplate.
4. Report all script errors in one pass, with "did you mean" hints (`{$count}`, `value`, `{$total}`) and no duplicates.
5. Fix AGENTS.md and the docs: `mount(el)`, "call derived values in JS", `el` for the event element, the copies-in-derived-lists rule.
