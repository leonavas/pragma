import { store } from '../../core/state.js'

const money = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' })

export default store({
  lines: [],
  get count() { return this.lines.reduce((sum, line) => sum + line.quantity, 0) },
  get total() { return this.lines.reduce((sum, line) => sum + line.quantity * line.price, 0) },
  get totalLabel() { return money.format(this.total) },
  get empty() { return this.lines.length === 0 },
  add(product) {
    const line = this.lines.find(entry => entry.id === product.id)
    if (line) line.quantity += 1
    else this.lines.push({ id: product.id, name: product.name, price: product.price, quantity: 1, removeLabel: `Remove ${product.name}` })
  },
  remove(id) { this.lines = this.lines.filter(line => line.id !== id) },
  clear() { this.lines = [] },
})
