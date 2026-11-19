export const state = ({ $ }) => ({
  quantity: 1,
  get soldOut() { return Number($.stock) === 0 },
  get atMin() { return this.quantity <= 1 },
  get atMax() { return this.quantity >= Number($.stock) },
})

export const actions = {
  decrement(ctx) { ctx.$.quantity-- },
  increment(ctx) { ctx.$.quantity++ },
  add(ctx) {
    const { id, name, price, stock } = ctx.$
    ctx.shared.cart.add({ id, name, price: Number(price), stock: Number(stock) }, ctx.$.quantity)
    ctx.log.info('product added to cart', { 'product.id': id, 'cart.item.quantity': ctx.$.quantity })
    ctx.$.quantity = 1
  },
}
