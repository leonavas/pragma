import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/stores', {
    cart: [
      '../../../lib/state/cart.js',
      '_snippets/cart/usage.html',
      '_components/shelf/shelf.html',
      '_components/basket/basket.html',
    ],
    scratchpad: [
      '../../../lib/state/scratchpad.js',
      '_components/draft-pad/draft-pad.html',
      'demo/scratchpad.html',
    ],
    shipping: ['_components/shipping/shipping.html'],
  })
  return { examples }
}
