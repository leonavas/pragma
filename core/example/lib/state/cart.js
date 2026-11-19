import { store } from '../../core/state.js'
import { usd } from '../../utils/money.js'

function setQuantity(item, quantity) {
  item.quantity = Math.min(quantity, item.stock)
  item.full = item.quantity >= item.stock
  item.subtotal = usd(item.quantity * item.price)
}

export default store({
  items: [],
  get count() { return this.items.reduce((sum, item) => sum + item.quantity, 0) },
  get total() { return usd(this.items.reduce((sum, item) => sum + item.quantity * item.price, 0)) },
  get empty() { return this.items.length === 0 },
  add({ id, name, price, stock }, quantity) {
    let item = this.items.find(entry => entry.id === id)
    if (!item) this.items.push((item = { id, name, price, stock, quantity: 0 }))
    setQuantity(item, item.quantity + quantity)
  },
  change(id, delta) {
    const item = this.items.find(entry => entry.id === id)
    if (item.quantity + delta < 1) return this.remove(id)
    setQuantity(item, item.quantity + delta)
  },
  remove(id) { this.items = this.items.filter(item => item.id !== id) },
  clear() { this.items = [] },
}, { persist: 'cart' })
