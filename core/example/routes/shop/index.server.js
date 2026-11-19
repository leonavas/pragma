import { listProducts } from './_catalog.server.js'

const DELAY_MS = 400

export async function load({ query, log }) {
  await new Promise(resolve => setTimeout(resolve, DELAY_MS))
  if (query.fail === '1') throw new Error('Could not load the products.')
  const products = listProducts(query.category)
  log.debug('catalog loaded', { 'product.total': products.length })
  return { products }
}
