import { readExamples } from '../_source.server.js'

export const ssr = true

const asPage = name => ({ file: `_components/${name}/${name}.html`, as: 'routes/index.html' })

export async function load() {
  const examples = await readExamples('routes/examples/stores', {
    cart: [
      '../../../lib/state/cart.js',
      '_components/add-button/add-button.html',
      '_components/cart-badge/cart-badge.html',
      '_snippets/cart/usage.html',
    ],
    shipping: [asPage('shipping')],
    notes: ['../../../lib/state/notes.js', asPage('note-pad'), 'demo/notes.html'],
  })
  return { examples }
}
