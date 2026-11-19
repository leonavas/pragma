export const state = ctx => ({
  likes: 0,
  query: '',
  get likesText() { return this.likes === 1 ? '1 like' : this.likes + ' likes' },
  get visiblePlugins() {
    const needle = this.query.trim().toLowerCase()
    return (ctx.data?.plugins ?? []).filter(plugin => plugin.name.includes(needle) || plugin.kind.includes(needle))
  },
})

export const actions = {
  like(ctx) { ctx.$.likes += 1 },
}
