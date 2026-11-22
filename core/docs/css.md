# CSS

Use utility classes (`classes.md`) first. Write a custom class only when utilities are not enough.

| File | Scope |
|---|---|
| `.pragma/base.css` | Preflight, layers, `.skeleton`, `[hidden]`, scroll lock. Do not edit. |
| `lib/theme.css` | Tokens and global custom classes (`@layer components`). |
| Component `<style>` or `<name>.css` | Scoped to the markup that component wrote. `:global(…)` opts out. `[data-c="<name>"]` stays as written (unique per folder for `p-`). |
| Page `<style>` or `<page>.css` | Scoped to the markup that page wrote. |

Scoping works as in Svelte. The build writes a `data-s` token on each element that a page or component wrote, and appends `:where([data-s~="<token>"])` to the last part of each selector (zero specificity). A selector does not match:

- elements inside a child component (style them in that component, or pass a class),
- slot content: it belongs to the page or component that wrote it.

A component's root element also belongs to the caller when the caller gives it a `class` (`<c-card class="wide">` + `.wide { … }` in the page).

Write plain selectors (`button`, `.row:hover`, `li::marker`). Use `:global(…)` only for parts outside the owner's markup:

| Written | Compiled | Matches |
|---|---|---|
| `.row` | `.row:where([data-s~="<token>"])` | `.row` written in this component only |
| `:global(html[data-theme="dark"]) .row` | `html[data-theme="dark"] .row:where(…)` | This component's `.row` in dark mode |
| `.list :global(.hit)` | `.list:where(…) .hit` | Any `.hit` inside this component's `.list` (slot content, child components) |
| `[popover] > :global(a)` | `[popover]:where(…) > a` | Slot links directly in this component's `[popover]` |
| `:global(:root)`, `:global(body.locked)` | `:root`, `body.locked` | Global |

An outside ancestor alone needs no `:global`: `html[data-theme="dark"] .row` compiles identically.

## Cascade order

`@layer reset, tokens, base, components, utilities`, then component and page styles outside any layer:

1. `lib/theme.css` custom classes (`@layer components`)
2. utilities (`@layer utilities`): beat theme classes
3. component and page `<style>`: beat utilities on the same element (`<div class="flex">` + `.panel { display: grid }` = grid)

Theme rules outside a layer compete with component styles by specificity and order. Put theme rules in `@layer components` or `@layer base`.

A custom class can have the name of a utility in component or page CSS (`.mural > .contents`); the utility is still generated. In `lib/theme.css`, a rule whose whole selector is a utility name (`.flex { … }`, `.hidden:hover { … }`) is a `css/utility-name` error: rename the class.

## What each page loads

Each page gets only the CSS its markup can use:

- `lib/theme.css`: rules whose selectors can match the route's elements and classes (static `class` and `.x` toggles). Rules with only attributes, ids, pseudo-classes, `:root`, `html` or `body` stay. `@keyframes` and `@font-face` stay when the kept CSS names them.
- Component and page CSS of the components on the page, without rules for classes that never appear on the owner's elements.
- Utilities used on the page.

Markup that pragma does not render (HTML from an npm widget) gets no theme rule for its classes. Style it in the component that hosts it with `:global(…)`.

Fonts: the build adds `<link rel="preload" as="font" crossorigin>` for each `@font-face` family that the page's CSS uses (directly or through `var(--font-…)`): the upright `.woff2` face that covers Latin letters (weight 400 when there are several). Remove manual font preloads from `_layout.html`: they load fonts on pages that do not use them.

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
