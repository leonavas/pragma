# Estado: state, actions, mount, stores

Mude o estado; o runtime reaplica as diretivas. Não há signals nem render manual.
Referência completa: `exemplo/routes/tarefas/index.js`, `exemplo/lib/state/tarefas.js`.

## Módulo de página ou componente

`routes/tarefas/index.js` (par do HTML de `diretivas.md`):
```js
export const state = ({ query, shared }) => ({
  nova: '',
  erro: '',
  get visiveis() {
    const so = query.filtro === 'pendentes'
    return shared.tarefas.itens.filter(t => !(so && t.feita))
  },
})

export const actions = {
  adicionar(ctx) {
    const titulo = ctx.state.nova.trim()
    ctx.state.erro = titulo.length < 3 ? 'Use ao menos 3 caracteres.' : ''
    if (ctx.state.erro) return
    ctx.shared.tarefas.adicionar(titulo)
    ctx.state.nova = ''
  },
  alternar(ctx) {
    const t = ctx.shared.tarefas.achar(ctx.scope.t.id)
    t.feita = !t.feita
  },
}
```

`lib/state/tarefas.js`:
```js
import { store } from '../../core/state.js'

export default store({
  itens: [],
  get pendentes() { return this.itens.filter(t => !t.feita).length },
  adicionar(titulo) { this.itens.push({ id: crypto.randomUUID(), titulo, feita: false }) },
  achar(id) { return this.itens.find(t => t.id === id) },
}, { persist: 'tarefas' })
```

`lib/components/relogio/relogio.js` (`mount`):
```js
const agora = () => new Date().toLocaleTimeString('pt-BR')

export const state = () => ({ hora: agora() })

export function mount(ctx) {
  const t = setInterval(() => ctx.update(s => { s.hora = agora() }), 1000)
  return () => clearInterval(t)
}
```

## Exports

| Export | Para quê |
|---|---|
| `state(ctx)` | retorna o estado inicial (um por instância) |
| `actions` | funções `(ctx)`, sync ou async, chamadas por `data-on-*` |
| `mount(ctx)` | só efeitos externos (timer, listener global, observer); retorna o cleanup |

## `state(ctx)` e getters

- `state` recebe o ctx do dono: `shared`, `params`, `query`, `page`, `props`, `data`, `el`, `log`.
  Sem `event`, `scope` nem `ctx.state`.
- Valor derivado (total, rótulo, lista filtrada, flag) = getter no objeto de estado ou no store.
  O getter roda a cada render; não guarde cópia atualizada pela action.
- No getter, use `this` para o próprio estado e o ctx capturado para o resto.
- `ctx.data` só existe depois do load: getter que o lê só aparece dentro do bloco pós-load
  (`data-else` de `page.loading`), ou em página `ssr`.
- Getter de store não vai para o `localStorage`.

## `ctx`

| Campo | Uso |
|---|---|
| `state` `data` `shared` | estado do dono, dados do `load`, stores |
| `params` `query` | path params e query string |
| `page` | `page.loading`, `page.error` |
| `props` | props do componente (strings; ver `componentes.md`) |
| `event` `el` | evento DOM e elemento que disparou a action |
| `scope` | variáveis do `data-for` / aliases no ponto do evento |
| `update(fn)` | mudar estado fora de action: `ctx.update(s => { s.x = 1 })` |
| `call(nome, input)` | action de servidor (ver `dados.md`) |
| `navigate(url)` | navegar pelo router |
| `open(id)` `close(id)` `confirm(id)` | overlays (ver `overlays.md`) |
| `log` | logs (ver `logs.md`) |

## Regras

- Depois de cada action (inclusive async) o runtime reaplica as diretivas.
- Fora de action (timer, callback, promise solta), mude estado só com `ctx.update(fn)`.
- Estado de uma página: `state`. Estado entre páginas ou persistido: store em `lib/state/`.
- Mutação usada por 2+ páginas: método no store (`ctx.shared.tarefas.adicionar(t)`).
- Filtro, aba, página da lista: na URL (`query`), lida num getter.
- `persist: 'chave'` grava o store no `localStorage`. Só preferências, rascunhos e dados locais;
  dado de verdade fica no servidor.
- Todo `mount` que cria timer, listener ou observer retorna o cleanup.

## Não faça

- Não escreva no DOM (`textContent`, `classList`, `style`, `setAttribute`): mude o estado.
- Não use `addEventListener` em elementos da página: use `data-on-*`. Em `window` e
  `document`, só dentro de `mount`, com cleanup.
- Não use `mount` para buscar dados da página: use `load` (ver `dados.md`).
- Não crie sistemas de evento, pub/sub ou estado global próprios: use stores.
