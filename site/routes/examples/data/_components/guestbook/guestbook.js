export const state = () => ({ name: '', message: '', sending: false, error: '' })

export const actions = {
  async sign(ctx) {
    ctx.$.sending = true
    ctx.$.error = ''
    try {
      const result = await ctx.call('sign', { name: ctx.$.name, message: ctx.$.message })
      ctx.data.guestbook = result.guestbook
      ctx.$.message = ''
    } catch (failure) {
      ctx.$.error = failure.message
    } finally {
      ctx.$.sending = false
    }
  },
}
