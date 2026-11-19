export const state = () => ({
  online: false,
  get offline() { return !this.online },
  get pressedText() { return this.online ? 'true' : 'false' },
  get tone() { return this.online ? 'ok' : 'down' },
  get heading() { return this.online ? 'Operational' : 'Down' },
  get link() { return this.online && '#bindings' },
  get linkTitle() { return this.online && 'Jump to this example' },
  get linkLabel() { return this.online ? 'Open status page' : 'No status page while down' },
})

export const actions = {
  toggle(ctx) { ctx.$.online = !ctx.$.online },
}
