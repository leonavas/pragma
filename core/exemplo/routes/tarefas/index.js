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
    const titulo = ctx.$.nova.trim()
    ctx.$.erro = validar(ctx.shared.tarefas, titulo)
    if (ctx.$.erro) return
    ctx.shared.tarefas.adicionar(titulo)
    ctx.$.nova = ''
    ctx.el.elements.titulo.focus()
  },
  alternar(ctx) {
    const tarefa = ctx.shared.tarefas.achar(ctx.scope.t.id)
    tarefa.feita = !tarefa.feita
  },
  remover(ctx) { ctx.shared.tarefas.remover(ctx.scope.t.id) },
  editar(ctx) {
    ctx.$.editando = ctx.scope.t.id
    ctx.$.rascunho = ctx.scope.t.titulo
  },
  teclaEdicao(ctx) {
    if (ctx.event.key === 'Escape') ctx.$.editando = null
    if (ctx.event.key === 'Enter') salvar(ctx)
  },
  salvarEdicao(ctx) { salvar(ctx) },
  concluirTodas(ctx) {
    for (const tarefa of ctx.shared.tarefas.itens) tarefa.feita = true
  },
  async limpar(ctx) {
    if (await ctx.confirm('limpar-concluidas') !== 'ok') return
    ctx.shared.tarefas.limparConcluidas()
    ctx.log.info('tarefas concluídas removidas')
  },
}

function salvar(ctx) {
  const id = ctx.$.editando, titulo = ctx.$.rascunho.trim()
  if (!id) return
  ctx.$.editando = null
  if (!validar(ctx.shared.tarefas, titulo, id)) ctx.shared.tarefas.achar(id).titulo = titulo
}
