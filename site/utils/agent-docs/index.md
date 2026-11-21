# The web framework for agents, with rules they can't bend.

Agents: the full docs are one file at {{origin}}/llms-full.txt (index: {{origin}}/llms.txt). This page in HTML: {{origin}}/. For agents: {{origin}}/agents.md.

Send your agent this page. It reads the docs and writes native HTML, CSS and JavaScript. A Rust CLI rejects anything off-spec with the file, the line and the fix. Pages ship prebuilt, with only the JavaScript they use, and every form is a tool an agent can call.

- [Hand it to your agent]({{origin}}/agents.md)
- [Install](#install)
- [Read the examples]({{origin}}/examples)

## Install

One Rust binary compiles, checks, serves and bundles npm packages: no Vite, no esbuild, no config. Node 22 or newer only runs your `.server.js` files.

Linux and macOS: run the one-line installer. Windows: download the binary from [GitHub Releases](https://github.com/leonavas/pragma/releases/latest).

```sh
curl -fsSL https://raw.githubusercontent.com/leonavas/pragma/main/install.sh | sh
pragma -V
```

From source (any platform with Cargo):

```sh
git clone https://github.com/leonavas/pragma
cd pragma
cargo install --path core/cli/crates/pragma-cli
```

Then `pragma new my-app`, `cd my-app` and `pragma dev`. [Start with the guide]({{origin}}/guide).

Update the CLI with `pragma self-update` (GitHub Releases). Update `.pragma/` (runtime, docs, `base.css`, `version`) inside an app with `pragma upgrade`.

```sh
pragma self-update
pragma upgrade
```

## This framework is for agents. If you're still reading this instead of asking your agent to try it, it's not for you yet. Come back in 6 months.

[Hand it to your agent]({{origin}}/agents.md)

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

{{source routes/_components/task-list/task-list.html}}

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

An agent in the browser sees these forms as WebMCP tools and calls them. You can use the same forms. Agents without a browser read the same docs at {{origin}}/llms-full.txt.

{{tools routes/_components/setup-tool/setup-tool.html routes/_components/docs-tool/docs-tool.html routes/_components/task-list/task-list.html}}

[Everything for agents]({{origin}}/agents.md)

## Every feature, running on this page's runtime

Each example shows its real source files next to the live result. The code you read is the code that renders. [Browse all examples]({{origin}}/examples).

- [Syntax]({{origin}}/examples/syntax): {path}, bindings, events, blocks
- [Components]({{origin}}/examples/components): one file, props, slots
- [State and actions]({{origin}}/examples/state): let, functions, mount, services
- [Stores]({{origin}}/examples/stores): shared and persisted state
- [Routing]({{origin}}/examples/routing): files as routes, params, query, prefetch
- [Data and SSR]({{origin}}/examples/data): load, server actions, skeletons
- [Overlays]({{origin}}/examples/overlays): modal, alert, dropdown, Back button
- [Styling]({{origin}}/examples/styling): utilities, theme tokens, component CSS
- [npm packages]({{origin}}/examples/packages): tree-shaken per package
- [Logs]({{origin}}/examples/logs): OpenTelemetry, client and server
- [SEO and head]({{origin}}/examples/seo): title, meta, Open Graph
