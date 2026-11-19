export const state = () => ({ apagado: false })

export const actions = {
  async apagar(ctx) {
    if (await ctx.confirm('apagar') == 'ok') ctx.$.apagado = true
  },
}
