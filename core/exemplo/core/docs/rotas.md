# Rotas e router

Referência: `exemplo/routes/`.

## Arquivo → rota

| Arquivo | Rota |
|---|---|
| `routes/index.html` | `/` |
| `routes/sobre.html` | `/sobre` |
| `routes/loja/index.html` | `/loja` |
| `routes/loja/[id].html` | `/loja/:id` |
| `routes/[slug]/fotos.html` | `/:slug/fotos` |

- Todo `.html` em `routes/` é rota, menos nomes e pastas que começam com `_`.
- `x.html` e `x/index.html` juntos = erro de build. Escolha um.
- `[nome]` (arquivo ou pasta) = path param. Rota estática vence param.
- Não existe `[...rest]` nem layout aninhado.

## Página = fragmento

A página não tem `<html>`, `<head>` nem `<body>`. Ela entra no `<slot>` do `_layout.html`.
Um `<title>` no topo vira o título.

`routes/loja/[id].html`:
```html
<title>Produto</title>
<section class="flex flex-col gap-4">
  <a href="/loja">← Voltar para a loja</a>
  <h1 class="text-2xl font-bold" data-text="data.produto.nome"></h1>
  <p data-show="query.destaque">Em destaque</p>
</section>
```

## Layout

`routes/_layout.html` (completo em `exemplo/routes/_layout.html`):
```html
<head>
  <link rel="icon" href="/favicon.svg" type="image/svg+xml">
</head>
<header>
  <nav class="flex gap-6 p-4" aria-label="Principal">
    <a href="/" class="nav-link">Início</a>
    <a href="/loja" class="nav-link">Loja</a>
    <span data-text="shared.carrinho.quantidade"></span>
  </nav>
</header>
<main data-page><slot></slot></main>
```

- `<head>` opcional, só no início: tags extras do `<head>` de toda página.
- Exatamente um `<main data-page>`, contendo só `<slot></slot>`.
- O layout não tem módulo JS. Suas diretivas usam só `shared`, `params` e `query`.
  Comportamento no layout: componente com `.js`.
- `routes/_404.html` é a página de rota inexistente (sem `.server.js`).

## Link ativo: `aria-current`

- Link `href="/caminho"` (sem query nem hash) para a página atual recebe
  `aria-current="page"` sozinho: no build e a cada navegação.
- Estilo do ativo: classe custom em `lib/theme.css` (não há variante `aria-current:`):
```css
@layer components {
  .nav-link { color: var(--color-muted); }
  .nav-link[aria-current="page"] { color: var(--color-ink); font-weight: 600; }
}
```
- Não escreva `aria-current="page"` à mão nem calcule link ativo no JS.
- Filtro na query (`?filtro=x`): o build não marca. Use `data-attr-aria-current` com getter que
  devolve `'true'` ou `false` (`exemplo/routes/tarefas/index.js`, `marcar` em
  `exemplo/utils/texto.js`):
```html
<a href="?filtro=pendentes" data-attr-aria-current="state.filtro.pendentes">Pendentes</a>
```
```js
get filtro() { return { todas: !query.filtro && 'true', pendentes: query.filtro === 'pendentes' && 'true' } },
```

## Params e query

- Template: `params.id`, `query.filtro`. Sempre strings.
- JS: `ctx.params.id`, `ctx.query.filtro`.
- Servidor: `load({ params, query })` e `actions.x({ params })` (ver `dados.md`).

## Navegação

- Navegue com `<a href="...">`. O router intercepta, faz prefetch no hover, foco e
  viewport, troca só o `<main data-page>` e mantém o layout.
- Cada navegação (inclusive só de query) desmonta a página e recria `state`.
- Em JS (depois de uma action): `ctx.navigate('/loja?filtro=promo')`.
- Carga completa: `<a href="/sair" data-reload>`. Links com `target` ou `download` também
  não passam pelo router.
- Filtro ou aba que deve sobreviver a refresh e deep link: query com `<a href="?filtro=x">`.

## Não faça

- Não use `<button>` com `ctx.navigate` no lugar de `<a>`: perde prefetch, abrir em nova
  aba e acessibilidade.
- Não use `location.href = ...` nem `history.pushState` direto.
- Não leia `location` para pegar params; use `params` e `query`.
