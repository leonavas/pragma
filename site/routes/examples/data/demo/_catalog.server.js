import products from './_products.json' with { type: 'json' }

const latencyMs = 700

export const categories = ['hardware', 'books']

export async function listProducts(category) {
  await new Promise(resolve => setTimeout(resolve, latencyMs))
  const visible = categories.includes(category) ? products.filter(product => product.category === category) : products
  return visible.map(product => ({ ...product, priceLabel: product.price ? `$${product.price}` : 'free' }))
}
