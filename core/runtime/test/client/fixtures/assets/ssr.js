export const state = () => ({ n: 0, open: false })
export const actions = {
  inc(ctx) { ctx.$.n++ },
}
