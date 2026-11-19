export const state = ctx => ({
  count: Number(ctx.$.start || 0),
  get increment() { return Number(ctx.$.step || 1) },
  get atMinimum() { return this.count - this.increment < 0 },
  get atStart() { return this.count === Number(ctx.$.start || 0) },
})

export const actions = {
  decrease(ctx) { ctx.$.count -= ctx.$.increment },
  increase(ctx) { ctx.$.count += ctx.$.increment },
  reset(ctx) { ctx.$.count = Number(ctx.$.start || 0) },
}
