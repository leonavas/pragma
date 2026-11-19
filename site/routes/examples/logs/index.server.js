import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/logs', {
    orderLog: ['_components/order-log/order-log.html'],
    autoCapture: ['_components/auto-capture/auto-capture.html'],
    refund: ['demo/refund.html', 'demo/refund.server.js'],
  })
  return { examples }
}
