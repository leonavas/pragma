# DX review: fable (new component API)

Method: read the proposal, `core/docs/{syntax,components,state,data}.md`, `core/starter/AGENTS.md`; made a scratch app with `pragma new` and ran `pragma check --strict` on about 60 deliberate mistakes; built it once to read the output. "Check passes" below means I ran it. Runtime effects are from reading `core/runtime/page.js` and `directives.js`, not from a browser, unless marked "built".

Overall: the API is small and reads well. Most messages are precise (`html/unknown-prop` lists the props; `js/prop-write`, `js/store-import`, `html/text-removed`, `v-if` give the exact fix). The problems are the mistakes that pass check with no message.

## P0

1. **`<style>` in a component is not scoped, and check does not say so.** `<style>.stepper { display: flex } button { color: red }</style>` in `lib/components/stepper/stepper.html` passes `check --strict`; built CSS contains the global rule `button{color:red}` (built). `components.md:20` says "its CSS, scoped with `[data-c="name"]`", which reads as automatic; an agent from Svelte or Vue will assume that. Fix: the compiler prefixes each selector in `<style>` with `[data-c="name"]` (best), or check rejects a selector that does not start with it (`css/unscoped`). Then delete the manual prefix from the starter (`core/starter/lib/components/dropdown/dropdown.html:12-37`) and the site.
2. **Bare `@event="fn"` gives the function no arguments.** `function search(event) { query = event.target.value }` with `@input="search"` passes check; `page.js:104` calls `fn(...args)` with an empty list, so `event` is `undefined`. Same for `@click="remove"` inside `<for task …>` with `function remove(task)`. Every other framework passes the event by default, so this is the most likely first-try failure. Fix: check error when the function declares N required parameters and the binding passes fewer ("`search` takes `event`: write `@input="search(event)"`"). Add one line to `syntax.md:14` and AGENTS.md.
3. **A derived value written as a plain expression is silently stale.** `let doubled = count * 2` and `const title = 'Count ' + count`, both used as `{$doubled}` / `{$title}`, pass check. They never update. This is the main cost of "top-level `let` is reactive": it looks reactive. Fix: check error `js/stale-derived` when a top-level initializer reads another top-level `let` or a prop and the name is used in HTML, with the fix in the message ("write `const doubled = () => count * 2`"). Exempt the documented copy `let count = start` only when the name is assigned later.
4. **`data` read at the top level passes check.** `let products = data.products` (page with a `load`) passes; `state.md:107` and `data.md:81` forbid it and it throws before the load ends. Fix: `js/component-script` error for any `data.` read outside a function.

## P1

5. **Wrong or confusing messages** (all run):
   - `{count}` -> `unknown root "count" (use $, data, …)`. `count` is declared in the script; say "did you mean `{$count}`?".
   - `@click="$increase"` -> "write `function $increase() { … }`". Should say "no `$` in events: `@click="increase"`". The `$` rule is uneven (`$` for functions in values, none in events); state it in `syntax.md`.
   - `:disabled="{$count}"` and `:title="Count {$count}"` -> `unknown root "$count" (use $, …)`. Should say "`:attr` takes a bare path (`:disabled="$count"`); for text use `title="Count {$count}"`".
   - `<if $tasks.length > 0>` -> `html/block-single-root` at the `>`. Should be `html/invalid-path`: "no operators in `<if>`; use a function".
   - `<for task in $tasks>` (no key) prints three errors for one mistake (`html/block`, `html/for-without-key`, `html/invalid-path`) and then `unknown root "task"` for the body. Print one.
   - `{$price(task)}` -> generic "use `$total`" hint. Per-item values are the common case; say "add the field in the function that returns the list".
   - `import { state, onMount } from 'pragma'` also prints a false `js/import` "'pragma' is only for page, component and store scripts" for a page script.
6. **Silent passes that should be errors or warnings** (all run): `$: doubled = count * 2` (Svelte label, dead code); `this.count += 1`; top-level `setInterval(…)` and `document.title = 'x'` (leak per instance, DOM write, breaks SSR); an unused `function onMount()`; `export let maxItems` plus `maxItems="3"` at the call site (`components.md:42` says lowercase only, check does not enforce it); `step="five"` for a number prop; `{$product.name}` with `export let product = null`.
7. **`<pre>{$output}</pre>` is literal text, silently.** The rule in `syntax.md:32` (literal when the nearest parent is `<code>`/`<pre>`, live in a child element) is hard to remember. When the path is a declared name, warn: "`{$output}` is literal inside `<pre>`; wrap it in `<span>` to interpolate, or write `\{` to silence".
8. **Per-item derived values force a view-model map.** No arguments in `{}` means every formatted field needs `list.map(item => ({ ...item, label }))`: `overlays/_components/sort-menu/sort-menu.html:16`, `packages/_components/relative-time/relative-time.html:21`, `routing/demo/_components/query-tabs/query-tabs.html:13` (which also returns `false | 'true'` from `&& 'true'`). Docs have no example of this pattern. Fix: add it to `state.md` (one `<for>` with a mapped list), or allow `{$format(item.price)}` with path arguments, the same grammar events already have.
9. **Starter teaches none of the new idioms.** `core/starter/routes/index.html` has one `let` and one action: no `{}`, no derived function, no `<for>`, no `fn(arg)`, no prop with a script. An agent copies the starter first. Fix: replace the "Draft deleted" demo with a 15-line task list (the `syntax.md` example).
10. **Docs gaps and stale text.**
    - `site/AGENTS.md:17` still says "Derived values are getters on the state or store" (starter AGENTS.md is correct).
    - `data.md:82` is one 90-word bullet with five conditions about SSR; split it into a table (runs on server / left to the browser / throws).
    - No doc says how to keep a non-reactive instance variable (timer id, observer, DOM node). The site does `let resetTimer` (`lib/components/terminal/terminal.html:32`) and `frame = el.querySelector('iframe')` (`lib/components/frame/frame.html:48`) in reactive `let`s. Say: declare it inside `mount`, and what happens to a DOM node in a top-level `let`.
    - `site/routes/examples/state/index.html:111-117` "Coming from `state(ctx)`": v0 has no outside users of the old API; this section teaches the removed names to every reader and agent. Delete it.

## P2

11. Two ways to write a whole attribute (`:href="p.url"` and `href="{p.url}"`). Pick one rule for AGENTS.md: "`{}` when there is other text, `:` otherwise".
12. Site shared components break the app rules they teach: `lib/components/live-meter/live-meter.html:20` selects `[data-c="showcase"]` from outside (`components.md:25` forbids it), `lib/components/source-copy/source-copy.html:28` uses `document.getElementById`. Pass the stage through a slot or a prop, or mark these as site infrastructure in `site/README.md`.
13. The site mixes `<script>` in the `.html` with separate `index.js` (`routes/examples/state/index.js`, `components/index.js`). Keep one separate-file example and say why on the page.
14. Errors for several `{}` in one text node all point at the start of the node (`e3.html:21:9` three times). Point at each brace.

Site at 390 and 1440 after the migration: no horizontal overflow, no visible error block and no console error on the 14 pages. Not checked: interaction of each example.

## Top 5

1. Scope `<style>` automatically, or reject unscoped selectors (item 1).
2. Arity check for `@event="fn"`, with the `fn(event)` / `fn(task)` fix in the message (item 2).
3. Error for stale derived values (`let x = <reads state>` used in HTML) and for `data.` at the top level (items 3, 4).
4. Fix the five misleading messages: `{count}`, `@click="$fn"`, `:attr="{…}"`, `<if a > b>`, keyless `<for>` (item 5).
5. Make the starter page a small task list and document the per-item map pattern (items 8, 9).
