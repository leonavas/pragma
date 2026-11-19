export const state = () => ({ n: 0 })
export const actions = {
  somar(ctx) {
    ctx.$.n += 1
  },
}
