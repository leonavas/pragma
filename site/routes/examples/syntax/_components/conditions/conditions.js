const sampleOrders = [
  { id: '#1042', total: '$18.00' },
  { id: '#1043', total: '$64.50' },
]

export const state = () => ({
  mode: 'ready',
  get modes() {
    return ['loading', 'error', 'empty', 'ready'].map(id => ({ id, pressed: id === this.mode ? 'true' : 'false' }))
  },
  get loading() { return this.mode === 'loading' },
  get failed() { return this.mode === 'error' },
  get orders() { return this.mode === 'ready' ? sampleOrders : [] },
})

export const actions = {
  pick(ctx) { ctx.$.mode = ctx.scope.mode.id },
}
