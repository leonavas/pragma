export const state = ctx => ({ title: 'from box', n: Number(ctx.$.initial), max: ctx.$.max ?? '' })

export const actions = {
  inc(ctx) { ctx.$.n++ },
  box() {},
}
