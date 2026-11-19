import products from './_products.json' with { type: 'json' }
import { usd } from '../../utils/money.js'

export const CATEGORIES = { pantry: 'Pantry', cleaning: 'Cleaning', produce: 'Produce' }

const forDisplay = product => ({
  ...product,
  url: `/shop/${product.id}`,
  priceLabel: usd(product.price),
  categoryName: CATEGORIES[product.category],
  soldOut: product.stock === 0,
})

export const listProducts = category => products
  .filter(product => !CATEGORIES[category] || product.category === category)
  .map(forDisplay)

export function findProduct(id) {
  const product = products.find(entry => entry.id === id)
  return product ? forDisplay(product) : null
}
