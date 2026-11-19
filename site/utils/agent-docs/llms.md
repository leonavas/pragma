# pragma

> pragma is a web framework built for coding agents. An app is native HTML, CSS and plain JavaScript (never TypeScript) with a closed set of directives; a Rust CLI compiles, lints and validates it and answers every error with the file, the line, the rule and the fix. Pages ship prebuilt with only the JavaScript they use, and every form is a WebMCP tool by default.

To build an app with pragma, read {{origin}}/llms-full.txt: install commands, the app rules (AGENTS.md) and every doc in one file. Each doc is also a separate markdown file below. Every app created with `pragma new` has the same docs in `core/docs/` and the rules in `AGENTS.md`.

## Start

- [Everything in one file]({{origin}}/llms-full.txt): install, app rules and every doc
- [For agents]({{origin}}/agents.md): the prompt, install commands, files and WebMCP tools of this site
- [Landing page]({{origin}}/index.md): what pragma is and why it exists

## Docs

{{docs}}

## Optional

- [Guide]({{origin}}/guide): from an empty folder to a deployed app, with real CLI output
- [Examples]({{origin}}/examples): every feature with its source files and a live result
- [Source code](https://github.com/leonavas/pragma): the framework, the CLI and this site
