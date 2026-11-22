import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/logs', {
    orderButton: [{ file: '_components/order-button/order-button.html', as: 'routes/index.html' }],
    saveButton: [{ file: '_components/save-button/save-button.html', as: 'routes/index.html' }],
    refund: ['demo/refund.html', 'demo/refund.server.js'],
  })
  return { examples }
}
