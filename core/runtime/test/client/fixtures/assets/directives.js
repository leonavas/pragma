export const state = () => ({
  name: 'Ana',
  active: true,
  link: '/x',
  disabled: false,
  title: null,
  visible: true,
  person: { name: 'Bia' },
  items: [
    { id: 1, name: 'one', ok: true, tags: [{ t: 'a' }, { t: 'b' }] },
    { id: 2, name: 'two', ok: false, tags: [] },
  ],
  colors: ['blue', 'green', 'purple'],
  color: 'green',
  accepted: false,
  qty: 2,
  size: 'p',
  saving: false,
  response: null,
  error: null,
  tick: 0,
  sent: false,
})

export const actions = {
  toggle(ctx) { ctx.$.visible = !ctx.$.visible },
  remove(ctx) { ctx.$.items = ctx.$.items.filter(i => i !== ctx.scope.item) },
  async save(ctx) {
    ctx.$.saving = true
    ctx.$.response = await ctx.call('save', { name: ctx.$.name })
    ctx.$.saving = false
  },
  async fail(ctx) {
    try { await ctx.call('fail') } catch (e) { ctx.$.error = e.message }
  },
  buy(ctx) { ctx.shared.cart.items.push({ id: ctx.shared.cart.items.length + 1 }) },
  submit(ctx) { ctx.$.sent = ctx.event.defaultPrevented },
}

export function mount(ctx) {
  self.__ctx = ctx
}
