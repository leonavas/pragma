# WebMCP: forms are agent tools

WebMCP is on by default (`"webmcp": true` in `pragma.json`; omitted means `true`). Every `<form>` is a declarative WebMCP tool. The browser registers the tool while the form is mounted and removes it when the form unmounts.

```html
<form toolname="search_products" tooldescription="Search the catalog by name or category" toolautosubmit @submit="search">
  <label for="query">Search</label>
  <input id="query" name="query" type="search" required toolparamdescription="Words to search for in product names and categories">
  <button class="btn btn-primary">Search</button>
</form>
```

| Attribute | Target | Rule |
|---|---|---|
| `toolname` | `<form>` | Required with `tooldescription`. Lowercase `snake_case` `verb_object` (`search_products`, `add_task`, `create_order`), unique per page. |
| `tooldescription` | `<form>` | Required with `toolname`. One sentence of 4+ words describing action and purpose. |
| `toolparamdescription` | input, select, textarea | Describes the expected input. Without it, agents fall back to `<label>` text, then `aria-description`. |
| `toolautosubmit` | `<form>` | Submits without user click. Use only on safe, reversible forms (search, filter, add). Never on delete, pay, buy, cancel, send, or transfer. |
| `notool` | `<form>` | Excludes the form (login, password, payment). Removed at build time. |

- Add `name` to every field (`type=submit|reset|button|image` exempt).
- Use native constraints (`required`, `type`, `min`/`max`, `<select>` options) to describe parameters.
- Use static strings only; do not use `:` bindings or `{}` in tool attributes.
- Without `toolautosubmit`, the browser fills the form and focuses the submit button for the user.
- With `"webmcp": false`, any tool attribute triggers `webmcp/disabled`.

## Result to the agent

`@submit="fn"` (no parentheses) passes the `SubmitEvent`. `event.agentInvoked` is `true` when an agent submits. Call `event.respondWith(promise)` before any `await`. Read fields from `new FormData(event.target)`, not `:model` state.

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

Return plain JSON (`{ products: [...] }`, `{ error: 'Use at least 3 characters.' }`). Respond with validation errors too so the agent can retry.

## CSS

`:tool-form-active` matches a form while an agent fills it. `:tool-submit-active` matches its submit button. `.pragma/base.css` styles both; override them in `lib/theme.css` or page `<style>`:

```css
form:tool-form-active { outline: 2px solid var(--color-primary-600); }
```

## Check rules (`webmcp/*`)

Errors (broken tool):

| Rule | Fix |
|---|---|
| `webmcp/pair` | Provide both `toolname` and `tooldescription`, or neither. |
| `webmcp/name` | Use 1–128 chars from `A-Z a-z 0-9 _ . -` in `toolname`. |
| `webmcp/duplicate` | Use a unique `toolname` per page (`<if>`/`<else>` branches count separately). |
| `webmcp/param-name` | Add `name` to every field in a tool form. |
| `webmcp/dynamic` | Use static text only in tool attributes (no `:` or `{}`). |
| `webmcp/notool` | Use `notool` only on `<form>`, never with `toolname`. |
| `webmcp/attribute` | Put `tooldescription`/`toolautosubmit` only on `<form>`, and `toolparamdescription` only on input, select, or textarea. |
| `webmcp/disabled` | Remove tool attributes when `"webmcp": false`, or set `"webmcp": true`. |

Warnings (errors with `--strict`):

| Rule | Fix |
|---|---|
| `webmcp/form-tool` | Add `toolname` + `tooldescription`, or `notool`. |
| `webmcp/name-style` | Use lowercase `snake_case` `verb_object` with at least one `_` (`search_products`). |
| `webmcp/description` | Write 4+ words in `tooldescription`. |
| `webmcp/param-description` | Add `toolparamdescription`, `<label>`, or `aria-description` to the field. |
| `webmcp/autosubmit` | Remove `toolautosubmit` from destructive or financial tools (`delete_`, `remove_`, `destroy_`, `pay_`, `buy_`, `purchase_`, `cancel_`, `send_`, `transfer_`). |

## Disable WebMCP

Set `"webmcp": false` in `pragma.json` to disable `webmcp/*` checks (existing tool attributes then trigger `webmcp/disabled`). Prefer `notool` on individual forms.

## Markdown for agents

Non-browser agents request `text/markdown`. Add a `.md` file in `public/` matching the route: `public/index.md` for `/`, `public/pricing.md` or `public/pricing/index.md` for `/pricing`. When `Accept` prefers `text/markdown` (quality >= `text/html`), the server returns that `.md` file; otherwise it returns HTML.
