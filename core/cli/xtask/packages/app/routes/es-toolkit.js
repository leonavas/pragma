import { groupBy } from 'es-toolkit'
import { debounce } from 'es-toolkit/function'

const products = [
  { name: 'Coffee', category: 'drinks' },
  { name: 'Tea', category: 'drinks' },
  { name: 'Bread', category: 'bakery' },
]

const saveLater = debounce(ctx => ctx.update(current => {
  current.saves += 1
  current.savedDraft = current.draft
}), 200)

export const state = () => ({
  draft: '',
  saves: 0,
  savedDraft: '',
  get productsByCategory() {
    return Object.entries(groupBy(products, product => product.category))
      .map(([category, inCategory]) => category + ': ' + inCategory.length)
      .join(', ')
  },
})

export const actions = {
  scheduleSave(ctx) {
    saveLater(ctx)
  },
}
