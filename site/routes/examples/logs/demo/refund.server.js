export async function load({ log }) {
  log.info('refund loaded', { 'refund.id': 'R-1042' })
  return { refund: { id: 'R-1042' } }
}

export const actions = {
  async approve({ input, log }) {
    log.info('refund approved', {
      'refund.id': input.id,
    })
  },
  async escalate({ input, log }) {
    log.warn('refund escalated', {
      'refund.id': input.id,
    })
    throw new Error('escalation service is down')
  },
}
