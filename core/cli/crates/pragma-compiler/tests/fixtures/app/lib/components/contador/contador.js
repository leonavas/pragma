export const state = () => ({ n: 0 })
export const actions = {
  somar(ctx) {
    ctx.state.n += 1
  },
}
