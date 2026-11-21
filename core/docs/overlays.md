# Overlays: modal, alert, dropdown

Use `c-modal`, `c-alert`, and `c-dropdown` for all overlays. The runtime handles open, close, Esc, Back, focus, and scroll lock.

```html
<button type="button" class="btn" @open="about">About</button>
<c-modal id="about" title="About the app">
  <p>Content.</p>
  <button slot="actions" type="button" class="btn btn-primary" @close value="ok">Close</button>
</c-modal>

<button type="button" class="btn" @click="remove">Delete</button>
<c-alert id="remove" title="Delete project?">
  <p>This cannot be undone.</p>
  <button slot="actions" type="button" class="btn" @close value="cancel" autofocus>Cancel</button>
  <button slot="actions" type="button" class="btn btn-danger" @close value="ok">Delete</button>
</c-alert>

<c-dropdown id="account" label="Account">
  <a href="/profile">Profile</a>
  <hr>
  <button type="button" @click="logout">Log out</button>
</c-dropdown>
```

```js
import { confirm, call, params } from 'pragma'

async function remove() {
  if (await confirm('remove') !== 'ok') return
  await call('remove', { id: params.id })
}
```

| Component | Props | Slots | Closes on |
|---|---|---|---|
| `c-modal` | `id`, `title` | default, `actions` | Esc, backdrop, ×, `@close`, Back |
| `c-alert` | `id`, `title` | default, `actions` | Esc, `@close`, Back |
| `c-dropdown` | `id`, `label` | default (`<a>`, `<button>`, `<hr>`) | outside click, Esc, `@close`, Back |

| JS (`import { open, close, confirm } from 'pragma'`) | Effect |
|---|---|
| `open(id)` / `close(id)` | Opens or closes the overlay. |
| `await confirm(id)` | Opens the overlay and resolves to the clicked `@close` button's `value`, or `'cancel'`. |

## Rules

- Back closes the top overlay, not the page. Overlays stack.
- Modal and alert lock scroll; dropdown does not. Focus returns to the opener.
- Put `autofocus` on the safe button in destructive confirmations.
- Open overlays only after a user click.
- Do not write `<dialog>` or `[popover]` by hand. Do not call `showModal()` or `window.confirm`. Do not store open state in a `let`. Do not use `role="menu"` or import `.pragma/overlay.js`.
