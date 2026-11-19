# pragma, for your agent

Agents: the full docs are one file at {{origin}}/llms-full.txt. This page in HTML: {{origin}}/agents. Human version: {{origin}}/index.md.

Everything an agent needs to build with pragma, in files it reads without a browser. Human: copy the prompt, paste it into your coding agent and say what to build.

## The prompt

Replace the brackets with your app. The agent reads the docs, installs pragma, creates the app and runs `pragma check --strict` until it passes.

```
Read {{origin}}/llms-full.txt and create a pragma app that [describe your app]. Install pragma first if the pragma command is missing. Follow the AGENTS.md that pragma new writes into the app, and finish with pragma check --strict at zero errors and zero warnings.
```

## Install and create

One Rust binary. Building it needs Cargo; Node 22 or newer only runs `.server.js` files.

```sh
git clone https://github.com/leonavas/pragma
cd pragma
cargo install --path core/cli/crates/pragma-cli
cd ..
pragma new my-app
cd my-app
pragma dev
```

## Files for agents

Same content as the HTML pages, in markdown. No hidden text, nothing written only for agents.

- [/llms-full.txt]({{origin}}/llms-full.txt): the app rules and every doc in one file. Start here.
- [/llms.txt]({{origin}}/llms.txt): the index, in the llmstxt.org format.
- [/index.md]({{origin}}/index.md): the landing page.
- [/agents.md]({{origin}}/agents.md): this page.

A request for `/` or `/agents` with `Accept: text/markdown` gets the markdown file instead of the HTML. One doc per file:

{{docs}}

## WebMCP tools on this site

This page and the landing page expose these tools to agents in the browser, as declarative WebMCP forms. Humans use the same forms. The landing page also has `add_task` (its demo to-do list), and `/examples/data` has `sign_guestbook`.

{{tools routes/_components/setup-tool/setup-tool.html routes/_components/docs-tool/docs-tool.html}}

## Still reading?

This framework is for agents. If you're still reading this instead of asking your agent to try it, it's not for you yet. Come back in 6 months.
