# CSS

Use utilities (classes.md) first. Write a custom class only when they are not enough.

| File | Contains |
|---|---|
| `core/base.css` | preflight, layers, `.skeleton`, `[hidden]`, scroll lock. Do not edit. |
| `lib/theme.css` | tokens and global custom classes (`@layer components`) |
| component `<name>.css` | that component only; scope selectors with `[data-c="<name>"]` (for `p-` components the build makes it unique per folder) |

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
import { theme, setTheme } from '../../../core/theme.js'

export const state = () => ({
  dark: theme() === 'dark',
  get label() { return this.dark ? 'Light theme' : 'Dark theme' },
})

export const actions = {
  toggle(ctx) {
    ctx.$.dark = !ctx.$.dark
    setTheme(ctx.$.dark ? 'dark' : 'light')
  },
}
```

Never write `data-theme` or read `localStorage` for the theme: use `core/theme.js`.

## Custom class

- Create one only when the same combination repeats in 3+ places, or it needs something utilities lack (state selector, animation).
- Forbidden: `style="..."`, arbitrary values (`w-[37px]`), negative utilities (`-mt-2`), undefined classes, `!important`, ID selectors, external `@import`, single-use custom classes.
