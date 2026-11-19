export const state = () => ({
  restarting: false,
})

export const actions = {
  replay(ctx) {
    ctx.$.restarting = true
    requestAnimationFrame(() => requestAnimationFrame(() => ctx.update(current => { current.restarting = false })))
  },
}
