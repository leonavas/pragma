export const state = () => ({ deleted: false })

export const actions = {
  async deleteDraft(ctx) {
    if (await ctx.confirm('delete-draft') === 'ok') ctx.$.deleted = true
  },
}
