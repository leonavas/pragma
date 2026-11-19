export const state = () => ({ sending: false, error: '' })

async function send(ctx, actionName) {
  ctx.$.sending = true
  ctx.$.error = ''
  try {
    ctx.data.refund.status = (await ctx.call(actionName, { id: ctx.data.refund.id })).status
  } catch (failure) {
    ctx.log.error('refund action failed', { 'refund.action': actionName, 'exception.message': failure.message })
    ctx.$.error = failure.message
  } finally {
    ctx.$.sending = false
  }
}

export const actions = {
  approve: ctx => send(ctx, 'approve'),
  escalate: ctx => send(ctx, 'escalate'),
  reopen: ctx => send(ctx, 'reopen'),
}
