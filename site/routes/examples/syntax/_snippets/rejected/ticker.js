export const state = ctx => ({
  count: Number(ctx.$.start),
  get label() { return this.count + ' ticks' },
})

export const actions = {
  reset(ctx) { ctx.$.count = 0 },
}

export function mount(ctx) {
  const timer = setInterval(() => ctx.update(current => { current.count += 1 }), 1000)
  return () => clearInterval(timer)
}
