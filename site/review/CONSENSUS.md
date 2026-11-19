# Council consensus (astra · fable · opus)

Source reviews: `astra.md`, `fable.md`, `opus.md`. Items are ordered by agreement and impact. Each has an owner for the polish pass.

## Agreed by all 3

1. **Horizontal page scroll at 390/768** (`/guide`, data, logs, stores, components). `.doc-section` grid without `minmax(0,1fr)` plus `.prose code { white-space: nowrap }`. Only code blocks may scroll sideways. Owner: B.
2. **No section navigation below 1024px.** Add a compact section switcher (sticky `<details>` "Sections" / on-page contents) for examples and guide, plus prev/next links at the end of each section page and a current state in the header. Owner: B.
3. **Showcase layout shift + color flash + empty without JS** (CLS up to 1.83). Tokenize the code on the server in `load` (highlight.js on the server), render it in the prebuilt/SSR HTML, and reserve the height. Keep a real client-side npm import for the tree-shaking story (e.g. lazy re-highlight on tab switch, or a clearly labelled client demo in /examples/packages) and update the README numbers. Owner: A.
4. **Routing/data iframe demos are miniature sites** with a hidden URL. New `<c-frame src title height>` component: address bar with the current URL (updates on in-frame navigation), Open and Reload buttons, full-width stage under the code. Demo routes drop the site chrome (use a minimal demo layout if pragma allows nested `_layout.html`, otherwise hide header/footer for `*/demo/*`). Owner: A builds it, C adopts it.

## Agreed by 2

5. **Code too small** (10.3px computed: `code { .86em }` compounds). 13–14px in showcase, terminal and prose blocks; remove compounded `em`. Owner: A.
6. **Internal audit words in public copy:** "undocumented", "Not in the docs", "the docs do not mention", BUG ids. Rewrite as plain feature descriptions. Owner: C.
7. **Wrong or stale claims:** `v-if`/`x-show` now rejected (syntax:117), props "always as strings" (components:12), `--help` block without `upgrade` (guide), hero/principles/guide/styling numbers disagree (8.4 / 8.8 / 7.5 kB, 412 ms): one measured set. Owner: C (sections + guide), B (landing numbers).
8. **No way to get pragma:** repo `https://github.com/leonavas/pragma`, a `git clone` + `cargo install` step with Copy, GitHub link in header/footer. Owner: B (header/footer/landing), C (guide).
9. **Unstyled 404:** on-brand terminal moment (`$ GET /nope` → `404 no route matches /nope`), links back. Owner: B.
10. **Landing lacks a live code+result:** one `<c-showcase>` right under the hero (the `<for>` task list from the Closed principle, live). Hero terminal ≤ 2s total, no empty 22rem box at first paint, honest sequence. Owner: B.

## Single reviewer, accepted

11. Stopwatch duplicate lap keys (`state/_components/stopwatch/stopwatch.js:33`). Owner: C.
12. Demo before long explanation (components, data). Owner: C.
13. Mobile file tabs clip with hidden scrollbar; long source panel pushes the result far down (cap source height on mobile, fade + "expand"). Soft-wrap code on small screens with hanging indent. Dead space under short code on desktop (`max-height: 34rem`). Owner: A.
14. Light grey skeletons in the dark theme (catalog demo). Owner: A (theme token) / C (demo usage).
15. Framework prefetch evaluates modules of every link in view: core bug BUG-S-10, sent to a core agent.

## File ownership for the polish pass

- **A (showcase & frames):** `lib/components/showcase/**`, new `lib/components/frame/**`, code-related rules in `lib/theme.css` (code size, skeleton token), `utils/highlight.js`, `utils/code-lines.js`, `routes/examples/_source.server.js`, every section's `index.server.js` only if the load contract changes.
- **B (shell & landing):** `routes/_layout.html`, `routes/_404.html`, `routes/index.html`, `routes/_components/**`, `lib/components/doc/**`, `lib/components/section-nav/**`, layout rules in `lib/theme.css` (overflow, grid).
- **C (content):** `routes/examples/<section>/**` (except `index.server.js` contract changes coordinated with A), `routes/guide.html`.
