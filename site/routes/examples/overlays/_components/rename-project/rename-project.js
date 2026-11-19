export const state = () => ({
  name: 'minimart',
  draft: '',
  get invalid() { return !this.draft.trim() },
})

export const actions = {
  edit(ctx) {
    ctx.$.draft = ctx.$.name
    ctx.open('rename-modal')
  },
  save(ctx) {
    ctx.$.name = ctx.$.draft.trim()
    ctx.close('rename-modal')
  },
  async discard(ctx) {
    if (ctx.$.draft !== ctx.$.name && await ctx.confirm('discard-alert') !== 'ok') return
    ctx.close('rename-modal')
  },
}
