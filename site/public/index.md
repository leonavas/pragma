# The web framework for agents, with rules they can't bend.

Agents: the full docs are one file at https://pragma.leonavas.com/llms-full.txt (index: https://pragma.leonavas.com/llms.txt). This page in HTML: https://pragma.leonavas.com/. For agents: https://pragma.leonavas.com/agents.md.

Send your agent this page. It reads the docs and writes native HTML, CSS and JavaScript. A Rust CLI rejects anything off-spec with the file, the line and the fix. Pages ship prebuilt, with only the JavaScript they use, and every form is a tool an agent can call.

- [Hand it to your agent](https://pragma.leonavas.com/agents.md)
- [Install](#install)
- [Read the examples](https://pragma.leonavas.com/examples)

## Install

One Rust binary compiles, checks, serves and bundles npm packages: no Vite, no esbuild, no config. Node 22 or newer only runs your `.server.js` files.

Linux x64: run the one-line installer or download [pragma-linux-x64.tar.gz](https://pragma.leonavas.com/downloads/pragma-linux-x64.tar.gz).

```sh
curl -fsSL https://pragma.leonavas.com/install.sh | sh
pragma -V
```

From source (any platform with Cargo):

```sh
git clone https://github.com/leonavas/pragma
cd pragma
cargo install --path core/cli/crates/pragma-cli
```

Then `pragma new my-app`, `cd my-app` and `pragma dev`. [Start with the guide](https://pragma.leonavas.com/guide).

## This framework is for agents. If you're still reading this instead of asking your agent to try it, it's not for you yet. Come back in 6 months.

[Hand it to your agent](https://pragma.leonavas.com/agents.md)

## Nobody argues about frameworks anymore. The agent picks one.

Most people who build with an LLM don't know frontend. They ask for React or Angular because those are the names they know, and ship slow pages.

Some don't care and get plain HTML and JavaScript. Nothing beats it on speed. Nothing about it scales: it eats context, turns into a tangle, and the agent loses the thread.

Some know a bit more and pick SvelteKit, Astro or htmx. Good tools, made for people who read docs.

**pragma is made for the agent:**

- The minimum lint that stops the tangle: one way to do each thing, and every error names the rule and the fix.
- Components and reactivity on what the browser already has: ES modules, `<template>`, `<dialog>`, popover, view transitions.
- One Rust binary compiles, lints and validates. No Vite, no config.
- Plain JavaScript. Never TypeScript.
- WebMCP by default: every form is a tool an agent in the browser can call. A page with a .md next to it answers in markdown when an agent asks.

## One file: markup, script and style

A component is one .html file. A top-level `let` is state, a function is an action or a derived value, and `{path}` puts a value in the text. Check a task, add one, hide the done ones: each action assigns a variable and the runtime reapplies the bindings. No hooks, no virtual DOM. The form is also a WebMCP tool: an agent in the browser can call `add_task`.

This is the whole component, read from disk when the page renders: the source you see is the source that runs.

```html
<script>
  let draft = ''
  let pendingOnly = false
  let nextId = 4
  let tasks = [
    { id: 1, title: 'Write the page in one .html file', done: true },
    { id: 2, title: 'Show values with {path}', done: false },
    { id: 3, title: 'Run pragma check --strict', done: false },
  ]

  const visible = () => (pendingOnly ? tasks.filter(task => !task.done) : tasks)
  const pending = () => tasks.filter(task => !task.done).length
  const total = () => tasks.length
  const filterText = () => (pendingOnly ? 'Show all' : 'Hide done')
  const filterPressed = () => (pendingOnly ? 'true' : 'false')

  function add(event) {
    const title = String(new FormData(event.target).get('title')).trim()
    if (!title) return
    tasks.push({ id: nextId, title, done: false })
    nextId += 1
    draft = ''
    if (event.agentInvoked) event.respondWith(Promise.resolve('Added "' + title + '". ' + pending() + ' of ' + total() + ' tasks pending.'))
  }

  function toggle(task) {
    task.done = !task.done
  }

  function toggleFilter() {
    pendingOnly = !pendingOnly
  }
</script>

<div class="grid w-full max-w-sm gap-3.5">
  <div class="flex items-center justify-between gap-4 font-mono text-xs">
    <p class="text-faint tabular-nums" role="status">{$pending} of {$total} pending</p>
    <button type="button" class="tasks-filter cursor-pointer py-1.5 underline hover:text-ink" @click="toggleFilter" :aria-pressed="$filterPressed">{$filterText}</button>
  </div>
  <form class="flex gap-2" toolname="add_task" tooldescription="Add a task to the demo to-do list on the pragma landing page" @submit="add">
    <input class="h-8 min-w-0 flex-1 border border-line-strong bg-sunken px-3 text-sm text-ink focus-visible:border-accent" type="text" name="title" required maxlength="48" placeholder="Add a task" aria-label="New task" toolparamdescription="Title of the task, up to 48 characters" :model="$draft">
    <button type="submit" class="btn btn-small">Add</button>
  </form>
  <ul class="border-t border-line">
    <for task in $visible key=task.id>
      <li class="flex min-h-11 items-center gap-3 border-b border-line" .done="task.done">
        <input type="checkbox" class="tasks-check size-4 shrink-0 cursor-pointer" :checked="task.done" :aria-label="task.title" @change="toggle(task)">
        <span class="tasks-title min-w-0">{task.title}</span>
      </li>
    </for>
    <empty>
      <li class="border-b border-line py-3.5 text-sm text-muted">Nothing pending. Ship it.</li>
    </empty>
  </ul>
</div>

<style>
  .tasks-filter { color: var(--color-muted); text-decoration-color: var(--color-line-strong); text-underline-offset: 0.3em; }
  .tasks-filter[aria-pressed="true"] { color: var(--color-accent); text-decoration-color: currentColor; }
  .tasks-check { accent-color: var(--color-accent); }
  .tasks-title {
    overflow-wrap: anywhere;
    background: linear-gradient(currentColor, currentColor) 0 55% / 0 1px no-repeat;
    transition: background-size 0.28s var(--ease-out), color 0.2s var(--ease-out);
  }
  .done .tasks-title { color: var(--color-faint); background-size: 100% 1px; }
  @media (prefers-reduced-motion: reduce) {
    .tasks-title { transition: none; }
  }
</style>
```

## Four constraints, enforced by the compiler

### Native

A page or a component is one .html file: the markup, its `<script>` and its `<style>`. Its data comes from the .server.js with the same name. No JSX, no virtual DOM, no framework between you and the platform.

```
routes/
  index.html
  shop/[id].html          markup, <script>, <style>
  shop/[id].server.js     load, server actions
  shop/_components/
    cart-button/cart-button.html
```

### Closed

Braces put a value in text, a colon binds an attribute, a dot a class, an at sign an event, a dollar sign names the script's state. Conditions and loops are tags. Anything outside that list is a build error, not a convention.

```html
<for task in $visible key=task.id>
  <li .done="task.done">
    <input type="checkbox" @change="toggle(task)">
    {task.title}
  </li>
</for>
```

### Small

The starter page loads 10.0 kB of JavaScript, gzip, router and runtime included. Each page preloads only the modules it imports, and budgets in pragma.json fail the build when a page grows.

```
page     JS gzip
/        10.0 KB
CSS       4.1 KB
1 page(s)
dist/ built in 6 ms
```

### Agent-first

Every app ships with AGENTS.md and its docs. pragma check answers with file, line, column, rule and fix, so an agent corrects itself in one pass instead of guessing. Every form is a WebMCP tool, and a page can answer in markdown.

```
routes/index.html:8:4 error [html/unknown-name]
  $count is not declared in the script of routes/index.html:
  add `let count = …` (state), `export let count = …` (prop)
  or `function count() { … }`
```

## Tools for agents on this page

An agent in the browser sees these forms as WebMCP tools and calls them. You can use the same forms. Agents without a browser read the same docs at https://pragma.leonavas.com/llms-full.txt.

- `get_setup_instructions`: Get the commands to install pragma and create an app, plus a prompt for a coding agent, for a short app description
  - `app`: What the app does, in one sentence, written to follow 'create a pragma app that'
- `get_pragma_docs`: Get one pragma framework doc as markdown, by topic; read rules first, then the topic you need
  - `topic`: Doc to read: rules is AGENTS.md, the others are the files in core/docs
- `add_task`: Add a task to the demo to-do list on the pragma landing page
  - `title`: Title of the task, up to 48 characters

[Everything for agents](https://pragma.leonavas.com/agents.md)

## Every feature, running on this page's runtime

Each example shows its real source files next to the live result. The code you read is the code that renders. [Browse all examples](https://pragma.leonavas.com/examples).

- [Syntax](https://pragma.leonavas.com/examples/syntax): {path}, bindings, events, blocks
- [Components](https://pragma.leonavas.com/examples/components): one file, props, slots
- [State and actions](https://pragma.leonavas.com/examples/state): let, functions, mount, services
- [Stores](https://pragma.leonavas.com/examples/stores): shared and persisted state
- [Routing](https://pragma.leonavas.com/examples/routing): files as routes, params, query, prefetch
- [Data and SSR](https://pragma.leonavas.com/examples/data): load, server actions, skeletons
- [Overlays](https://pragma.leonavas.com/examples/overlays): modal, alert, dropdown, Back button
- [Styling](https://pragma.leonavas.com/examples/styling): utilities, theme tokens, component CSS
- [npm packages](https://pragma.leonavas.com/examples/packages): tree-shaken per package
- [Logs](https://pragma.leonavas.com/examples/logs): OpenTelemetry, client and server
- [SEO and head](https://pragma.leonavas.com/examples/seo): title, meta, Open Graph
