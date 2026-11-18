export const state = () => ({ n: 0, aberto: false })
export const actions = {
  mais(ctx) { ctx.state.n++ },
}
