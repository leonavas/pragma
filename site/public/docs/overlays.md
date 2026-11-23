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
| `c-modal` | `id`, `title`, `autofocus` | default, `actions` | Esc, backdrop, ×, `@close`, Back |
| `c-alert` | `id`, `title`, `autofocus` | default, `actions` | Esc, `@close`, Back |
| `c-dropdown` | `id`, `label`, `placement` | default (`<a>`, `<button>`, `<hr>`) | outside click, Esc, `@close`, Back |

`c-dropdown` is a menu of links and actions. It sends no value to a form. To pick a form value from a styled list, use `c-select` (`forms.md`).

| JS (`import { open, close, confirm } from 'pragma'`) | Effect |
|---|---|
| `open(id)` / `close(id)` | Opens or closes the overlay. |
| `await confirm(id)` | Opens the overlay and resolves to the clicked `@close` button's `value`, or `'cancel'`. |

## Placement

`placement` on `c-dropdown`: `bottom-start` (default), `bottom-end`, `top-start`, `top-end`. The side of the trigger, then the shared edge (`end`: right edges line up in LTR). With no room, the menu flips to the other side or edge.

```html
<c-dropdown id="send-as" label="Agent" placement="top-end">…</c-dropdown>
```

## Focus

On open, focus goes to the element with `autofocus` inside the overlay, else to the first focusable element. For long or text-first content, put `autofocus` on the component (`<c-modal id="terms" title="Terms" autofocus>`): the dialog itself gets focus and the reader starts at the top. On close, focus returns to the opener.

## Motion

Opening animates with `.dialog[open]` (or `@starting-style`). Closing is CSS too: the runtime sets `data-leaving` on the `<dialog>` and makes it inert, waits for its CSS animations (also `::backdrop`) to end, then closes it. No animation rule or `prefers-reduced-motion` → it closes at once. History, scroll lock and `confirm()` resolve when closing starts; focus returns after the dialog is closed. The starter `.dialog` fades out. A side sheet is a `c-modal` with a class (`<c-modal id="person" title="Ana" class="sheet">`) that slides out:

```css
.sheet { margin: 0 0 0 auto; height: 100dvh; max-height: none; }
.sheet[open] { animation: sheet-in 0.22s ease-out; }
.sheet[data-leaving] { animation: sheet-out 0.14s ease-in forwards; }
@keyframes sheet-in { from { translate: 100% 0; } }
@keyframes sheet-out { to { translate: 100% 0; } }
@media (prefers-reduced-motion: reduce) { .sheet[open], .sheet[data-leaving] { animation: none; } }
```

Use `forwards` on exit animations, so the dialog does not flash back before it closes.

## Content renders on first open

A closed overlay costs no DOM work: its content (blocks, components, bindings) renders the first time it opens and stays rendered after that, so state inside it survives a close. Components inside it run `mount` on that first open, not with the page. Put a heavy summary or a long list in a modal without an `<if>` around it.

## Rules

- Back closes the top overlay, not the page. Overlays stack.
- Modal and alert lock scroll; dropdown does not. Focus returns to the opener.
- Put `autofocus` on the safe button in destructive confirmations.
- Open overlays only after a user click.
- Do not write `<dialog>` or `[popover]` by hand. Do not call `showModal()` or `window.confirm`. Do not store open state in a `let`. Do not use `role="menu"` or import `.pragma/overlay.js`.
