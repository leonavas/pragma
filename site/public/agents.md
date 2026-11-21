# pragma, for your agent

Agents: the full docs are one file at https://pragma.leonavas.com/llms-full.txt. This page in HTML: https://pragma.leonavas.com/agents. Human version: https://pragma.leonavas.com/index.md.

Everything an agent needs to build with pragma, in files it reads without a browser. Human: copy the prompt, paste it into your coding agent and say what to build.

## The prompt

Replace the brackets with your app. The agent reads the docs, installs pragma, creates the app and runs `pragma check --strict` until it passes.

```
Read https://pragma.leonavas.com/llms-full.txt and create a pragma app that [describe your app]. Install pragma if missing, or validate the version with pragma -V and .pragma/version and run pragma self-update and pragma upgrade if outdated. Follow the AGENTS.md that pragma new writes into the app, and finish with pragma check --strict at zero errors and zero warnings.
```

## Install and create

One Rust binary. The installer supports Linux and macOS. Binaries for all platforms: [GitHub Releases](https://github.com/leonavas/pragma/releases/latest). Node 22 or newer only runs `.server.js` files.

```sh
curl -fsSL https://raw.githubusercontent.com/leonavas/pragma/main/install.sh | sh
pragma new my-app
cd my-app
pragma dev
```

Or build from source with Cargo:

```sh
git clone https://github.com/leonavas/pragma && cd pragma && cargo install --path core/cli/crates/pragma-cli
```

## Validate version and update

Validate the pragma version before you change code:

- Run `pragma -V` to check the installed CLI version.
- Compare that version with `.pragma/version` inside the app.
- Run `pragma self-update` to update the CLI binary from GitHub Releases if the CLI is old.
- Run `pragma upgrade` inside the app to update `.pragma/` (runtime, docs, `base.css`, `version`) if `.pragma/` is outdated.

```sh
pragma -V
cat .pragma/version
pragma self-update
pragma upgrade
```

## Files for agents

Same content as the HTML pages, in markdown. No hidden text, nothing written only for agents.

- [/llms-full.txt](https://pragma.leonavas.com/llms-full.txt): the app rules and every doc in one file. Start here.
- [/llms.txt](https://pragma.leonavas.com/llms.txt): the index, in the llmstxt.org format.
- [/index.md](https://pragma.leonavas.com/index.md): the landing page.
- [/agents.md](https://pragma.leonavas.com/agents.md): this page.

A request for `/` or `/agents` with `Accept: text/markdown` gets the markdown file instead of the HTML. One doc per file:

- [App rules (AGENTS.md)](https://pragma.leonavas.com/docs/rules.md): the rules every pragma app follows; read first
- [App structure](https://pragma.leonavas.com/docs/structure.md): where each file goes, pragma.json
- [HTML syntax](https://pragma.leonavas.com/docs/syntax.md): HTML directives ({path}, :, ., @, $) and blocks
- [Components](https://pragma.leonavas.com/docs/components.md): components, props, slots
- [State: script, actions, derived values, stores](https://pragma.leonavas.com/docs/state.md): script: state, actions, derived values, mount, stores
- [Routes and router](https://pragma.leonavas.com/docs/routes.md): pages, links, params, query, layouts (including nested), head tags
- [Data: load, server actions, skeleton, SSR](https://pragma.leonavas.com/docs/data.md): load, server actions, skeleton, SSR
- [Overlays: modal, alert, dropdown](https://pragma.leonavas.com/docs/overlays.md): modal, alert, dropdown
- [CSS](https://pragma.leonavas.com/docs/css.md): styles and theme
- [Utility classes](https://pragma.leonavas.com/docs/classes.md): every utility class
- [WebMCP: forms are agent tools](https://pragma.leonavas.com/docs/webmcp.md): forms as WebMCP tools, markdown for agents
- [npm packages](https://pragma.leonavas.com/docs/packages.md): npm packages: when and which
- [Performance](https://pragma.leonavas.com/docs/performance.md): performance defaults and budgets
- [Logs](https://pragma.leonavas.com/docs/logs.md): OpenTelemetry logs
- [pragma CLI](https://pragma.leonavas.com/docs/cli.md): commands, self-update, upgrade, check rules, deploy

## WebMCP tools on this site

This page and the landing page expose these tools to agents in the browser, as declarative WebMCP forms. Humans use the same forms. The landing page also has `add_task` (its demo to-do list), and `/examples/data` has `sign_guestbook`.

- `get_setup_instructions`: Get the commands to install pragma and create an app, plus a prompt for a coding agent, for a short app description
  - `app`: What the app does, in one sentence, written to follow 'create a pragma app that'
- `get_pragma_docs`: Get one pragma framework doc as markdown, by topic; read rules first, then the topic you need
  - `topic`: Doc to read: rules is AGENTS.md, the others are the files in .pragma/docs

## Still reading?

This framework is for agents. If you're still reading this instead of asking your agent to try it, it's not for you yet. Come back in 6 months.
