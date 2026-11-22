# Forms: values, select and styled select

Every `<form>` is a WebMCP tool (`webmcp.md`). Read values with `new FormData(event.target)` in `@submit`.

## Values from `:model`

| Input | Value in the `let` |
|---|---|
| `text`, `email`, `<textarea>`, `<select>` | String (for `<select>`, the `value` of the selected option, not its label). |
| `number`, `range` | Number; `null` when the field is empty. |
| `checkbox` | Boolean. |
| `radio` | The `value` of the checked radio of the group (each radio has the same `:model`). |

## Select

Use a native `<select>` by default. It works in all browsers, with the keyboard and on phones.

```html
<select name="column" aria-label="Column" toolparamdescription="Column id: todo, doing or done">
  <option value="todo">To do</option>
  <option value="doing">Doing</option>
  <option value="done">Done</option>
</select>
```

Style it with utilities (border, padding, radius). Do not use `appearance: base-select`, `::picker(select)`, or `<selectedcontent>`: Firefox does not support them.

## Styled select: `c-select`

Use `c-select` when the list needs a custom look (icons, colors, check mark). It opens a `c-dropdown` and keeps a hidden native `<select name>` in sync, so the form, `FormData`, and the WebMCP tool get the value.

```html
<script>
  const columns = () => [
    { value: 'todo', label: 'To do' },
    { value: 'doing', label: 'Doing' },
    { value: 'done', label: 'Done' },
  ]

  function add(event) {
    const column = new FormData(event.target).get('column')
  }
</script>

<form toolname="add_card" tooldescription="Add a card to a column of the board" @submit="add">
  <c-select name="column" label="Column" value="todo" toolparamdescription="Column id: todo, doing or done" :options="$columns"></c-select>
  <button class="btn btn-primary">Add</button>
</form>
```

| Prop | Value |
|---|---|
| `name` | Form field name (the tool parameter). The list id is `<name>-options`: use a unique `name` per page. |
| `label` | Accessible name of the field. |
| `value` | Initial value. Without it, the first option. |
| `options` | `:options="path"` to an array of `{ value, label }`. |
| `toolparamdescription` | Fixed text for agents (no `:` or `{}`). |

- Arrow Up and Arrow Down move between options. Esc, outside click, and Back close the list.
- Copy `lib/components/select/` into a `p-` component to change its markup (icons, colors).
- Do not build a listbox with `[popover]`, `role="listbox"`, or a `let open` flag.
