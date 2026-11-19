import Fuse from 'fuse.js'

const products = ['Roasted coffee', 'Green tea', 'Brown sugar', 'Black tea', 'Oat milk'].map((name, index) => ({ id: index + 1, name }))
const productSearch = new Fuse(products, { keys: ['name'], threshold: 0.3 })

export const state = () => ({
  term: '',
  get results() {
    return this.term ? productSearch.search(this.term).map(result => result.item) : products
  },
})
