# Review: fable

Checked at 1440, 768 and 390 in the Sheltie browser (dev server, 2026-11-18). Measured values are marked "measured". Production build was not run (read-only), so payload numbers are dev numbers.

## P0

1. **Layout shift on every example page** — `lib/components/showcase/showcase.html:17-30`, `showcase.css:108-116`. Each showcase goes 4-line skeleton -> plain code -> colored code. Measured CLS on `/examples/syntax#lists`: 1.83. Fix: give `.showcase-skeleton` and `.showcase-code` the same fixed `block-size` (for example 20rem, scroll inside), so the box never changes size. Better: tokenize in `routes/examples/_source.server.js` (Node has highlight.js) and send tokens in `load`; this removes the 15 kB gzip client chunk and the color flash.
2. **Horizontal page scroll at 390** — measured `scrollWidth`: `/guide` 860, `/examples/data` 489, `/examples/logs` 444, `/examples/stores` 402, `/examples/components` 396 (viewport 375). Cause: `.doc-section { display: grid }` (`lib/theme.css:229`) has no `grid-template-columns: minmax(0, 1fr)`, and `.prose code { white-space: nowrap }` (`theme.css:197`). Fix: add the `minmax(0,1fr)` column, set `overflow-wrap: anywhere; white-space: normal` on `.prose code`, and `min-width: 0` on `.env-vars-row` children in `logs/_components/env-vars/env-vars.css`.
3. **404 page is the unstyled starter** — `routes/_404.html:3-8`. No `.wrap`, text touches the left edge, sans heading, `text-primary-600`. Fix: wrap in `.wrap`, use the site h1 style, show the requested path as a terminal line (`$ GET /nope` -> `404 no route matches`), add links to Guide and Examples.
4. **Routing and data demos hide the URL** — `routes/examples/routing/index.html:28,39,49,60,71,81,91`, `data/index.html:12,35,45`, `stores/index.html:27`. The iframe shows the full site header again inside a ~300 px column, and the reader cannot see the address change, which is the point of a routing demo. Fix: add a small frame component with an address bar (current path + query), back/reload buttons and "open in new tab"; stack these showcases (code on top, frame full width below, 100% width, 26rem high).
5. **No section navigation below 1024 px** — `lib/components/doc/doc.css:6`. The aside is `display: none`; pages are 5k to 15k px long on a phone with no way to move to another section or heading. Fix: a `<details>` "Sections" bar under the header on small screens, and previous/next section links at the end of `<c-doc>`.

## P1

6. **Code column cuts the important part** — `showcase.css:89-95`. At 1440 most HTML lines overflow and the directives sit off-screen to the right; at 390 only ~35 characters show. Fix: soft wrap with hanging indent (`white-space: pre-wrap; padding-left` + negative `text-indent`) below 64rem, and a "wrap" toggle on desktop; raise the code column to `1.7fr` or use the stacked variant from item 4 for wide results.
7. **Dead space under short code** — `showcase.css:78` (`max-height: 34rem`). On `/examples/seo` the result column is taller than 34rem, so the horizontal scrollbar floats in the middle of the panel. Fix: remove `max-height` on desktop when the grid row is taller (`max-height: none; min-height: 0` with the row driven by a fixed showcase height), or cap the live column too.
8. **Internal audit words in public copy** — "undocumented" / "Not in the docs" / "the docs do not mention" at `syntax/index.html:21,103`, `stores/index.html:27`, `routing/index.html:61,93`, `data/index.html:13,25`, `styling/index.html:24,56`; "(BUG-S-3)" at `syntax/index.html:117`. This reads as "the framework is not stable". Fix: add these facts to `core/docs/*` and delete the words; replace the bug id with "not checked yet in v0".
9. **Numbers disagree** — hero terminal and principles say `/ 8.4 KB` (`_components/terminal/terminal.html:16`, principles); guide says `/ 8.8 KB` and "about 7.5 kB" (`guide.html`, build and budgets sections); styling demo says `pragma build 412 ms` (`styling/_components/build-card/build-card.html:4`) while the hero says 7 ms. Units mix `KB` and `kB`. Fix: one measured set of numbers, one unit (`kB`), and a build time that agrees.
10. **Guide `pragma --help` is out of date** — `routes/guide.html` install block has no `pragma upgrade`; `core/docs/cli.md:10` has it. Fix: paste the current help; add `upgrade` to the guide text.
11. **Header does not mark the current area** — `routes/_layout.html:28-29`. On `/examples/<section>` no header link has `aria-current` (measured: 0). Fix: bind the state from a small nav component (`:aria-current` from a getter on the path prefix), or add an "Examples / Routing" breadcrumb above the h1 in `<c-doc>`.
12. **Hero terminal takes 5.5 s and starts empty** — `terminal.css:80-92`. A 22rem empty box is the first impression (also what a screenshot or slow device sees), and a slow animation argues against "7 ms". Fix: first line visible at 0 s, full run in <= 2.5 s, final frame as the non-animated default (animate only with `@media (prefers-reduced-motion: no-preference)`).
13. **Landing has no live example and no SSR/SEO proof** — `routes/index.html`. All four principle blocks are static text at 11 px. Fix: put one real `<c-showcase>` (counter) under the hero, and a "this page" strip with real values (`performance` API: JS transferred, navigation time; link to `/examples/seo` with the preview card). This is the best place for delight that fits "fast": a status line like `route / · 8.4 kB js · swapped in 14 ms`, updated after each client navigation.
14. **Guestbook accepts unbounded names and has no rate limit** — `routes/examples/data/index.server.js:32-36`. Text from any visitor shows to all visitors of a public site. Fix: cap name at 40 characters, keep entries per visitor (cookie or session id) or reset on a timer, and say so in the caption.
15. **Per-page SEO is missing on the pages that teach it** — only `/`, `/guide` and `/examples/seo` set `description`, `canonical`, `og:url`. Ten example pages and `/examples` inherit the home description. Fix: add the four tags to each page (the SEO page explains why).
16. **Tap targets and small type** — header and footer `.nav-link` are 14 px high (measured), `.showcase-copy` 30 px; 11 px mono (`0.6875rem`) in terminal, labels, captions and principle code. Fix: `min-height: 2.75rem` with inline-flex on nav links, 12 px minimum for mono text.
17. **Dev only: pages with many iframes hang the tab** — after `/examples/routing` (7 demo frames, each with an `EventSource`), the next navigation timed out at 30 s while `curl` answered in 1 ms. Same cause as BUG-S-9. Framework fix: do not open `/_pragma/reload` when `window !== window.top` (the parent reload is enough).

## P2

18. `_components/hero/hero.css:19` — `line-height: 0.98` at 4.25rem: descenders of "apps," touch "rules". Use 1.04.
19. `lib/components/section-nav/section-nav.css:16` — transition on `background-color`, but hover changes only the name color. Add a real row hover (surface background, `->` that moves 4 px) or remove the transition.
20. `showcase.html:4-8` — a showcase with one file still shows a tab strip; hide the strip when `$tabs.length` is 1 (getter). Tabs are toggle buttons with `aria-pressed`; use `role="tablist"` + arrow keys, and announce "Copied" with `role="status"`.
21. `showcase.css:146-151` — the "Live result" dot pulses forever, 8 times per page. Fix: no idle animation; flash the dot once when the stage re-renders. It also shows the reader that only bindings changed.
22. `lib/components/modal/modal.html` — close button uses `rounded-lg`; the site is square (2 px). Remove it; same check for alert and dropdown.
23. `routes/guide.html` — the guide does not use `<c-doc>`: different sidebar width, h1 size and top padding from the example pages; no copy button on command blocks; the "On this page" list has no active state. Fix: one shell for both, copy on each `$` line.
24. `lib/theme.css:84-87` — focus ring is 1 px; on the green primary button it almost disappears. Use 2 px and `outline-offset: 2px`, ink color on `.btn-primary`.
25. `state/index.html:58,60` — text says "same microtask batch", caption says "in a frame". The runtime uses `queueMicrotask` (`core/runtime/page.js:22`). Use "microtask" in both.
26. `<c-doc lede>` renders tags such as `<head>` as plain text with no code style (`doc.html:9`). Use a `lede` slot so inline `<code>` works.
27. `site/README.md:23,33,160-166` — says the guide and sections are TODO and that per-page head is not possible. Both are false now.
28. `routes/_layout.html:13` — `og:image` uses the placeholder origin `https://pragma.dev`. Replace before publish.
29. Example pages have no heading list. Add an "On this page" list (h2 anchors) in the aside under the section list, and a `#` link on h2 hover.

## Top 5 changes that most raise quality

1. Fixed-height showcase with server-side tokens: no layout shift, no color flash, 15 kB less JS (item 1).
2. Stop the horizontal scroll at 390 and wrap code on small screens (items 2, 6).
3. A frame component with address bar for all iframe demos, stacked full width (item 4).
4. Mobile section navigation, previous/next links, header current state and a designed 404 (items 3, 5, 11).
5. One live showcase and a real "this page" performance status line on the landing, with a 2.5 s terminal and one set of numbers (items 9, 12, 13).

## Round 2

Checked 2026-11-19 on the dev server at 390, 768 and 1440. "Measured" = value read in the browser. Not checked: production build, Firefox/Safari, no-JS rendering in a real browser (only the first HTML with `curl`).

CONSENSUS items:

1. RESOLVED — measured `scrollWidth == clientWidth` on all 15 pages at 390 and at 768.
2. RESOLVED — sticky "Examples / <section>" switcher below 1024 px, pager at the end of `<c-doc>`, header "Examples" has the current state, guide has `<c-page-contents>`. The guide contents at 390 were not opened.
3. RESOLVED — measured CLS 0 on `/examples/syntax#lists` (was 1.83), 0.002 on `/examples/seo`, 0.001 on `/`. Code is in the first HTML (31 `code-line` in `curl /examples/components`); no client highlight chunk loads; colors stay after a tab switch and a state change (1012 ranges, 0 detached).
4. RESOLVED — `<c-frame>`: address bar follows in-frame navigation (measured `/examples/routing/demo` -> `/demo/closed-syntax`), reload and open buttons, site header and footer hidden inside, full-width stage.
5. PARTIAL — showcase code is 13 px from 768 up, but 12 px below 48rem (`lib/components/showcase/showcase.css:122`); the agreed minimum was 13 px. Prose code is 13.3 px.
6. RESOLVED — no "undocumented", "not in the docs" or bug id left in the files checked (syntax, components, stores, routing, data, styling, guide, landing).
7. RESOLVED — one set of numbers (8.8 / 7.6 / 4.1, 6 ms) in hero, principles and guide; `412 ms` is gone; guide has `pragma upgrade`; `syntax/index.html:117` now says `v-if`/`x-show` fail. Units still mix `KB` (CLI output) and `kB` (prose): acceptable if the CLI prints `KB`.
8. RESOLVED — GitHub link in header and footer, `git clone` + `cargo install` with copy on the landing and in the guide.
9. RESOLVED — 404 has `.wrap`, site heading, two actions and the `$ GET /nope` -> `404 no route matches` block.
10. RESOLVED — live task-list showcase under the hero; terminal shows the full run at first paint in the screenshot; sequence is honest (no fake `pragma dev`).
11. RESOLVED — lap ids come from a counter (`state/_components/stopwatch/stopwatch.js:35-36`).
12. RESOLVED on components (demo first, then explanation). Data page not opened.
13. PARTIAL — tabs wrap, code soft-wraps with hanging indent, long files have "Show all N lines" on mobile. Dead space remains on desktop: on `/examples/seo` the code box is capped at 512 px while its column is 758 and 877 px high (measured), so 250-360 px stay empty under a scrolling code box.
14. RESOLVED by code read — `--skeleton-bg: var(--color-raised)` in `lib/theme.css:50`; the catalog demo was not opened.

New issues from the polish:

- P1 — `lib/components/showcase/showcase.css:45`: `max-height: 32rem` still cuts code that has free room beside a taller live column (item 13). Fix: inside `@media (min-width: 80rem)` set `.showcase-code { max-height: none; flex: 1 1 0; min-height: 20rem; }` so the code fills the row height and scrolls only when the row is shorter.
- P2 — `showcase.css:64`: `overflow-wrap: anywhere` lets the browser break at hyphens inside tokens (`aria-` / `label="Decrease"` on `/examples/components`). Fix: add `word-break: normal; hyphens: none` and prefer breaks at spaces (`overflow-wrap: break-word`).
- P2 — `showcase.css:122`: raise mobile code to `0.8125rem` (item 5).

No new P0.
