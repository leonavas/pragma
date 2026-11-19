import { listProducts } from './_catalog.server.js'

export async function load({ query, log }) {
  if (query.fail) throw new Error('Inventory service timed out after 700 ms')
  const products = await listProducts(query.category)
  log.info('catalog loaded', { 'product.total': products.length })
  return { products, loadedAt: new Date().toISOString().slice(11, 23) }
}
