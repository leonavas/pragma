import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/logs', {
    'order-log': ['_components/order-log/order-log.js', '_components/order-log/order-log.html'],
    'auto-capture': ['_components/auto-capture/auto-capture.js', '_components/auto-capture/auto-capture.html'],
    refund: ['demo/refund.server.js', 'demo/refund.js', 'demo/refund.html'],
  })
  return { examples }
}
