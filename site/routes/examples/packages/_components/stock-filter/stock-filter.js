import { groupBy } from 'es-toolkit'
import { debounce } from 'es-toolkit/function'

const products = [
  { name: 'Roasted coffee', category: 'drinks' },
  { name: 'Green tea', category: 'drinks' },
  { name: 'Oat milk', category: 'drinks' },
  { name: 'Sourdough bread', category: 'bakery' },
  { name: 'Butter croissant', category: 'bakery' },
  { name: 'Brown sugar', category: 'pantry' },
  { name: 'Black tea leaves', category: 'pantry' },
]

const applyLater = debounce(ctx => ctx.update(current => {
  current.applied = current.term.trim().toLowerCase()
  current.runs += 1
}), 300)

export const state = () => ({
  term: '',
  applied: '',
  keystrokes: 0,
  runs: 0,
  get keystrokesText() { return this.keystrokes + ' keystrokes' },
  get runsText() { return this.runs + ' filter runs' },
  get groups() {
    const matching = products.filter(product => product.name.toLowerCase().includes(this.applied))
    return Object.entries(groupBy(matching, product => product.category))
      .map(([category, inCategory]) => ({ category, names: inCategory.map(product => product.name).join(', ') }))
  },
})

export const actions = {
  scheduleFilter(ctx) {
    ctx.$.keystrokes += 1
    applyLater(ctx)
  },
}

export function mount() {
  return () => applyLater.cancel()
}
