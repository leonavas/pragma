# Review: opus (1440 / 768 / 390, Playwright + source)

## P0
1. **Horizontal page scroll on mobile.** scrollWidth > viewport on /guide (390: 860px, 768: 872px), /examples/components, /data, /stores, /logs (390). Cause: `.prose code{white-space:nowrap}` (lib/theme.css:197) inside `.doc-section{display:grid}` (theme.css:229, implicit `auto` track). The whole section, showcase included, gets wider: on /examples/data at 390 the Copy button is off-screen. Fix: `.doc-section{grid-template-columns:minmax(0,1fr)}`, and `.prose code{white-space:normal;overflow-wrap:anywhere}` below 48rem. In guide.html "Under the hood", move the route JSON from inline `<code>` to `<pre>`.
2. **Showcase is empty in the prebuilt HTML, which causes layout shift and unstyled code.** Tabs (`<for>`), path, Copy label and code all bind to data fetched after boot. Measured CLS at 768: stores 0.094, logs 0.087, seo 0.072, state 0.070, overlays 0.069. Sources: `.showcase-meta` y682→724 (tab bar appears), `.showcase-live` y716→864 (code pane appears). The code first shows plain, then gets colored when highlight.js arrives. Without JS, and for crawlers, there is no code at all. This contradicts "prebuilt HTML" and "the code you read is the code that renders". Fix: `export const ssr = true` in each `examples/*/index.server.js`, and highlight in `load` (highlight.js runs on Node; send tokens). This puts colored code in the first HTML and drops the 15 kB client chunk. If SSR cannot expand a component `<for>` over `data`, that is a framework gap worth fixing: it is this site's main content. Minimum fix: `min-height` on `.showcase-bar` (2.625rem) and on the code pane, and fallback text `Copy` in `<span :text="$copyText">`.
3. **The 404 page is unstyled.** routes/_404.html has no `.wrap` and no padding, so the text touches the left edge and the header. It is the only off-brand page. Fix: `wrap pt-24`, then make it a terminal moment: `$ open /nope` → `no route matches /nope`, plus `<c-section-nav>`.
4. **Wrong claim:** examples/syntax/index.html:117 says `v-if`/`x-show` "still pass silently (BUG-S-3)". The compiler now rejects them (`foreign_directive`, validate.rs:218, `html/unknown-directive`). Fix: add `<li v-if="done">` to `_snippets/rejected/wrong.html`, show its real error, and delete the sentence.

## P1
5. **Other copy that does not match the docs.** components/index.html:12 says props arrive "always as strings", but components.md:30 and the prop-probe section on the same page say bare/"true"/"false" become booleans. The guide's `pragma --help` block has no `pragma upgrade` (cli lib.rs:21), and upgrade is not covered anywhere, yet the lede says "real output from pragma 0.1.0". Size figures disagree: landing/OG say "8.4 KB" for the starter, the guide build prints `/ 8.8 KB`, and the guide prose says "about 7.5 kB". Use one re-measured set.
6. **Internal notes leak into public copy.** "undocumented" appears 7 times (syntax:21, syntax:103, routing:61, routing:93, data:13, styling:24, styling:56), plus "Two things the docs do not mention" (data:25) and "(BUG-S-3)". A docs site cannot call its own facts undocumented. syntax:21 is also false: syntax.md now documents `:data-x`. Fix: add the facts to core/docs and drop the word.
7. **The landing has no live code next to a result.** That is the brief's main promise, but the landing has only static snippets. Put one `<c-showcase>` right after the hero: the `<for>` task list from the "Closed" principle, live, with `.done` toggling. The h2 "running on this page's runtime" (index.html) is then also true.
8. **The hero terminal feels slow.** It takes 5.45 s (terminal.css:89-101) to print "compiled in 3 ms". At 768/1440 it starts as a 22rem empty box. The sequence cannot happen: `pragma dev` blocks, then `pragma check` runs in the same shell, under "Real output". Fix: total ≤1.8 s (type at ~18 ms per character, output instant), start when in view, show `^C` after dev (or put dev last).
9. **No way to get pragma.** The guide's `cargo install --path core/cli/...` assumes a clone, but there is no repo URL, no `git clone` step and no GitHub link in the header or footer. Add them. The 9 guide terminal `<figure>`s repeat the same 3 class lists, which breaks the AGENTS.md "3+ repeats" rule. Make them a `<c-terminal title>` with a Copy button, and use it for the landing terminal style too.
10. **The code pane is too narrow.** At 1440 it is ≈520px of Martian Mono at 100% width, ≈60 columns, so most lines clip (bindings.html:2, catalog.html:4). Fix: `font-stretch:87.5%` on `.showcase-code`, and widen doc pages to ~84rem. Iframe demos (routing, data, logs, stores) need a stacked variant with the live pane full width. Now they are ~330px wide and show the full site header inside the frame (chrome inside chrome).
11. **The path truncates the wrong end:** "routes/examples/components/_snippets/counter/usage.h…" (showcase.css:73). Show the path relative to the section, or truncate from the start (`direction:rtl;text-align:left`).
12. **The skip link breaks (framework CSS bug).** On focus, `position` is `static`: a 1440×24 bar at y=0 that pushes the header down 24px. The generated CSS emits `.focus\:not-sr-only:focus{position:static}` after `.focus\:fixed:focus`, so the standard Tailwind skip-link pattern fails. Site fix: a `.skip-link` class in theme.css. Framework fix: sort `sr-only`/`not-sr-only` before the position utilities.
13. **Prefetch is too eager (framework, core/router.js:63).** Each visible link does `import(m)` for all its modules. With the 11-link sticky sidebar, one view of /examples/components fetched and evaluated the other 10 sections' HTML and ~40 modules, valibot and es-toolkit included. This contradicts "only the JS each page uses". Fix: on viewport, fetch only the HTML at idle. Load modules on hover/focus/touchstart with `<link rel=modulepreload>` (no eval). Skip when `navigator.connection.saveData` is set.
14. **SEO on the site itself.** 10 of 11 section pages have no description, canonical, og:url or og:title, so they inherit the landing's. Only /guide, /examples and /examples/seo set them, and B2/B4 are fixed (site/README "SEO" section is stale). robots.txt has no `Sitemap:` line, which the SEO page teaches. The `/examples/*/demo/*` routes are indexable duplicates: add `noindex`.
15. **No section navigation below 64rem** (doc.css:3 hides the aside) and no prev/next. Add a prev/next footer to each section, with `[` / `]` keys (delight), and a `<c-dropdown>` section switcher under the h1 on mobile.
16. **Prose before each example is dense.** /data and /components have 2–3 paragraphs (~150 words) before the showcase. Lead with one sentence, then the example, then rules and edge cases as a short list under it.

## P2
17. Tabs scroll with `scrollbar-width:none` and no cue that more exist (at 390, counter.css is hidden). Add a right-edge fade mask.
18. On mobile the hero buttons stack at two different widths. Use `w-full sm:w-auto`.
19. Many texts are 11px (showcase path and live label, landing terminal on mobile, `.label`); principles code is 11.5px. Set a 12px floor. The focus ring is 1px (theme.css:85); use 2px.
20. View transitions run for reduced-motion users (router.js has no matchMedia check) and cross-fade the sticky header. Give the header `view-transition-name` and skip the transition for reduced motion.
21. Copy: "Copied" is not announced (add `role=status`). Repeated clicks start overlapping timers (showcase.js `copy`).
22. Units: the CLI build table prints "KB" (cli report.rs:66) and the budget error prints "kB" (compiler lib.rs:711), both /1024. The site mixes "8.4 KB", "7.5 kB" and "15.0 kB". Use one unit.
23. `:style` is reported as `html/on-inline` (validate.rs:354), visible in the syntax "rejected" list line 7. It should be `html/style-inline`.
24. Copy: "behaviour" (principles.html:7, routing:61) vs "behavior" elsewhere (og:locale en_US). The footer says "no framework" but the modal says "A framework for web apps". Say "no virtual DOM, no client framework".
25. The landing has a double rule (last `.principle` border plus the sections `border-t`, 7rem apart). Keep one.
26. The dropdown has no arrow-key navigation (only Tab), which the menu pattern expects.

## Delight that fits "fast and robust" (missing now)
- **Show updates toggle in the live pane.** Flash the nodes the runtime touched after each action (rects in state → `<for>` overlay). This shows "only bindings, never a DOM rebuild" visually.
- **Live perf line in the footer:** "this page: 8.1 kB JS · interactive in 38 ms · CLS 0", from the Performance API. Proof, not a claim.
- **Terminal beat:** `pragma check` prints one red error, the file is edited, check runs again → `no problems found`. This shows the agent loop in 1 s.

## Top 5 changes that most raise quality
1. Put showcase code in the first HTML (ssr + highlight in `load`) or at least reserve its height. This removes CLS 0.05–0.09, the unstyled-code flash and empty no-JS/crawler pages.
2. Fix mobile overflow: `minmax(0,1fr)` on `.doc-section` and wrapping inline `code` (guide plus 4 example pages at 390).
3. Correctness and copy sweep: the v-if claim, "always as strings", the `--help` block without upgrade, one set of size numbers, and no "undocumented" or BUG ids.
4. Landing: one live showcase under the hero, a terminal of ≤1.8 s, a repo link and an install command with a copy button.
5. Navigation and robustness: prev/next plus a mobile section switcher, an on-brand 404, per-page SEO tags, and a lazier framework prefetch (no module eval for links only in view).

## Round 2

Method: Playwright on all 15 pages at 390/768/1440 (overflow, CLS, console). Clicked all 220 buttons in the live results: the only errors are the intentional demo failures (logs, data `?fail`, 422, 404). Also tested the frame, the switcher and the highlight ranges.

1. RESOLVED: scrollWidth equals the viewport on every page at all 3 widths.
2. RESOLVED: on mobile, a sticky "Examples / Section ▾" switcher (works, marks the current section, anchors stay clear of it), a contents bar on the guide, prev/next on each section, and a current-page underline in the header.
3. RESOLVED: `ssr = true` plus server-side marks put ~250 code lines into the first HTML of /components. CLS ≤0.002 on all pages and widths (was 0.05–0.09). The Highlight API colors stay after tab switch, actions and back/forward. Without JS the code shows uncolored, which is fine.
4. RESOLVED: `<c-frame>` sits full width under the code. The address updates on in-frame router navigation ("router · 3 ms"). Reload and Open work. Demo pages have no site header or footer and are `noindex`.
5. PARTIAL: showcase code is 13px at 1440 but 12px at 390. guide.html:327 (route JSON `<pre>` inside `.prose`) computes to 9.36px because `.prose code` (0.78em) applies on top of `text-xs`. Fix: `.prose pre code{font-size:inherit;padding:0;background:none}`.
6. RESOLVED: a grep finds no "undocumented", "do not mention" or BUG ids in routes/ or lib/.
7. PARTIAL: v-if, prop types, `upgrade` and the 8.8/7.6 figures are fixed. But public/og.png still says "8.4 KB JS on the starter page", and every link preview shows it. Fix: regenerate it with 8.8 KB, or drop the number.
8. PARTIAL: header/footer GitHub links and the clone + `cargo install` steps with Copy are in place. But `gh repo view leonavas/pragma` reports **PRIVATE**, so every visitor gets a 404 on the GitHub links, the landing #install and the guide clone. See new P0.
9. RESOLVED: the 404 page is styled, shows `GET <path>` → `404 no route matches`, and is `noindex`.
10. RESOLVED: the live task-list showcase is under the hero. The terminal finishes in ≈1.7 s (`sibling-index()` × 0.14 s; without support it shows all lines at once) and the sequence is honest (cd → check → build).
11. RESOLVED: lap ids come from a counter that only goes up (stopwatch.js `lapCount`).
12. RESOLVED: components and data now show the demo before the long explanation.
13. PARTIAL: tabs wrap, mobile source is capped at 272px with "Show all N lines", and soft wrap has a hanging indent. Still open: dead space under short code on desktop (components counter, usage.html: 1 line, ~170px empty). Soft wrap also splits tokens at hyphens where there is room (1440: `</p-`|`counter>`; 768: `:aria-`|`pressed`). Fix: `white-space:pre` from 64rem up, soft wrap only below. Size the source pane to its content and center the live result.
14. RESOLVED: `--skeleton-bg: var(--color-raised)`.

New issues
- P0 (launch): the repo github.com/leonavas/pragma is private. All install paths are dead links for the public: routes/_layout.html (header and footer GitHub), the landing `#install` section, and the guide.html Install clone. Fix: make the repo public before the site goes live, or remove the links and the clone step until it is.
- P2: the live-result counter shows "4 updates" at page load with no interaction (landing showcase), because the observer counts the hydration mutations. Fix: ignore mutations until the first interaction (showcase.js `mount`).
- P2: `.prose code` now wraps at every width, so it breaks inside tokens on desktop (`<c-`|`counter>`, `.is-`|`pending`). Fix: keep `white-space:nowrap` from 48rem up.
