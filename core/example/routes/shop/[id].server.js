import { findProduct } from './_catalog.server.js'

const likes = new Map()

export async function load({ params }) {
  const product = findProduct(params.id)
  return { product: product && { ...product, likes: likes.get(product.id) ?? 0 } }
}

export const actions = {
  async like({ params, log }) {
    if (!findProduct(params.id)) throw new Error('Product not found.')
    const total = (likes.get(params.id) ?? 0) + 1
    likes.set(params.id, total)
    log.info('product liked', { 'product.id': params.id, 'product.likes': total })
    return { likes: total }
  },
}

export const ssr = true
