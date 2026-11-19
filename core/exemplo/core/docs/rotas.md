# Rotas e router

| Arquivo | Rota |
|---|---|
| `routes/index.html` | `/` |
| `routes/sobre.html` | `/sobre` |
| `routes/loja/index.html` | `/loja` |
| `routes/loja/[id].html` | `/loja/:id` |

- Todo `.html` em `routes/` é rota, menos nomes que começam com `_`. `x.html` + `x/index.html` = erro.
- `[nome]` = path param; estático vence param. Sem `[...rest]` nem layout aninhado.
- Página é fragmento: sem `<html>`, `<head>`, `<body>`. `<title>` no topo vira o título.
- `params.id` e `query.filtro` no HTML; `ctx.params`, `ctx.query` no JS. Sempre strings.

## Layout

`routes/_layout.html`:
```html
<head>
  <link rel="icon" href="/favicon.svg" type="image/svg+xml">
</head>
<header>
  <nav aria-label="Principal">
    <a href="/" class="nav-link">Início</a>
    <a href="/loja" class="nav-link">Loja</a>
    <span :text="shared.carrinho.quantidade"></span>
  </nav>
</header>
<main data-page><slot></slot></main>
```

- `<head>` opcional, só no início. Exatamente um `<main data-page>` com só `<slot></slot>`.
- Layout sem `.js`: só `shared`, `params` e `query`. Comportamento: componente com `.js`.
- `routes/_404.html` = rota inexistente.

## Link ativo

- `<a href="/caminho">` da página atual ganha `aria-current="page"` sozinho. Estilo:
  `.nav-link[aria-current="page"]` em `lib/theme.css`.
- Filtro na query: `:aria-current` com getter que devolve `'true'` ou `false`.

```html
<a href="?filtro=pendentes" :aria-current="$filtro.pendentes">Pendentes</a>
```
```js
export const state = ctx => ({
  get filtro() { return { todas: !ctx.query.filtro && 'true', pendentes: ctx.query.filtro === 'pendentes' && 'true' } },
})
```

## Navegação

- Use `<a href>`: o router troca só o `<main data-page>`, faz prefetch e mantém o layout.
- Cada navegação (até só de query) recria o estado da página.
- No JS, depois de uma action: `ctx.navigate('/loja?filtro=promo')`.
- Carga completa: `<a href="/sair" data-reload>`. `target` e `download` também saem do router.
- Não use `<button>` + `navigate` no lugar de `<a>`, `location.href`, `history.pushState` nem `location` para ler params.
