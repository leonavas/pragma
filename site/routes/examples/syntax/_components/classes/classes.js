export const state = () => ({
  done: false,
  get doneText() { return this.done ? 'true' : 'false' },
  get buttonLabel() { return this.done ? 'Reopen' : 'Mark as done' },
})

export const actions = {
  toggle(ctx) { ctx.$.done = !ctx.$.done },
}
