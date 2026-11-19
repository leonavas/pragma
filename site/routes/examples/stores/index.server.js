import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/stores', {
    cart: [
      '../../../lib/state/cart.js',
      '_snippets/cart/usage.html',
      '_components/shelf/shelf.html',
      '_components/shelf/shelf.js',
      '_components/basket/basket.html',
      '_components/basket/basket.js',
    ],
    shipping: ['_components/shipping/shipping.js', '_components/shipping/shipping.html'],
    scratchpad: [
      '../../../lib/state/scratchpad.js',
      '_components/draft-pad/draft-pad.html',
      '_components/draft-pad/draft-pad.js',
      'demo/scratchpad.html',
    ],
  })
  return { examples }
}
