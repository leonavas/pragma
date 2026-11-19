import { contains, markCurrent } from '../../utils/text.js'

const CATEGORIES = ['all', 'pantry', 'cleaning', 'produce']

export const state = ctx => ({
  search: '',
  get category() { return markCurrent(CATEGORIES, CATEGORIES.includes(ctx.query.category) ? ctx.query.category : 'all') },
  get products() { return ctx.data.products.filter(product => contains(product.name, this.search)) },
})
