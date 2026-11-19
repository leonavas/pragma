import { flush } from '../../../../../core/log.js'

const severityNumbers = { debug: 5, info: 9, warn: 13, error: 17 }

let nextRecordId = 1

function record(ctx, level, body, attributes) {
  ctx.log[level](body, attributes)
  ctx.$.records = [{ id: nextRecordId++, level, body, attributes }, ...ctx.$.records].slice(0, 4)
}

export const state = () => ({
  records: [],
  orderNumber: 1041,
  get orderId() { return 'A-' + this.orderNumber },
  get recordRows() {
    return this.records.map(entry => ({
      ...entry,
      levelText: entry.level.toUpperCase() + ' ' + severityNumbers[entry.level],
      isWarn: entry.level === 'warn',
      isError: entry.level === 'error',
      attributesText: JSON.stringify(entry.attributes),
    }))
  },
})

export const actions = {
  placeOrder(ctx) {
    ctx.$.orderNumber += 1
    record(ctx, 'info', 'order placed', { 'order.id': ctx.$.orderId, 'order.items': 3 })
  },
  retryPayment(ctx) {
    record(ctx, 'warn', 'payment retried', { 'order.id': ctx.$.orderId, 'payment.attempt': 2 })
  },
  failPayment(ctx) {
    try {
      throw new Error('card declined')
    } catch (failure) {
      record(ctx, 'error', 'payment failed', { 'order.id': ctx.$.orderId, 'exception.message': failure.message })
    }
  },
  sendNow() {
    flush()
  },
}

export function mount(ctx) {
  ctx.log.debug('order panel mounted')
}
