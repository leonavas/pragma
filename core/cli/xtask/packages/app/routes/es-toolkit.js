import { groupBy } from 'es-toolkit'
import { debounce } from 'es-toolkit/function'

const products = [
  { name: 'Coffee', category: 'drinks' },
  { name: 'Tea', category: 'drinks' },
  { name: 'Bread', category: 'bakery' },
]

const saveLater = debounce(ctx => ctx.update(counters => { counters.saves += 1 }), 150)

export const state = () => ({
  clicks: 0,
  saves: 0,
  get productsByCategory() {
    return Object.entries(groupBy(products, product => product.category))
      .map(([category, inCategory]) => category + ': ' + inCategory.length)
      .join(', ')
  },
})

export const actions = {
  registerClick(ctx) {
    ctx.$.clicks += 1
    saveLater(ctx)
  },
}
