const tickMs = 31

export const state = () => ({
  startedAt: 0,
  elapsedBefore: 0,
  now: 0,
  timer: 0,
  laps: [],
  lapCount: 0,
  get running() { return this.timer !== 0 },
  get elapsed() { return this.elapsedBefore + (this.running ? this.now - this.startedAt : 0) },
  get display() {
    const centis = Math.floor(this.elapsed / 10)
    const seconds = Math.floor(centis / 100)
    return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}.${String(centis % 100).padStart(2, '0')}`
  },
  get toggleText() { return this.running ? 'Stop' : 'Start' },
  get lapText() { return this.running || this.untouched ? 'Lap' : 'Reset' },
  get untouched() { return !this.running && this.elapsed === 0 },
})

export const actions = {
  toggle(ctx) {
    if (ctx.$.running) {
      clearInterval(ctx.$.timer)
      ctx.$.elapsedBefore = ctx.$.elapsed
      ctx.$.timer = 0
      return
    }
    ctx.$.startedAt = ctx.$.now = performance.now()
    ctx.$.timer = setInterval(() => ctx.update(current => { current.now = performance.now() }), tickMs)
  },
  lap(ctx) {
    if (ctx.$.running) {
      ctx.$.lapCount += 1
      ctx.$.laps = [{ id: ctx.$.lapCount, time: ctx.$.display }, ...ctx.$.laps].slice(0, 3)
      return
    }
    ctx.$.elapsedBefore = 0
    ctx.$.lapCount = 0
    ctx.$.laps = []
  },
}

export function mount(ctx) {
  return () => clearInterval(ctx.$.timer)
}
