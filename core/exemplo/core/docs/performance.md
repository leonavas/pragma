# Performance

O padrão já é rápido. Não estrague.

## O que o pragma faz sozinho

- HTML pré-gerado: a página pinta sem esperar JS.
- Fetch do `load` disparado no `<head>`, em paralelo com os módulos.
- Cada página baixa só o JS dela e dos componentes que usa. `modulepreload` só do fecho dos
  imports estáticos; `import()` dinâmico fica fora.
- `core/overlay.js` só carrega (e só tem preload) em página com `<dialog>` ou `[popover]`.
- Prefetch de links no hover, foco e viewport.
- Assets com hash e cache `immutable`; CSS só com as classes usadas.
- Orçamentos de `pragma.json` checados no `build` e no `check`.

## Faça

- Navegue com `<a href>`: é o que ganha prefetch.
- Mostre skeleton no formato do conteúdo final: sem salto de layout.
- `<img>` com `width` e `height`; `loading="lazy"` abaixo da dobra; formatos modernos
  (AVIF, WebP) em `public/`.
- Listas com `data-for` e `data-key` estável: o diff reaproveita os nós.
- Página sem interação não tem `.js`.
- Código pesado usado por uma página só fica no módulo dessa página ou num componente
  usado só nela, não em `lib/state/` nem em `utils/` importado por todo lado.
- Código que só roda depois de uma interação (editor, gráfico, parser): `await import('...')`
  dentro da action.
- Dado do `load` só com o que a página mostra.

## Não faça

- Não estoure o orçamento aumentando o limite em `pragma.json`: reduza o código.
- Não importe um módulo grande em `lib/state/`: stores carregam em toda página que os usa.
- Não faça `await` em sequência quando as chamadas são independentes: `Promise.all`.
- Não busque dados no `mount`: atrasa a página e duplica o `load`.
- Não esconda conteúdo pronto atrás de JS; o que pode vir no HTML vem no HTML.
- Não ligue `ssr = true` por padrão: só quando o dado precisa estar no primeiro HTML.
