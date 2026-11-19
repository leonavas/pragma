export const state = () => ({ sending: false, error: '' })

export const actions = {
  async like(ctx) {
    ctx.$.sending = true
    ctx.$.error = ''
    try {
      const response = await ctx.call('like')
      ctx.data.product.likes = response.likes
    } catch (failure) {
      ctx.log.error('failed to like product', { 'product.id': ctx.params.id, 'exception.message': failure.message })
      ctx.$.error = 'Could not save your like right now.'
    } finally {
      ctx.$.sending = false
    }
  },
}
