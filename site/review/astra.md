# Council review — astra

Manual independent review of the running site at 1440×900, 768×1024 and 390×844. All main routes inspected; source compared with `core/docs/`. Exercised counter, source tabs/copy, modal/Esc, dropdown sorting, cart, lazy package loading, build state, client logs, invalid server action, routing and stopwatch. No build or automated test suite run. Only this report was edited; all review tabs closed.
The worktree changed during inspection. Code sizes and Data/Logs/Guide overflow were checked again near completion. Paths below are relative to `site/`, except explicit `core/runtime/` references.

## P1

1. **`routes/examples/state/_components/stopwatch/stopwatch.js:33` — fifth lap breaks rendering.** Start, then click Lap five times: `laps.length + 1` repeats id `4` because the array is capped at three. Chromium reports `duplicate key in <for lap in state.laps>` every 31 ms afterward. Use a separate monotonic lap counter, independent of the displayed array length. This is an example bug, not a framework failure.

2. **`lib/theme.css:197,229`; `/guide`, `/examples/data`, `/examples/logs` — page-wide horizontal overflow.** At 390 px, document widths were Guide **860**, Data **489**, Logs **444**, Components **396**; Guide also reached **872** at 768 px. Inline code cannot wrap, and grid children retain their minimum content width. Set section tracks to `minmax(0,1fr)`, constrain children with `min-width:0`, allow long inline code to wrap, and move long JSON/signatures into scrollable blocks. Keep horizontal scrolling inside code panels.

3. **`lib/theme.css:94`; `lib/components/showcase/showcase.css:76`; `lib/components/terminal/terminal.css:27` — the central product demonstration is too small to read.** Global `code { font-size:.86em }` compounds the pre size: showcase code is **10.32 px**, and mobile Guide terminals are **9.46 px**. Set `pre code { font-size:inherit }`; use a deliberate 13–14 px code size and retain local scrolling. Do not reduce type to fit the desktop split.

4. **`routes/guide.html:31` — installation starts inside a repository the visitor does not have.** The first command is `cargo install --path core/cli/crates/pragma-cli`, but there is no repository link, clone command or explicit checkout step. Add the actual repository URL, prerequisites and complete commands, followed by one small edit-and-run example. Keep the full CLI transcript as secondary detail.

## P2

5. **`lib/components/doc/doc.css:6`; `routes/guide.html:15` — navigation disappears below 1024 px.** The examples lose their section list; the Guide loses its contents list, despite being roughly ten screens long at 768 px. Add a compact mobile/tablet section chooser and an “On this page” disclosure with anchors; retain desktop navigation. Add heading permalinks so a specific example can be shared.

6. **`routes/examples/routing/index.html:33`; `routes/examples/data/index.html:17`; `.showcase-stage` — live routes are squeezed into miniature websites.** At 1440 px the iframe content is only **300 px** wide and 384 px high, with a second site header, inner navigation and its own scrollbar. Use a wider or full-width variant for route examples, reduce surrounding stage padding, and provide an explicit current URL, Open route and Reload control. Preserve the full layout in the example that teaches layouts; use a compact embedded presentation for the others.

7. **`lib/components/doc/doc.html`; `routes/examples/data/index.html:10`; `routes/examples/components/index.html:5` — demonstrations arrive too late.** Components' first interaction is below the first 390 px viewport; Data puts three dense technical paragraphs before its first example. Lead each section with one rule and the working example, then put edge cases and implementation detail below or in a disclosure. The current repeated title/prose/panel rhythm makes the catalogue feel slower than the runtime.

8. **`lib/components/showcase/showcase.css:16,78` — source discovery and comparison require too much scrolling.** Mobile file tabs clip while their scrollbar is hidden; long source panels reach 544 px before the live result; changing files can push later content far down. Add a visible overflow cue for tabs, and a compact Code/Result switch or bounded source view on phones. Keep the selected file and Copy action visible while reading.

9. **`lib/theme.css`; `routes/examples/data/demo/catalog.html` — loading states flash light grey in the dark design.** The catalog's three skeletons render almost white, unlike the dark source skeletons. The base skeleton defaults to neutral-200 unless the theme supplies the dark value. Set the site's `--skeleton-bg` to a dark raised token and keep the loaded list and skeleton dimensions close.

10. **`routes/_components/cli-session/cli-session.css:80`; `lib/components/showcase/showcase.css:146` — motion spends attention on simulated waiting.** The hero starts with a blank terminal and withholds the build result until 5.1 s; every example pulses indefinitely. Show useful output immediately, shorten the initial reveal, and keep the longer sequence behind Replay. Replace repeated ambient pulses with brief feedback after a real action, such as a value change or successful copy. Retain reduced-motion rules.

11. **`core/runtime/router.js:125` — route transitions ignore reduced motion.** The router always calls `startViewTransition` when available, while the site disables only component animations. Bypass view transitions for `prefers-reduced-motion: reduce`, or disable the corresponding pseudo-element animations. This is a framework-level accessibility gap.

12. **`core/runtime/router.js:56,77`; desktop `.doc-aside` — idle prefetch loads the whole catalogue.** A fresh Components visit produced **74 requests / 738,951 transferred bytes** in dev, including modules from every other example section, because every sidebar link is visible. These are dev measurements, not a production bundle claim. Add a prefetch concurrency/budget policy and a way to suppress viewport prefetch on dense navigation; retain hover/focus prefetch and honor data-saving connections.

13. **`lib/components/showcase/showcase.html`; `showcase.js:42` — copy confirmation is only a changing button label.** Copy works, but there is no persistent status/live region; failure says “Select and copy” and clears after 1.6 s without selecting anything. Add a polite status region, preserve a readable failure instruction, and offer a deliberate Select source fallback. Give Guide command blocks a command-only Copy action too.

14. **`routes/examples/routing/index.html:104` — router explanation is stale.** It says navigation swaps only main and title, while `core/runtime/router.js` also replaces page metadata and links through `syncHead`. Include description, canonical and Open Graph updates, linking to the SEO example, so the routing and SEO explanations agree.

15. **`routes/_layout.html:13`; page canonicals and `public/sitemap.xml` — production metadata still uses a placeholder origin.** `https://pragma.dev` is explicitly identified as a placeholder in the README. Set the actual deployment origin consistently before publishing; otherwise canonical URLs and social images advertise the wrong host. Keep this distinct from the functioning SSR head example.

Observed a shared-localhost navigation stall during the review. The current CLI source now uses WebSocket reload instead of EventSource, so the earlier connection-limit symptom is a verification item for that concurrent framework change, not a new unfixed finding here.

## Top 5 changes that most raise quality

1. Fix the stopwatch's duplicate lap keys so the live catalogue remains reliable after repeated use.
2. Remove document-level overflow at phone/tablet widths; confine horizontal scrolling to code.
3. Raise rendered code to 13–14 px and remove compounded `em` scaling.
4. Add mobile section navigation and put each working example before its detailed explanation.
5. Give route demos a wider presentation with visible URL, Open and Reload controls.

## Round 2

Verified the running site at 390×844, 768×1024 and 1440×900; inspected server HTML and current source. No builds or automated suites run. All review tabs closed. Numbers below match CONSENSUS items 1–14.

1. **RESOLVED** — Guide, Data, Logs, Stores and Components have no document overflow at all three widths: scrollWidth equals clientWidth (375 / 753 / 1425 px). Long code stays inside its panel.
2. **RESOLVED** — Mobile section dropdown opens/closes correctly, current section and top navigation are marked, prev/next links point to adjacent sections, and Guide contents remain available below 1024 px.
3. **PARTIAL** — Source is present in the HTTP response (Components: 226 code lines), and measured initial CLS fell to 0.00056 on Components / 0 on landing. Server token ranges, the retained client import and README measurements are implemented; colors still require `paint()` in `showcase.js:23,98`, so plain-to-colored rendering remains possible and no-JS/unsupported-API output stays uncolored. Emit token styling in SSR to finish this item.
4. **RESOLVED** — Frames use the full stage (852 px at desktop), hide duplicate site chrome, and show the current URL. Navigating to hello-pragma updated both address and Open destination; Reload retained that route. The sm height renders at 288 px as intended.
5. **PARTIAL** — Compounded `em` scaling is gone; showcase and terminal code reach 13 px at 768/1440, but both remain 12 px at 390 (`showcase.css:122`, `terminal.css:27`). Raise the mobile values to the agreed 13–14 px floor.
6. **RESOLVED** — Public route copy no longer contains the audited “undocumented”, “not in the docs”, “docs do not mention” or BUG-id wording.
7. **RESOLVED** — Foreign directives are described as rejected, boolean/string prop conversion is explicit, help includes upgrade, and starter/build figures agree (8.8 kB JS, 4.1 kB CSS, 6 ms build); the styling card now shows the same 6 ms figure.
8. **RESOLVED** — Landing and Guide include the repository, clone/change-directory/install steps and working Copy feedback; GitHub is linked from the desktop header and footer.
9. **RESOLVED** — `/nope` returns HTTP 404 and displays the styled request/error transcript with usable home/examples links at 390 px.
10. **RESOLVED** — A working task-list showcase follows the hero; checking a task updates its status. Terminal content is present immediately and its final line delay is 1.54 s, within the two-second target; landing CLS measured 0.
11. **RESOLVED** — Seven successive laps produced ids 7/6/5 with no console errors; stopping preserved the elapsed time. Keys now use the independent lapCount.
12. **RESOLVED** — Components and Data lead with a short rule and live showcase; detailed ownership, loader and SSR explanations follow the demonstration.
13. **RESOLVED** — File tabs wrap instead of clipping, narrow code soft-wraps with hanging indentation, and long mobile sources cap at 272 px with an operable Show all/Show fewer control. Short source blocks no longer reserve the old 34rem height.
14. **RESOLVED** — Reloading the catalog showed dark skeletons with computed background `oklch(0.225 0.008 255)`, matching the raised token at phone and desktop widths.

New P0/P1 issues introduced by the polish: **none found**.
Counts: **12 RESOLVED / 2 PARTIAL / 0 NOT**.
