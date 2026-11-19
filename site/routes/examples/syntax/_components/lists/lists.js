const startingGroups = () => [
  { id: 'today', name: 'Today', items: [{ id: 'a', label: 'Review the PR' }, { id: 'b', label: 'Write the changelog' }] },
  { id: 'later', name: 'Later', items: [{ id: 'c', label: 'Upgrade the runtime' }] },
]

export const state = () => ({
  raw: startingGroups(),
  done: {},
  get groups() {
    return this.raw.map(group => ({
      ...group,
      cleared: group.items.length === 0,
      items: group.items.map(item => ({ ...item, done: Boolean(this.done[item.id]), pressed: this.done[item.id] ? 'true' : 'false' })),
    }))
  },
})

export const actions = {
  pick(ctx) {
    const id = ctx.scope.item.id
    ctx.$.done = { ...ctx.$.done, [id]: !ctx.$.done[id] }
  },
  clear(ctx) {
    const groupId = ctx.scope.group.id
    ctx.$.raw = ctx.$.raw.map(group => (group.id === groupId ? { ...group, items: [] } : group))
  },
  reset(ctx) {
    ctx.$.raw = startingGroups()
    ctx.$.done = {}
  },
}
