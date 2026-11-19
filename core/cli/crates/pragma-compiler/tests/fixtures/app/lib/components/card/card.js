export const actions = {
  buy(ctx) {
    ctx.shared.cart.total += 1
  },
}
