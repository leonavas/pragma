# Dados: load, actions de servidor, SSR, skeleton

A página abre na hora com skeleton; os dados chegam do `load` em paralelo.
Referência: `exemplo/routes/loja/` (`index.*` com skeleton e erro; `[id].*` com SSR e action).

## Exemplo canônico

`routes/loja/_catalogo.server.js` (privado: `_` no nome; importado só por `.server.js`):
```js
import produtos from './_produtos.json' with { type: 'json' }

export const listar = categoria => produtos.filter(p => !categoria || p.categoria === categoria)
export const achar = id => produtos.find(p => p.id === id) ?? null
```

`routes/loja/index.server.js`:
```js
import { listar } from './_catalogo.server.js'

export async function load({ query, log }) {
  const produtos = listar(query.categoria)
  log.debug('catálogo carregado', { 'produto.total': produtos.length })
  return { produtos }
}
```

`routes/loja/index.html`:
```html
<title>Loja</title>
<section class="flex flex-col gap-6">
  <h1 class="text-3xl font-semibold">Loja</h1>
  <template data-if="page.loading">
    <ul class="grid gap-4 sm:grid-cols-2" aria-busy="true" aria-label="Carregando produtos">
      <li class="skeleton h-48 rounded-2xl"></li>
      <li class="skeleton h-48 rounded-2xl"></li>
    </ul>
  </template>
  <template data-else>
    <div>
      <template data-if="page.error">
        <div class="card flex flex-col items-start gap-3" role="alert">
          <p class="text-danger-600" data-text="page.error.message"></p>
          <a href="/loja" class="btn">Tentar de novo</a>
        </div>
      </template>
      <template data-else>
        <ul class="grid gap-4 sm:grid-cols-2">
          <template data-for="p in data.produtos" data-key="p.id">
            <li class="card" data-text="p.nome"></li>
          </template>
          <template data-empty>
            <li class="card text-muted">Nenhum produto</li>
          </template>
        </ul>
      </template>
    </div>
  </template>
</section>
```

Action de servidor, `routes/loja/[id].server.js`:
```js
import { achar } from './_catalogo.server.js'

const curtidas = new Map()

export async function load({ params }) {
  const produto = achar(params.id)
  return { produto: produto && { ...produto, curtidas: curtidas.get(produto.id) ?? 0 } }
}

export const actions = {
  async curtir({ params, log }) {
    if (!achar(params.id)) throw new Error('Produto não encontrado.')
    const total = (curtidas.get(params.id) ?? 0) + 1
    curtidas.set(params.id, total)
    log.info('produto curtido', { 'produto.id': params.id })
    return { curtidas: total }
  },
}

export const ssr = true
```

`routes/loja/[id].js`:
```js
export const state = () => ({ enviando: false, erro: '' })

export const actions = {
  async curtir(ctx) {
    ctx.state.enviando = true
    try {
      ctx.data.produto.curtidas = (await ctx.call('curtir')).curtidas
    } catch (e) {
      ctx.log.error('falha ao curtir produto', { 'produto.id': ctx.params.id, 'exception.message': e.message })
      ctx.state.erro = 'Não foi possível registrar agora.'
    } finally {
      ctx.state.enviando = false
    }
  },
}
```

## `load`

- Assinatura: `load({ params, query, request, env, log })`. Retorna objeto JSON.
- O retorno vira `data` no template e `ctx.data` no JS.
- Lançar erro → `page.error` no cliente (`page.error.message`) e 500 com log `ERROR` no servidor.
- "Não encontrado" é dado (`{ produto: null }`), não erro.
- Fonte: API (`fetch` com URL e segredo de `env`), banco via `env`, ou JSON local com
  `import x from './_x.json' with { type: 'json' }`.
- Helper de servidor da rota: `routes/<pasta>/_nome.server.js`. Compartilhado: `utils/`.
- O build já dispara o fetch de `/_data/<caminho>` no `<head>`, antes dos módulos.

## Actions de servidor

- `actions.nome({ input, params, env, log })` no `.server.js`; chame com
  `ctx.call('nome', input)` na action do cliente. Retorno = JSON. Erro lançado = `ctx.call` rejeita.
- Use para toda escrita (criar, salvar, apagar) e para tudo que exige segredo.
- Depois da escrita, atualize `ctx.data` ou o estado com o retorno.

## Skeleton

- Toda página com `load` sem `ssr` tem `<template data-if="page.loading">` com `.skeleton` no
  formato do conteúdo, e o conteúdo no `data-else`; erro em `page.error` dentro dele.
- Parte fixa (título, voltar, filtros) fica fora do bloco de loading: aparece na hora.

## SSR

- Padrão: sem SSR (não exporte `ssr`).
- `export const ssr = true` só quando os dados precisam estar no primeiro HTML: SEO, preview de
  link, LCP que depende do dado. O HTML e as diretivas são os mesmos; sem skeleton.

## Não faça

- Não busque os dados da página com `fetch` no `mount` ou numa action: use `load`.
- Não crie rotas `/api/...`: use `load` e actions de servidor.
- Não devolva segredo, token ou dado de outro usuário no `load`: tudo vai para o cliente.
- Não leia `env` no cliente; `env` só existe no `.server.js`.
- Não importe `_x.server.js` em módulo de cliente.
- Não monte HTML no servidor: o servidor usa o mesmo `.html`.
