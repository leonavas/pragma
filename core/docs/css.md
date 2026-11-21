# CSS

Use utility classes (`classes.md`) first. Write a custom class only when utilities are not enough.

| File | Scope |
|---|---|
| `.pragma/base.css` | Preflight, layers, `.skeleton`, `[hidden]`, scroll lock. Do not edit. |
| `lib/theme.css` | Tokens and global custom classes (`@layer components`). |
| Component `<style>` or `<name>.css` | Scoped to that component (root and descendants). `:global(…)` opts out. `[data-c="<name>"]` stays as written (unique per folder for `p-`). |
| Page `<style>` or `<page>.css` | Scoped to that page's `<main data-page="…">`. |

Scoping appends `:where(…)` to each selector's last part, adding zero specificity. Write plain selectors (`button`, `.row:hover`, `li::marker`). Use `:global(…)` only for parts outside the component or page:

| Written | Compiled | Matches |
|---|---|---|
| `.row` | `.row:where(<scope>,<scope> *)` | `.row` inside this component only |
| `:global(html[data-theme="dark"]) .row` | `html[data-theme="dark"] .row:where(…)` | This component's `.row` in dark mode |
| `.list :global(.hit)` | `.list:where(…) .hit` | Any `.hit` inside this component's `.list` (including child components) |
| `:global(:root)`, `:global(body.locked)` | `:root`, `body.locked` | Global |

An outside ancestor alone needs no `:global`: `html[data-theme="dark"] .row` compiles identically.

## Tokens and dark mode

The starter includes theme-aware tokens (`bg-surface`, `text-ink`, `text-muted`, `border-line`) and classes (`.card`, `.btn`, `.btn-primary`, `.btn-danger`, `.dialog`).

```css
@layer tokens {
  :root {
    --color-primary-600: oklch(0.55 0.2 260);
    --color-brand: oklch(0.7 0.15 150);
  }
  [data-theme="dark"] {
    --color-brand: oklch(0.6 0.12 150);
  }
}

@layer components {
  .btn:hover { background: var(--color-primary-700); }
}
```

- `--color-<name>` and `--color-<name>-<shade>` generate `bg-<name>`, `text-<name>-<shade>`, and related utilities. Redefine `--color-primary-*` to change the brand color.
- Redefinable tokens: `--color-*`, `--font-{sans,serif,mono}`, `--radius-*`, `--shadow-*`, `--spacing`.
- Dark mode uses `[data-theme="dark"]` on `<html>`. Use theme-aware tokens (such as `bg-surface`), or pair palette colors with `dark:` (`bg-white dark:bg-neutral-900`).

## Theme toggle

```js
import { theme, setTheme } from '../../../.pragma/theme.js'

let dark = theme() === 'dark'

const label = () => (dark ? 'Light theme' : 'Dark theme')

function toggle() {
  dark = !dark
  setTheme(dark ? 'dark' : 'light')
}
```

Do not write `data-theme` or read `localStorage` for theme state. Use `.pragma/theme.js`.

## Values from state: CSS variables

Use `:--name="path"` when a value comes from state or data (position, width, color, column count). Write the CSS once, with `var(--name)`:

```html
<div class="timeline">
  <for shift in $shifts key=shift.id>
    <div class="shift" :--start="shift.start" :--span="shift.span" :--tone="shift.color">{shift.label}</div>
  </for>
</div>
<ul class="grid" :--cols="$columns">…</ul>
<div class="pill" :--i="$selected" :--n="$options.length">…</div>
```

```css
.timeline { position: relative; }
.shift { position: absolute; left: calc(var(--start) * 1%); width: calc(var(--span) * 1%); background: var(--tone, var(--color-primary-600)); }
.grid { display: grid; grid-template-columns: repeat(var(--cols, 3), minmax(0, 1fr)); }
.pill::before { width: calc(100% / var(--n)); translate: calc(var(--i) * 100%) 0; }
```

- Pass numbers without units (`42`) and add the unit in CSS (`* 1%`, `* 1px`). Compute the number in a script function (`const start = shift => (shift.from - dayStart) / dayLength * 100`).
- `false`, `null`, `undefined`, and `""` remove the variable; use a fallback (`var(--tone, …)`).
- On a component tag, `:--name` goes to the component root.
- Values set every frame (scroll, drag, progress) update only their bindings, not the page (`state.md`, "Per-frame updates").
- The server writes `style="--start:42"` in SSR pages. Do not write `style="..."` yourself (`html/style-inline`).

## Custom classes

- Create a custom class only when the same combination repeats in 3+ places, or when you need state selectors or animations.
- Forbidden: inline `style="..."` and `:style` (use `:--name`), arbitrary values (`w-[37px]`), negative utilities (`-mt-2`), undefined classes, `!important`, ID selectors, external `@import`, and single-use custom classes.
