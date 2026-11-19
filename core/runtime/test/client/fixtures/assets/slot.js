export const state = ctx => ({
  title: 'from page',
  field: '',
  visible: true,
  clicks: 0,
  picked: '',
  marked: false,
  editing: false,
  items: [{ id: 1, name: 'one', max: 3 }, { id: 2, name: 'two', max: 7 }],
  get double() { return ctx.shared.account.total * 2 },
})

export const actions = {
  page(ctx) { ctx.$.clicks++; ctx.shared.account.n++ },
  pick(ctx) { ctx.$.picked = ctx.scope.item.name },
}

export function mount(ctx) {
  self.__slot = ctx
}
