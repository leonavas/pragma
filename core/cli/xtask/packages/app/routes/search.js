import Fuse from 'fuse.js'

const productSearch = new Fuse(['Roasted coffee', 'Green tea', 'Brown sugar'])

export const state = () => ({
  term: 'rosted',
  get foundProduct() {
    return productSearch.search(this.term)[0]?.item ?? 'Nothing found'
  },
})
