export const state = () => ({ enviando: false, erro: '' })

export const actions = {
  async curtir(ctx) {
    ctx.$.enviando = true
    ctx.$.erro = ''
    try {
      const resposta = await ctx.call('curtir')
      ctx.data.produto.curtidas = resposta.curtidas
    } catch (erro) {
      ctx.log.error('falha ao curtir produto', { 'produto.id': ctx.params.id, 'exception.message': erro.message })
      ctx.$.erro = 'Não foi possível registrar agora.'
    } finally {
      ctx.$.enviando = false
    }
  },
}
