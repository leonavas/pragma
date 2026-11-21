# CSS

Use utilities (classes.md) first. Write a custom class only when they are not enough.

| File | Contains |
|---|---|
| `.pragma/base.css` | preflight, layers, `.skeleton`, `[hidden]`, scroll lock. Do not edit. |
| `lib/theme.css` | tokens and global custom classes (`@layer components`) |
| component `<style>` or `<name>.css` | that component only: the build scopes every selector to it (root and inside); `:global(…)` opts out; `[data-c="<name>"]` written by hand still works (for `p-` components the build makes it unique per folder) |
| page `<style>` or `<page>.css` | that page only: scoped to its `<main data-page="…">` the same way |

Scoping adds `:where(…)` to each selector's last part, so it adds no specificity. Write plain
selectors (`button`, `.row:hover`, `li::marker`). `:global(…)` opts out only what is inside it, for
something outside the component or page; the rest of the selector stays scoped:

| You write | The build makes | Matches |
|---|---|---|
| `.row` | `.row:where(<scope>,<scope> *)` | `.row` of this component only |
| `:global(html[data-theme="dark"]) .row` | `html[data-theme="dark"] .row:where(…)` | this component's `.row`, in dark mode |
| `.list :global(.hit)` | `.list:where(…) .hit` | any `.hit` inside this component's `.list` (child components too) |
| `:global(:root)`, `:global(body.locked)` | `:root`, `body.locked` | global: every part is inside `:global` |

An outside ancestor alone needs no `:global`: `html[data-theme="dark"] .row` gives the same result.

The starter ships the theme-aware tokens `bg-surface`, `text-ink`, `text-muted`, `border-line`
(they switch in dark mode) and the classes `.card`, `.btn`, `.btn-primary`, `.btn-danger`, `.dialog`.

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

- `--color-<name>` or `--color-<name>-<shade>` creates `bg-<name>`, `text-<name>-<shade>`, etc. Redefining `--color-primary-*` changes the brand.
- Redefinable tokens: `--color-*`, `--font-{sans,serif,mono}`, `--radius-*`, `--shadow-*`, `--spacing`.
- Dark mode: `[data-theme="dark"]` on `<html>`. Use tokens redefined under `[data-theme="dark"]` (like `bg-surface`). Palette colors (`neutral-*`, `primary-*`…) do not switch: pair them with `dark:` (`bg-white dark:bg-neutral-900`).

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

Never write `data-theme` or read `localStorage` for the theme: use `.pragma/theme.js`.

## Custom class

- Create one only when the same combination repeats in 3+ places, or it needs something utilities lack (state selector, animation).
- Forbidden: `style="..."`, arbitrary values (`w-[37px]`), negative utilities (`-mt-2`), undefined classes, `!important`, ID selectors, external `@import`, single-use custom classes.
