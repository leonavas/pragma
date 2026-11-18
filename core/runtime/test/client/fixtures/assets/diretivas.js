export const state = () => ({
  nome: 'Ana',
  ativo: true,
  link: '/x',
  desabilitado: false,
  titulo: null,
  mostrar: true,
  pessoa: { nome: 'Bia' },
  itens: [
    { id: 1, nome: 'um', ok: true, tags: [{ t: 'a' }, { t: 'b' }] },
    { id: 2, nome: 'dois', ok: false, tags: [] },
  ],
  cores: ['azul', 'verde', 'roxo'],
  cor: 'verde',
  aceito: false,
  qtd: 2,
  tamanho: 'p',
  salvando: false,
  resposta: null,
  erro: null,
  tick: 0,
  enviado: false,
})

export const actions = {
  alternar(ctx) { ctx.state.mostrar = !ctx.state.mostrar },
  remover(ctx) { ctx.state.itens = ctx.state.itens.filter(i => i !== ctx.scope.item) },
  async salvar(ctx) {
    ctx.state.salvando = true
    ctx.state.resposta = await ctx.call('salvar', { nome: ctx.state.nome })
    ctx.state.salvando = false
  },
  async falhar(ctx) {
    try { await ctx.call('falhar') } catch (e) { ctx.state.erro = e.message }
  },
  comprar(ctx) { ctx.shared.carrinho.itens.push({ id: ctx.shared.carrinho.itens.length + 1 }) },
  enviar(ctx) { ctx.state.enviado = ctx.event.defaultPrevented },
}

export function mount(ctx) {
  self.__ctx = ctx
}
