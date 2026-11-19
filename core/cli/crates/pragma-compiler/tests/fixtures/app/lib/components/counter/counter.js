export const state = () => ({ n: 0 })
export const actions = {
  add(ctx) {
    ctx.$.n += 1
  },
}
