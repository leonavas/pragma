# Dados: load, actions de servidor, skeleton, SSR

| Arquivo | Export | Uso |
|---|---|---|
| `x.server.js` | `load({ params, query, request, env, log })` | retorno vira `data` / `ctx.data` |
| `x.server.js` | `actions.nome({ input, params, env, log })` | chamada por `ctx.call('nome', input)` |
| `x.server.js` | `ssr = true` | dados no primeiro HTML (SEO, preview, LCP) |
| `_nome.server.js` | helpers | privado da pasta; só `.server.js` importa |

```js
import { achar } from './_catalogo.server.js'

const curtidas = new Map()

export async function load({ params }) {
  return { produto: achar(params.id) }
}

export const actions = {
  async curtir({ params, log }) {
    const total = (curtidas.get(params.id) ?? 0) + 1
    curtidas.set(params.id, total)
    log.info('produto curtido', { 'produto.id': params.id })
    return { curtidas: total }
  },
}
```

```js
export const state = () => ({ enviando: false, erro: '' })

export const actions = {
  async curtir(ctx) {
    ctx.$.enviando = true
    try {
      ctx.data.produto.curtidas = (await ctx.call('curtir')).curtidas
    } catch (erro) {
      ctx.log.error('falha ao curtir produto', { 'produto.id': ctx.params.id, 'exception.message': erro.message })
      ctx.$.erro = 'Não foi possível registrar agora.'
    } finally {
      ctx.$.enviando = false
    }
  },
}
```

## Skeleton (página com `load` sem `ssr`)

```html
<h1 class="text-3xl font-semibold">Loja</h1>
<template :if="page.loading">
  <ul aria-busy="true" aria-label="Carregando produtos">
    <li class="skeleton h-48 rounded-2xl"></li>
  </ul>
</template>
<template :else>
  <div>
    <template :if="page.error">
      <p role="alert" :text="page.error.message"></p>
    </template>
    <template :else>
      <ul>
        <template :for="p in data.produtos" :key="p.id">
          <li class="card" :text="p.nome"></li>
        </template>
      </ul>
    </template>
  </div>
</template>
```

Título, voltar e filtros ficam fora do bloco de loading.

## Regras

- Erro lançado no `load` → `page.error` e 500. "Não encontrado" é dado (`{ produto: null }`).
- Toda escrita e tudo com segredo: action de servidor. Depois, atualize `ctx.data` ou `$`.
- Fonte: `fetch` com segredo de `env`, banco via `env` ou `import x from './_x.json' with { type: 'json' }`.
- Padrão sem SSR. Com `ssr = true` o HTML é o mesmo, sem skeleton.
- Não busque dados no `mount` nem em action; não crie `/api/...`; não devolva segredo no `load`;
  `env` só no servidor.
