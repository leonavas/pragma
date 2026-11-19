export const state = () => ({
  open: false,
  get openText() { return this.open ? 'true' : 'false' },
  get buttonLabel() { return this.open ? 'Hide both' : 'Show both' },
})

export const actions = {
  toggle(ctx) { ctx.$.open = !ctx.$.open },
}
