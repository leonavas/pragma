# Estado: state, actions, mount, stores

Mude o estado; o runtime reaplica o HTML. Sem signals nem render manual.

| Export | Para quê |
|---|---|
| `state(ctx)` | estado inicial, um por instância; getters para valores calculados |
| `actions` | funções `(ctx)`, sync ou async, chamadas por `@evento` |
| `mount(ctx)` | só efeito externo (timer, listener global, observer); retorna o cleanup |

```js
export const state = ctx => ({
  nova: '',
  erro: '',
  get visiveis() {
    const soPendentes = ctx.query.filtro === 'pendentes'
    return ctx.shared.tarefas.itens.filter(t => !(soPendentes && t.feita))
  },
})

export const actions = {
  adicionar(ctx) {
    const titulo = ctx.$.nova.trim()
    ctx.$.erro = titulo.length < 3 ? 'Use ao menos 3 caracteres.' : ''
    if (ctx.$.erro) return
    ctx.shared.tarefas.adicionar(titulo)
    ctx.$.nova = ''
  },
  alternar(ctx) {
    const tarefa = ctx.shared.tarefas.achar(ctx.scope.t.id)
    tarefa.feita = !tarefa.feita
  },
}

export function mount(ctx) {
  const intervalo = setInterval(() => ctx.update(estado => { estado.hora = Date.now() }), 1000)
  return () => clearInterval(intervalo)
}
```

## `ctx`

| Campo | Uso |
|---|---|
| `$` | estado + props do dono (props só leitura) |
| `data` `shared` | retorno do `load`, stores |
| `params` `query` `page` | rota, query, `page.loading`/`page.error` |
| `event` `el` `scope` | evento, elemento e item do `:for` (só em action) |
| `update(fn)` | mudar estado fora de action |
| `call(nome, input)` | action de servidor (`dados.md`) |
| `navigate(url)` | navegar pelo router |
| `open(id)` `close(id)` `confirm(id)` | overlays (`overlays.md`) |
| `log` | logs (`logs.md`) |

## Store

`lib/state/tarefas.js`, lido como `shared.tarefas` no HTML e `ctx.shared.tarefas` no JS:
```js
import { store } from '../../core/state.js'

export default store({
  itens: [],
  get pendentes() { return this.itens.filter(t => !t.feita).length },
  adicionar(titulo) { this.itens.push({ id: crypto.randomUUID(), titulo, feita: false }) },
  achar(id) { return this.itens.find(t => t.id === id) },
}, { persist: 'tarefas' })
```

## Regras

- Getter roda a cada render: use `this.x` para o próprio estado e `ctx` para o resto. Não guarde
  cópia calculada pela action.
- `ctx.data` só existe depois do load: getter que o lê fica dentro do `:else` de `page.loading`.
- Fora de action (timer, promise solta), mude estado só com `ctx.update(fn)`.
- Estado da página: `state`. Entre páginas ou persistido: store. Mutação de 2+ páginas: método do store.
- Filtro, aba, paginação: na `query`, lida num getter.
- `persist` só para preferência, rascunho e dado local.
- Não escreva no DOM (`textContent`, `classList`, `setAttribute`), não use `addEventListener` em
  elemento da página (só `window`/`document` dentro de `mount`), não busque dados no `mount`,
  não crie pub/sub próprio.
