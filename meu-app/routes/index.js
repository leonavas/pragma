export const state = () => ({
  apagado: false,
  contador: 0,
  get rotuloContador() { return `Cliques: ${this.contador}` },
})

export const actions = {
  incrementar(ctx) {
    ctx.log.info('aaaa');
    ctx.$.contador++
    ctx.log.info('contador incrementado', { 'contador.valor': ctx.$.contador })
  },
  async apagar(ctx) {
    ctx.log('aaaa');
    if (await ctx.confirm('apagar') == 'ok') ctx.$.apagado = true
  },
}
