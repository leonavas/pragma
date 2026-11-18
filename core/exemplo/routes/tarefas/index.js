import { contem, marcar } from '../../utils/texto.js'

const PASSA = { todas: () => true, pendentes: t => !t.feita, concluidas: t => t.feita }

function validar(tarefas, titulo, id) {
  if (titulo.length < 3) return 'Use ao menos 3 caracteres.'
  if (titulo.length > 80) return 'Use no máximo 80 caracteres.'
  if (tarefas.existe(titulo, id)) return 'Essa tarefa já existe.'
  return ''
}

export const state = ({ query, shared }) => ({
  nova: '',
  erro: '',
  busca: '',
  editando: null,
  rascunho: '',
  get invalida() { return String(!!this.erro) },
  get tom() { return shared.tarefas.concluidas && !shared.tarefas.pendentes ? 'ok' : 'neutro' },
  get filtro() { return marcar(Object.keys(PASSA), PASSA[query.filtro] ? query.filtro : 'todas') },
  get visiveis() {
    const passa = PASSA[query.filtro] ?? PASSA.todas
    return shared.tarefas.itens
      .filter(t => passa(t) && contem(t.titulo, this.busca))
      .map(t => ({ ...t, editando: t.id === this.editando, rotuloRemover: `Remover ${t.titulo}` }))
  },
})

export const actions = {
  adicionar(ctx) {
    const titulo = ctx.state.nova.trim()
    ctx.state.erro = validar(ctx.shared.tarefas, titulo)
    if (ctx.state.erro) return
    ctx.shared.tarefas.adicionar(titulo)
    ctx.state.nova = ''
    ctx.el.elements.titulo.focus()
  },
  alternar(ctx) {
    const t = ctx.shared.tarefas.achar(ctx.scope.t.id)
    t.feita = !t.feita
  },
  remover(ctx) { ctx.shared.tarefas.remover(ctx.scope.t.id) },
  editar(ctx) {
    ctx.state.editando = ctx.scope.t.id
    ctx.state.rascunho = ctx.scope.t.titulo
  },
  teclaEdicao(ctx) {
    if (ctx.event.key === 'Escape') ctx.state.editando = null
    if (ctx.event.key === 'Enter') salvar(ctx)
  },
  salvarEdicao(ctx) { salvar(ctx) },
  concluirTodas(ctx) {
    for (const t of ctx.shared.tarefas.itens) t.feita = true
  },
  async limpar(ctx) {
    if (await ctx.confirm('limpar-concluidas') !== 'ok') return
    ctx.shared.tarefas.limparConcluidas()
    ctx.log.info('tarefas concluídas removidas')
  },
}

function salvar(ctx) {
  const id = ctx.state.editando, titulo = ctx.state.rascunho.trim()
  if (!id) return
  ctx.state.editando = null
  if (!validar(ctx.shared.tarefas, titulo, id)) ctx.shared.tarefas.achar(id).titulo = titulo
}
