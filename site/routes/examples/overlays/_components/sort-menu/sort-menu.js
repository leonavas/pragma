const files = [
  { name: 'router.js', size: 4120 },
  { name: 'overlay.js', size: 2860 },
  { name: 'state.js', size: 1310 },
  { name: 'boot.js', size: 690 },
]

export const state = () => ({
  key: 'name',
  descending: false,
  get menuLabel() { return 'Sort: ' + this.key + (this.descending ? ', desc' : ', asc') },
  get sorted() {
    const rows = files
      .map(file => ({ ...file, label: (file.size / 1000).toFixed(1) + ' kB' }))
      .sort((left, right) => this.key === 'size' ? left.size - right.size : left.name.localeCompare(right.name))
    return this.descending ? rows.reverse() : rows
  },
})

export const actions = {
  sortBy(ctx) { ctx.$.key = ctx.el.value },
  reverse(ctx) { ctx.$.descending = !ctx.$.descending },
}
