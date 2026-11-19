export const state = () => ({
  collapsed: false,
  get expanded() { return String(!this.collapsed) },
  get toggleText() { return this.collapsed ? 'Show' : 'Hide' },
})

export const actions = {
  toggle(ctx) { ctx.$.collapsed = !ctx.$.collapsed },
}
