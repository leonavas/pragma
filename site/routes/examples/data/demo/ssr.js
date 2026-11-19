export const state = () => ({ hydratedAt: '', error: '' })

export const actions = {
  async vote(ctx) {
    ctx.$.error = ''
    try {
      const result = await ctx.call('vote', { id: ctx.scope.option.id })
      ctx.data.options = result.options
      ctx.data.title = result.title
    } catch (failure) {
      ctx.log.error('ssr demo vote failed', { 'exception.message': failure.message })
      ctx.$.error = failure.message
    }
  },
}

export function mount(ctx) {
  ctx.update(current => { current.hydratedAt = new Date().toISOString().slice(11, 23) })
}
