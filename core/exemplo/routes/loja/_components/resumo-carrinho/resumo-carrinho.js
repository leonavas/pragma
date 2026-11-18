export const actions = {
  menos(ctx) { ctx.shared.carrinho.mudar(ctx.scope.i.id, -1) },
  mais(ctx) { ctx.shared.carrinho.mudar(ctx.scope.i.id, 1) },
  remover(ctx) { ctx.shared.carrinho.remover(ctx.scope.i.id) },
  async esvaziar(ctx) {
    if (await ctx.confirm('esvaziar-carrinho') === 'ok') ctx.shared.carrinho.esvaziar()
  },
}
