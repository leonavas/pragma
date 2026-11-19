const catalog = [
  { id: 'mug', name: 'Mug', price: 14 },
  { id: 'tee', name: 'T-shirt', price: 28 },
  { id: 'cap', name: 'Cap', price: 22 },
]

export const state = () => ({
  products: catalog.map(product => ({ ...product, priceLabel: `$${product.price}`, addLabel: `Add ${product.name}` })),
})

export const actions = {
  add(ctx) { ctx.shared.cart.add(ctx.scope.product) },
}
