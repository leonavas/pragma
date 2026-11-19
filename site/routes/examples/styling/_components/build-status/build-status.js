const buildMs = 900

export const state = () => ({
  status: 'idle',
  get running() { return this.status === 'running' },
  get passed() { return this.status === 'passed' },
  get failed() { return this.status === 'failed' },
})

function finish(ctx, outcome) {
  ctx.$.status = 'running'
  setTimeout(() => ctx.update(current => { current.status = outcome }), buildMs)
}

export const actions = {
  run(ctx) { finish(ctx, 'passed') },
  breakIt(ctx) { finish(ctx, 'failed') },
}
