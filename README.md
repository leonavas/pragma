<p align="center">
  <img src="design/logo/readme-banner.svg" alt="pragma" width="480">
</p>

# pragma

The web framework for agents, with rules they can't bend.

A coding agent reads the docs and writes native HTML, CSS and JavaScript. A Rust CLI rejects anything
off-spec with the file, the line and the fix. Pages ship prebuilt, with only the JavaScript they use,
and every form is a tool an agent can call.

- Site: <https://pragma.leonavas.com>
- Guide: <https://pragma.leonavas.com/guide>
- Examples: <https://pragma.leonavas.com/examples>
- For agents: <https://pragma.leonavas.com/agents.md> (all docs in one file:
  <https://pragma.leonavas.com/llms-full.txt>)

## Why

- **Native.** A page or a component is one `.html` file: markup, `<script>` and `<style>`. No JSX, no
  virtual DOM, no client framework.
- **Closed.** One way to do each thing. Anything outside the syntax is a build error, not a convention.
- **Small.** The starter page loads 10.0 kB of JavaScript (gzip), router and runtime included.
  `pragma build` prints the JS size of each page.
- **Agent-first.** Every app ships with `AGENTS.md` and its docs. `pragma check` answers with file,
  line, column, rule and fix. Every `<form>` is a WebMCP tool, and a page can answer in markdown.
- **One binary.** The CLI compiles, checks, serves and bundles npm packages. No Vite, no esbuild, no
  config. Plain JavaScript, never TypeScript.

## Install

Linux and macOS (x64 and arm64):

```sh
curl -fsSL https://raw.githubusercontent.com/leonavas/pragma/main/install.sh | sh
pragma -V
```

The installer downloads the binary from [GitHub Releases](https://github.com/leonavas/pragma/releases/latest) and checks its SHA-256. Windows: download
the `.zip` from the same page. Other platforms build from source with Cargo:

```sh
git clone https://github.com/leonavas/pragma
cd pragma
cargo install --path core/cli/crates/pragma-cli
```

Node 22 or newer is necessary only to run `.server.js` files.

## Quick start

```sh
pragma new my-app
cd my-app
pragma dev
```

Or give this prompt to your coding agent:

```
Read https://pragma.leonavas.com/llms-full.txt and create a pragma app that [describe your app].
Follow the AGENTS.md that pragma new writes into the app, and finish with pragma check --strict
at zero errors and zero warnings.
```

## A component

A top-level `let` is state. A function is an action or a derived value. `{path}` puts a value in the
text.

```html
<script>
  export let label = 'Count'
  export let start = 0
  export let step = 1

  let count = start

  const atStart = () => count === start

  function decrease() { count -= step }
  function increase() { count += step }
  function reset() { count = start }
</script>

<div role="group" :aria-label="$label">
  <span>{$label}</span>
  <button type="button" aria-label="Decrease" @click="decrease">−</button>
  <output>{$count}</output>
  <button type="button" aria-label="Increase" @click="increase">+</button>
  <button type="button" @click="reset" :disabled="$atStart">Reset</button>
</div>
```

## Commands

| Command | Function |
|---|---|
| `pragma new <dir>` | Creates an app with the fixed structure. |
| `pragma dev` | Runs the dev server with live reload (port 5180). |
| `pragma check [--strict]` | Checks JS, HTML, CSS, accessibility and WebMCP tools. |
| `pragma build` | Builds `dist/`. |
| `pragma serve` | Serves `dist/` as in production (port 3000). |
| `pragma upgrade` | Syncs `.pragma/` in an app with the installed CLI version. |
| `pragma self-update [version]` | Updates the CLI binary from GitHub Releases. |

## Repository layout

```
core/      the framework: spec, docs, runtime, starter and the Rust CLI (see core/README.md)
site/      pragma.leonavas.com, built with pragma (see site/README.md)
npm/       npm packages that wrap the CLI binaries
install.sh the installer: downloads a release binary and checks its SHA-256
design/    brand sources: logo, the Pragma Reto font, Open Graph image (see design/README.md)
```

The v0 contract is [core/SPEC.md](core/SPEC.md). Decisions and their rationale are in
[core/DECISIONS.md](core/DECISIONS.md).

## Development

```sh
cd core/cli
cargo build --release      # the pragma binary
cargo test                 # see core/TESTING.md

cd ../../site
npm i --ignore-scripts
pragma dev --port 5181     # the site
pragma check --strict
```

## CI and releases

- `.github/workflows/build.yml` runs on each push to `main` and on each pull request. It builds the
  CLI, then checks and builds the site with that binary.
- `.github/workflows/release.yml` builds the binaries for Linux, macOS and Windows and publishes them
  to GitHub Releases with `SHA256SUMS.txt`. It runs when you push a tag `vX.Y.Z`. You can also run it
  by hand with `release: true`: the tag is then `v<version>` from `core/cli/Cargo.toml`.
- `.github/workflows/deploy-site.yml` runs when a push to `main` changes `site/`. It builds the CLI,
  checks and builds the site, then deploys `site/dist` to Cloudflare Workers with `wrangler`. It needs
  the repository secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`.
- `install.sh` and `pragma self-update` download only from GitHub Releases.

## License

[MIT](LICENSE)
