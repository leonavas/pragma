# CSS

Utilitários primeiro (`classes.md`). Classe custom só quando eles não bastam.

| Arquivo | Conteúdo |
|---|---|
| `core/base.css` | preflight, camadas, `.skeleton`, `[hidden]`, scroll lock. Não edite. |
| `lib/theme.css` | tokens e classes custom globais (`@layer components`) |
| `<nome>.css` do componente | só daquele componente, escopo `[data-c="<nome>"]` |

O starter já traz `bg-surface`, `text-ink`, `text-muted`, `border-line` (claro e escuro) e
`.card`, `.btn`, `.btn-primary`, `.btn-danger`, `.dialog`.

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
  .btn:hover { background: var(--color-primary-700); }
}
```

- `--color-<nome>` cria `bg-<nome>`, `text-<nome>`… Redefinir `--color-primary-*` troca a marca.
- Prefira token que muda no `[data-theme="dark"]` a pares `dark:`.

## Tema

```js
import { theme, setTheme } from '../../../core/theme.js'

export const state = () => ({
  escuro: theme() === 'dark',
  get rotulo() { return this.escuro ? 'Tema claro' : 'Tema escuro' },
})

export const actions = {
  alternar(ctx) {
    ctx.$.escuro = !ctx.$.escuro
    setTheme(ctx.$.escuro ? 'dark' : 'light')
  },
}
```

Não escreva `data-theme` nem leia `localStorage` para o tema: use `core/theme.js`.

## Classe custom

- Mesma combinação em 3+ lugares, ou algo fora do subconjunto (seletor de estado, animação).
- Proibido: `style="..."`, valor arbitrário (`w-[37px]`), classe inventada, `!important`, ID
  como seletor, `@import` externo, classe custom de uso único.
