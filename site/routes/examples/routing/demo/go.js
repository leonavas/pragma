export const state = () => ({
  slug: 'closed-syntax',
})

export const actions = {
  open(ctx) {
    ctx.navigate('/examples/routing/demo/' + encodeURIComponent(ctx.$.slug.trim()))
  },
}
