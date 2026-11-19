const TABS = ['a', 'b', 'c']
const ARROW_STEPS = { ArrowRight: 1, ArrowLeft: -1 }
const perTab = compute => Object.fromEntries(TABS.map(tab => [tab, compute(tab)]))

export const state = () => ({
  current: 'a',
  get visible() { return perTab(tab => tab === this.current) },
  get selected() { return perTab(tab => String(tab === this.current)) },
  get tabindex() { return perTab(tab => (tab === this.current ? 0 : -1)) },
})

export const actions = {
  select(ctx) { ctx.$.current = ctx.el.value },
  moveWithArrows(ctx) {
    const step = ARROW_STEPS[ctx.event.key]
    if (!step) return
    const index = (TABS.indexOf(ctx.$.current) + step + TABS.length) % TABS.length
    ctx.$.current = TABS[index]
    ctx.el.children[index].focus()
  },
}
