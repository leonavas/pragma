export const actions = {
  decrement(ctx) { ctx.shared.cart.change(ctx.scope.item.id, -1) },
  increment(ctx) { ctx.shared.cart.change(ctx.scope.item.id, 1) },
  remove(ctx) { ctx.shared.cart.remove(ctx.scope.item.id) },
  async clear(ctx) {
    if (await ctx.confirm('empty-cart') === 'ok') ctx.shared.cart.clear()
  },
}
