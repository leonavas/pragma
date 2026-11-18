# CSS

Use utilitários. Crie classe custom só quando eles não bastam.
Referência: `exemplo/lib/theme.css`, `exemplo/lib/components/*/*.css`.

## Camadas

- `core/base.css`: preflight + `@layer reset, tokens, base, components, utilities`. Já
  inclui `.skeleton`, `[hidden]`, scroll lock (`html[data-scroll-lock]`) e `dialog::backdrop`.
  Não edite.
- `lib/theme.css`: tokens e classes custom globais. O do starter já cria `surface`, `ink`,
  `muted` e `line` (`bg-surface`, `text-ink`, `text-muted`, `border-line`), claro e escuro, e
  `.card`, `.btn`, `.btn-primary`, `.btn-danger`, `.dialog`.
- `<nome>.css` do componente: CSS só daquele componente, escopado por `[data-c="<nome>"]`.

## Utilitários

- Nomes do Tailwind v4, subconjunto fechado. Lista completa: `classes.md`.
- Só as classes usadas vão para o build.
- Variantes como `hover:`, `md:` e `dark:`: só as listadas em `classes.md`.
- Sem valores arbitrários: `w-[37px]`, `bg-[#f00]` = erro. Use a escala ou um token.
- Cores: `white`, `black`, `transparent`, `current` e
  `{neutral,primary,danger,success,warning}-{50…950}` (`text-`, `bg-`, `border-`, `ring-`…).
- Tema escuro: `data-theme="dark"` no `<html>` e variante `dark:`. Prefira tokens que mudam no
  `[data-theme="dark"]` (`text-ink`) a pares `dark:`.

## Exemplo canônico

`lib/theme.css`:
```css
@layer tokens {
  :root {
    --color-primary-600: oklch(0.55 0.2 260);
    --color-marca: oklch(0.7 0.15 150);
  }
  [data-theme="dark"] {
    --color-marca: oklch(0.6 0.12 150);
  }
}

@layer components {
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 0.5rem;
    background: var(--color-primary-600);
    color: var(--color-white);
  }
  .btn:hover { background: var(--color-primary-700); }
  .btn:disabled { opacity: 0.5; }
}
```

- `--color-<nome>` ou `--color-<nome>-<tom>` cria a cor; `bg-marca` e `text-marca` passam
  a existir.
- Redefinir `--color-primary-*` troca a cor da marca no app inteiro.

## Tema claro/escuro

`core/theme.js`: `theme()` → `'dark'|'light'`; `setTheme('dark'|'light')` aplica e grava
(`localStorage['pragma:theme']`). Se algum módulo importa `core/theme.js`, o build põe em toda
página um script inline que restaura o tema (ou segue `prefers-color-scheme`) antes do CSS, sem
piscar. Componente pronto: `exemplo/lib/components/tema/`.

```js
import { theme, setTheme } from '../../../core/theme.js'

export const state = () => ({
  escuro: theme() === 'dark',
  get rotulo() { return this.escuro ? 'Tema claro' : 'Tema escuro' },
})

export const actions = {
  alternar(ctx) {
    ctx.state.escuro = !ctx.state.escuro
    setTheme(ctx.state.escuro ? 'dark' : 'light')
  },
}
```

- Não escreva `data-theme` no `<html>` nem leia `localStorage` para o tema: use `core/theme.js`.

## Quando criar classe custom

- A mesma combinação de utilitários se repete em 3+ lugares (`.btn`, `.card`, `.campo`).
- Precisa de algo fora do subconjunto (seletor de estado, animação, `grid-template-areas`).
- Global em `lib/theme.css`, `@layer components`. De um componente só: no `.css` dele.

## Não faça

- Não use `style="..."`.
- Não invente nome de utilitário: classe fora de `classes.md` e sem definição no CSS do
  projeto = erro no `pragma check`.
- Não use cor ou medida solta repetida: crie token.
- Não crie classe custom para uso único: use utilitários.
- Não use `!important`, IDs como seletor nem `@import` de CSS externo.
