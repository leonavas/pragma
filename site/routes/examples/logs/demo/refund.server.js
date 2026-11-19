const refundStatus = new Map([['R-1042', 'pending']])

export async function load({ log }) {
  log.info('refund queue loaded', { 'refund.count': refundStatus.size })
  return { refund: { id: 'R-1042', customer: 'Order A-1042', amount: '€38.00', status: refundStatus.get('R-1042') } }
}

export const actions = {
  async approve({ input, log }) {
    refundStatus.set(input.id, 'approved')
    log.info('refund approved', { 'refund.id': input.id })
    return { status: 'approved' }
  },
  async escalate({ input, log }) {
    log.warn('refund escalation requested', { 'refund.id': input.id })
    throw new Error('escalation service unavailable')
  },
  async reopen({ input }) {
    refundStatus.set(input.id, 'pending')
    return { status: 'pending' }
  },
}
