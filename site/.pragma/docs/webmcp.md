# WebMCP: forms are agent tools

On by default (`"webmcp": true` in `pragma.json`; absent = on). Every `<form>` is a WebMCP
declarative tool: an AI agent in the browser finds it, fills it and submits it. The browser
registers the tool while the form is in the page and removes it when the form leaves (router
navigation included). No JS for this.

```html
<form toolname="search_products" tooldescription="Search the catalog by name or category" toolautosubmit @submit="search">
  <label for="query">Search</label>
  <input id="query" name="query" type="search" required toolparamdescription="Words to search for in product names and categories">
  <button class="btn btn-primary">Search</button>
</form>
```

| Attribute | On | Rule |
|---|---|---|
| `toolname` | `<form>` | required with `tooldescription`; lowercase `snake_case` verb_object (`search_products`, `add_task`, `create_order`); unique per page |
| `tooldescription` | `<form>` | required with `toolname`; one sentence, 4+ words: what it does and when to use it |
| `toolparamdescription` | input, select, textarea | what to put in the field; without it the agent reads the `<label>` text, then `aria-description` |
| `toolautosubmit` | `<form>` | the agent submits without a user click. Only safe, reversible forms (search, filter, add). Never delete, pay, buy, cancel, send or transfer |
| `notool` | `<form>` | not a tool (login, password, payment details); the build removes it |

- Every field needs `name`: it is the parameter name (`type=submit|reset|button|image` exempt).
- Use native constraints (`required`, `type`, `min`/`max`, `<select>` options): they describe the parameter.
- Static values only: no `:toolname`, no `{}` in any tool attribute.
- Without `toolautosubmit` the browser fills the form, focuses the submit button and the user submits.
- With `"webmcp": false` every tool attribute is an error (`webmcp/disabled`): remove them.

## Result to the agent

`@submit="fn"` (no parentheses) gives `fn` the `SubmitEvent`. `event.agentInvoked` is `true` when
an agent submitted. `@submit` already calls `event.preventDefault()`; call
`event.respondWith(promise)` before any `await`: the resolved value goes to the model. The agent
fills the fields directly: read them with `new FormData(event.target)`, not from `:model` state.

```html
<script>
  import { call } from 'pragma'

  let products = []

  async function search(event) {
    const query = new FormData(event.target).get('query')
    const reply = call('search', { query })
    if (event.agentInvoked) event.respondWith(reply)
    products = (await reply).products
  }
</script>
```

Return plain JSON the model can read (`{ products: [...] }`, `{ error: 'Use at least 3 characters.' }`).
For a validation error, respond with it too, so the agent can fix the input and try again.

## CSS

`:tool-form-active` matches the form while an agent fills it; `:tool-submit-active` matches its
submit button. `.pragma/base.css` styles both with theme tokens; restyle them in `lib/theme.css` or
the page `<style>`:

```css
form:tool-form-active { outline: 2px solid var(--color-primary-600); }
```

## Check rules (`webmcp/*`)

Errors: the tool is broken.

| Rule | Fix |
|---|---|
| `webmcp/pair` | write both `toolname` and `tooldescription`, or neither |
| `webmcp/name` | `toolname` with 1–128 chars of `A-Z a-z 0-9 _ . -` |
| `webmcp/duplicate` | one `toolname` per page (`<if>`/`<else>` branches count separately): rename one |
| `webmcp/param-name` | `name` on every field of a tool form |
| `webmcp/dynamic` | tool attributes are static text: no `:` binding, no `{}` |
| `webmcp/notool` | `notool` only on a `<form>`, never with `toolname` |
| `webmcp/attribute` | `tooldescription`/`toolautosubmit` only on `<form>`; `toolparamdescription` only on input, select, textarea |
| `webmcp/disabled` | `"webmcp": false`: remove every tool attribute, or set `"webmcp": true` |

Warnings (errors with `--strict`).

| Rule | Fix |
|---|---|
| `webmcp/form-tool` | add `toolname` + `tooldescription`, or `notool` if agents must not use it |
| `webmcp/name-style` | lowercase `snake_case` verb_object with at least one `_` (`search_products`) |
| `webmcp/description` | `tooldescription` of 4+ words |
| `webmcp/param-description` | `toolparamdescription`, a `<label>` or `aria-description` on the field |
| `webmcp/autosubmit` | remove `toolautosubmit` from tools named `delete_`, `remove_`, `destroy_`, `pay_`, `buy_`, `purchase_`, `cancel_`, `send_`, `transfer_`… |

## Turn it off

`pragma.json`: `"webmcp": false`. The `webmcp/*` rules stop, and any tool attribute in the source
is a `webmcp/disabled` error. Prefer `notool` on single forms.

## Markdown for agents

Agents that fetch pages (not in a browser) ask for `text/markdown`. Put a `.md` next to the page
in `public/`: `public/index.md` for `/`, `public/pricing.md` or `public/pricing/index.md` for
`/pricing`. A request whose `Accept` lists `text/markdown` with a q at least as high as `text/html` gets that file; browsers get the HTML.
No `.md` = HTML for everyone.
