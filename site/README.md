# pragma site

Landing page, guide and "every example" site for pragma, built with pragma itself (dogfooding).
It follows `AGENTS.md` and `core/docs/` like any pragma app; `pragma check --strict` must stay at
zero errors and zero warnings.

## Run

```sh
npm i --ignore-scripts        # highlight.js only
pragma dev --port 5181        # http://127.0.0.1:5181 (starts Node for the .server.js loaders)
pragma check --strict
pragma build && pragma serve  # production build; run from this folder (see "Source files" below)
```

## Layout

```
routes/
  _layout.html                 header, footer and the site-wide <head> defaults (SEO/OGP)
  index.html                   landing: <p-audience>, <p-hero>, speed, install, callout, why, showcase, principles, sections
  agents.html                  /agents: the prompt, install, files for agents, the WebMCP tools
  _components/hero|principles|prompt/   landing-only pieces (<p-...>); prompt is the hero box with the agent prompt and its Copy button (same text as /agents)
  _components/audience/        Human / Agent switch (real links) + the "agents: read /llms-full.txt" note
  _components/docs-tool/       WebMCP tool get_pragma_docs (fetches public/docs/<topic>.md)
  _components/setup-tool/      WebMCP tool get_setup_instructions (commands + prompt for an app description)
  guide.html                   "how to use": install to deploy, real CLI output in <c-terminal>
  examples/
    index.html                 index of all sections
    _source.server.js          shared loader helper: reads example files from disk
    components/                REFERENCE section (done)
      index.html               the page
      index.server.js          load(): which files each showcase shows
      _components/counter/     the live example component (<p-counter>)
      _snippets/counter/usage.html   extra file shown as a tab (usage)
    syntax/ state/ stores/ routing/ data/ overlays/ styling/ packages/ logs/ seo/
      index.html               skeleton with a TODO list
lib/
  theme.css                    tokens (incl. --code-* colors, --skeleton-bg), fonts, global classes
  components/                  every component is one .html (<script>, markup, <style>)
    doc/                       <c-doc heading lede>: page shell with sidebar + h1 + lede
    section-nav/               <c-section-nav>: the single list of sections (class="compact" in sidebars)
    showcase/                  <c-showcase :source caption>: server-rendered source tabs, copy, highlight ranges,
                               update counter and the live result, in one scripted component
    frame/                     <c-frame src title height>: a demo route in an iframe with a live address bar
    demo-page/                 <c-demo-page>: root of a demo route; hides the site header and footer
    terminal/                  <c-terminal title copy>: static terminal window for real CLI output (see below)
    modal/ alert/ dropdown/    the starter overlays, restyled
utils/
  agent-docs/                  generate.server.js + markdown templates for public/*.md and llms*.txt (see below)
  highlight.js                 highlight.js wrapper: markLines() on the server, highlightLines() for token spans
  code-lines.js                codeKinds (shared with the client) and plainLines()
public/
  fonts/                       Inter + JetBrains Mono (variable woff2, latin subset from Fontsource, OFL)
  og.png                       1200x630 Open Graph image (source: design/og/og.html)
  robots.txt, sitemap.xml      sitemap lists every indexable page; update it when you add one
  index.md, agents.md          the landing and /agents in markdown (generated)
  llms.txt, llms-full.txt      llmstxt.org index and every doc in one file (generated)
  docs/<topic>.md              one doc per file: rules (core/starter/AGENTS.md) + core/docs/*.md (generated)
```

## Agent-first: markdown, llms.txt and WebMCP

Same truth, different format: the markdown files carry the same content as the HTML, never extra
text for agents. The landing and `/agents` start with the Human / Agent switch (plain `<a>` links)
and a note pointing agents to `/llms-full.txt`; their `<head>` has
`<link rel="alternate" type="text/markdown">` to their `.md`. The server answers `/` and `/agents`
with `public/index.md` / `public/agents.md` when `Accept` prefers `text/markdown` (SPEC §11).

Regenerate after changing `core/docs/`, `core/starter/AGENTS.md`, the landing or `/agents`:

```sh
npm run agent-docs     # node utils/agent-docs/generate.server.js
```

It reads the docs from the repo's `core/` (not `site/core/`, which lags until `pragma upgrade`),
fills the templates in `utils/agent-docs/` (`{{origin}}`, `{{docs}}`, `{{all-docs}}`,
`{{source <file>}}`, `{{tools <file>...}}`: tool names and params are read from the forms) and
writes `public/`. It exits 1 and prints `drift:` lines when a doc is missing from its topic list,
when a markdown heading is not in the page sources, or when `docs-tool.html` options or the
`/docs/*.md` links in `agents.html` differ from the topics. Copy edits on the landing or `/agents`
go in both the `.html` and the template.

WebMCP is on (`"webmcp": true`). Tools: `get_setup_instructions` and `get_pragma_docs` on `/` and
`/agents`, `add_task` on `/` (task-list), `sign_guestbook` on `/examples/data`. Every other form
has `notool`.

## Design tokens

Light and dark. `lib/theme.css` defines the light values in `:root` and the dark ones in
`@media (prefers-color-scheme: dark)`, so the site follows the device with no script (the site does
not import `.pragma/theme.js`, so the compiler injects no `data-theme` boot script and `dark:`
utilities stay off). Tokens: `ground`, `sunken`, `surface`, `raised`,
`line`, `line-strong`, `ink`, `muted`, `faint`, `accent` (phosphor lime fill in both modes), `accent-text` (green text: deep in light so it reads on white, lime in dark),
`accent-ink`, `error` and `warning`; each becomes a utility (`bg-ground`, `text-muted`, `text-error`...).
Use `error`/`warning` for status text, never a palette shade (`danger-300` is unreadable on light).
`primary-*` is remapped to the accent. Fonts: `font-sans` = Inter (headings, UI, prose), `font-mono` =
JetBrains Mono (code, CLI output and data only). The Pragma Reto brand font now lives only in the logo
SVG (source: `design/font/build_reto.py`, see
`design/README.md`). The header logo is the Pipe mark (a bar and a chevron) and the word "pragma" in
the same shapes, as inline SVG in `routes/_layout.html`; the favicon is the mark. Global classes: `.wrap`, `.btn` (`.btn-primary`, `.btn-small`), `.nav-link`,
`.prose`, `.doc-section`, `.todo`, `.label`. Code colors: `--code-*` tokens, used by
`::highlight(code-*)` in the showcase and by `[data-hl]` in the SEO response view.

## How a section is structured

One folder per section under `routes/examples/<section>/`, rendered at `/examples/<section>`:

```html
<title>Components · pragma examples</title>
<meta name="description" content="Under 160 characters: what the page shows.">
<link rel="canonical" href="https://pragma.dev/examples/components">
<meta property="og:url" content="https://pragma.dev/examples/components">
<meta property="og:title" content="pragma components: props, slots and per-instance state">
<meta property="og:description" content="One short sentence for link previews.">

<c-doc heading="Components" lede="One or two sentences.">
  <section class="doc-section" aria-labelledby="counter">
    <h2 id="counter">A counter with props and per-instance state</h2>
    <div class="prose"><p>Explanation, with <code>inline code</code>.</p></div>
    <c-showcase :source="data.examples.counter" caption="One line under the example.">
      <p-counter label="Steps of 5" start="10" step="5"></p-counter>
    </c-showcase>
  </section>
</c-doc>
```

- `<c-doc>` gives the sidebar, the single `<h1>` and the lede. Each example is a `.doc-section`
  with an `<h2>` that has an `id` (anchor).
- The slot of `<c-showcase>` is the live result: put the real component there.
- `:source` is a path into the page's `load` data, so the section's `index.server.js` must export
  `ssr = true`: the code then arrives in the first HTML (no skeleton, no layout shift, readable
  without JS). Keys in `readExamples` may use dashes (`'email-check'`); the HTML path is camelCase
  (`data.examples.emailCheck`), because a path segment cannot contain `-`.
- The head lines go right after `<title>` (see "SEO and Open Graph"). Demo routes add
  `<meta name="robots" content="noindex">` instead.
- Static CLI output (commands, `pragma check` errors, logs) goes in `<c-terminal title="~/my-app">`
  with a `<pre tabindex="0" aria-label="..."><code>` inside. Classes for the spans: `term-cmd`
  (a command; the `$ ` is added by CSS), `term-ok`, `term-err`, `term-warn`, `term-dim`, `term-ink`,
  `term-link`.

## How to add an example

1. Create the example as a real component in `routes/examples/<section>/_components/<name>/<name>.html`:
   one file with its `<script>` (state, props, functions, `mount(el)`), the markup and its `<style>`
   (scoped automatically; `:global(…)` opts out). That single file is the showcase's first tab, colored
   as HTML, JS and CSS. Use separate `.js`/`.css` files only when they are clearly better (a page
   script shown on its own, a large store in `lib/state/`).
2. If the reader needs to see how it is used, add `_snippets/<name>/usage.html` with the call site.
3. List the files in the section's `index.server.js` (paths relative to the section folder; the
   tab label is the file name, in this order; tabs are keyed by path, so two files may share a name):

   ```js
   import { readExamples } from '../_source.server.js'

   export const ssr = true

   export async function load() {
     const examples = await readExamples('routes/examples/<section>', {
       counter: ['_components/counter/counter.html', '_snippets/counter/usage.html'],
     })
     return { examples }
   }
   ```
4. In `index.html`, add a `.doc-section` with `<c-showcase :source="data.examples.<key>">` and the live component in its slot.
5. `pragma check --strict`.

Page-level features (routing, `load`, server actions, SSR) cannot run inside a slot. For those,
create a real demo route (for example `routes/examples/data/demo/catalog.html` + `.server.js`),
list its files in `readExamples`, and put a frame alone in the showcase slot:

```html
<c-showcase :source="data.examples.catalog" caption="...">
  <c-frame src="/examples/data/demo/catalog" title="Catalog demo route" height="lg"></c-frame>
</c-showcase>
```

- `src` and `title` are required; `height` is `sm` (18rem), `md` (26rem, the default) or `lg` (34rem).
- The bar shows the frame's live URL (router navigations, back/forward and full loads), a Reload
  button, Open (new tab) and the measured time of the last navigation: `router · 12 ms` is click to
  the router's URL commit (Navigation API; before the view transition), or click to the `<main>`
  swap where that API is missing; `load · 17 ms` is the frame's `loadEventStart`.
- A showcase whose slot holds a `<c-frame>` stacks by itself: code on top, frame full width below.
- The demo route wraps its content in `<c-demo-page>…</c-demo-page>` (after its `<title>`/`<meta>`):
  it gives the page padding and a grid, and hides the site header and footer (CSS `:has()`, no JS).
  pragma v0 has no nested layouts, so this is the minimal demo layout. Leave it out when the demo
  must show the real layout. Keep display utilities (`flex`, `block`) off the root `<header>` and
  `<footer>` of `_layout.html`, or the utilities layer wins over the component rule.

## File ownership

- One agent per section folder: `routes/examples/<section>/` is yours, including `_components/`
  and `_snippets/`. Name components after what they are (`counter`, `product-card`); the tag is
  private to your folder, and so are its styles (`p-` components are scoped by folder).
- Shared, coordinate before editing: `lib/theme.css`, `lib/components/*` (doc, showcase,
  section-nav, terminal), `routes/_layout.html`, `routes/examples/_source.server.js`, `utils/`,
  `pragma.json`, `package.json`, `public/` (robots, sitemap).
- Landing and agents: `routes/index.html`, `routes/agents.html`, `routes/_components/` and `utils/agent-docs/`.
- Guide: `routes/guide.html`.
- `INVENTORY.md` and `BUGS.md` have their own owners; append to `BUGS.md` with the next free id.
- Adding a section: add the route folder and one `<li>` to `lib/components/section-nav/section-nav.html`
  (the only list of sections; the landing, the examples index and every sidebar render it).

## TODO markers

Visible `<p class="todo">TODO(<section>): ...</p>` blocks (dashed amber) mark unfinished content:
HTML comments are rejected by `pragma check`. `grep -rn "TODO(" routes` lists them all.
Open items: none (`grep` finds no TODO marker). Known gap: `_source.server.js` needs a filesystem,
so Workers deploys need the sources inlined (see below).

## Source files at request time

`_source.server.js` reads example files with `node:fs` relative to `process.cwd()`. `pragma dev` and
`pragma serve` start Node in the app folder, so it works there and with
`node dist/server/node.mjs` run from this folder. It does not work on Cloudflare Workers (no
filesystem): deploying there needs the sources inlined at build time.

## Syntax highlighting and the tree-shaking test

Library: **highlight.js 11**, imported with fine-grained subpaths in `utils/highlight.js`:
`highlight.js/lib/core` plus `lib/languages/xml`, `javascript` and `css`. A single-file component
is one tab: the xml grammar hands its `<script>` to the JavaScript grammar and its `<style>` to CSS.
Directives (`:attr`, `@click`, `.done`), `{path}` in text and attribute strings, block tags (`<if>`,
`<for>`) and `<c-*>`/`<p-*>` components get their own colors.

Showcase code is tokenized **on the server**: `_source.server.js` calls `markLines()` in `load`,
which returns each line's text, its indent and its marks (`[start, end, kind]` offsets, kinds in
`codeKinds`). The page is `ssr = true`, and with `ssr` the server runs component scripts too, so
the showcase's tabs, paths and every code line (a `<span>` with plain text) are in the first HTML.
Its state never touches the page's data (props are read-only, also inside): `activeId`,
`copiedId` and `expanded` are `let`s, and `rows()` derives one row per file (`row.active`,
`row.copied`) for the template. `mount(el)` turns the marks into `StaticRange`s in the CSS Custom
Highlight API (`CSS.highlights`, styled with `::highlight(code-*)` in its `<style>`; the `{path}`
highlight has priority 1 so it paints over attribute strings) and watches the live result for the
update counter: the colors cost no DOM node and cause no layout shift. Browsers without the API
show plain code.
Below 64rem long lines soft-wrap with a hanging indent (`:data-indent` + CSS); from 64rem they do
not wrap (the browser would split `aria-label` at the hyphen) and the pane scrolls sideways. From
80rem the code pane fills the row next to the live result (min 20rem). On phones, files longer
than about 16 wrapped rows collapse behind "Show all N lines". The "N updates · X ms" counter
starts at the first interaction in the live result, so hydration is never counted.

The client import of highlight.js is now in one place: `seo/_components/http-response` loads
`utils/highlight.js` with `import()` in `mount` for the head tags it fetches at request time. The
chunk is outside that page's modulepreload list and its page size.

Measured `dist/client/assets/npm/highlight.js-*.js` (rolldown via `pragma build`; raw and gzip in
kB = 1000 bytes, gzip level 9):

| Import | Raw | Gzip |
|---|---|---|
| `import hljs from 'highlight.js'` (index, all 193 languages) | 926.7 kB | 303.4 kB |
| `highlight.js/lib/core` + xml + javascript + css (shipped) | 40.8 kB | 14.8 kB |
| `@lezer/highlight` + `@lezer/javascript` + `html` + `css` (rejected) | 161.5 kB | 60.9 kB |

What got shaken: the 190 unused language modules (95% less gzip). Nothing inside the core is
shaken: highlight.js's ESM entry (`es/core.js`) re-exports the CommonJS `lib/core.js`, and pragma
ships CommonJS whole (packages.md). Lezer is pure ESM and shook well per module, but its
JavaScript grammar alone is 29.6 kB gzip.

What an example page costs now (`pragma build`, dev server for the HTML):

| | Raw | Gzip |
|---|---|---|
| `showcase-*.js` (tabs, copy, highlight ranges, live update counter) | 2.6 kB | 1.4 kB |
| `frame-*.js` | 1.9 kB | 0.8 kB |
| `/examples/syntax` HTML, 10 showcases, code included | 340 kB | 34.0 kB |
| of which the embedded `pragma-data` JSON | 70.6 kB | 12.2 kB |

Example pages need no highlighter chunk (14.8 kB gzip less than before) and measured CLS 0 at 390
and 1440 px (it was up to 1.83 while the code arrived after boot). Route JS per page is in the
`pragma build` table.

## SEO and Open Graph

Per-page head works (`core/SPEC.md` §5.1, `core/docs/routes.md`): a page writes `<title>`,
`<meta>` and `<link>` at its top level and the build moves them into `<head>`, replacing the layout
tag with the same `name`/`property`/`http-equiv` (or `rel` + `hreflang`) and appending the rest.

- `routes/_layout.html` `<head>`: site-wide defaults only (description, theme-color, font preloads,
  `og:type`, `og:site_name`, `og:title`, `og:description`, `og:image*`, `twitter:card`). Never put
  `canonical` or `og:url` there. Directives in the layout head are an error (`html/head`).
- Every indexable page (landing, guide, examples index, each section) sets `meta description`
  (under 160 characters), `canonical`, `og:url`, `og:title` and `og:description`. X falls back to
  `og:*`, so no `twitter:title`/`twitter:description` are needed.
- Every iframe demo route under `*/demo/*` sets `<meta name="robots" content="noindex">`. The seo
  demo (`examples/seo/demo/*`) is the exception: it models indexable articles, and its `[slug]`
  route binds `robots` from data (`ssr = true`) for missing notes.
- `public/robots.txt` points to `public/sitemap.xml`; add new indexable pages to the sitemap.
- The origin `https://pragma.dev` (canonical, `og:url`, `og:image`, sitemap) is a placeholder:
  replace it with the real domain everywhere at once.
