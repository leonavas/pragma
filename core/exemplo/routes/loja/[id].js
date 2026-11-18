export const state = () => ({ enviando: false, erro: '' })

export const actions = {
  async curtir(ctx) {
    ctx.state.enviando = true
    ctx.state.erro = ''
    try {
      const r = await ctx.call('curtir')
      ctx.data.produto.curtidas = r.curtidas
    } catch (e) {
      ctx.log.error('falha ao curtir produto', { 'produto.id': ctx.params.id, 'exception.message': e.message })
      ctx.state.erro = 'Não foi possível registrar agora.'
    } finally {
      ctx.state.enviando = false
    }
  },
}
