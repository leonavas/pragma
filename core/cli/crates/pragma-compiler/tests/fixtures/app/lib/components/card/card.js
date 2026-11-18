export const actions = {
  comprar(ctx) {
    ctx.shared.carrinho.total += 1
  },
}
