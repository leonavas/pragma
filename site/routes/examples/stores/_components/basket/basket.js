export const actions = {
  remove(ctx) { ctx.shared.cart.remove(ctx.scope.line.id) },
  clear(ctx) { ctx.shared.cart.clear() },
}
