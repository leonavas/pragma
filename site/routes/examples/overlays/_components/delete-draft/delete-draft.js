export const state = () => ({
  deleted: false,
  answer: '',
  get answerText() { return this.answer ? `ctx.confirm() resolved '${this.answer}'` : 'ctx.confirm() not called yet' },
})

export const actions = {
  async remove(ctx) {
    ctx.$.answer = await ctx.confirm('delete-draft')
    if (ctx.$.answer === 'ok') ctx.$.deleted = true
  },
  restore(ctx) {
    ctx.$.deleted = false
    ctx.$.answer = ''
  },
}
