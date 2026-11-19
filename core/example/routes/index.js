export const state = ({ shared }) => ({
  get taskTone() { return shared.tasks.pending ? 'neutral' : 'warning' },
  get cartTone() { return shared.cart.empty ? 'warning' : 'ok' },
})
