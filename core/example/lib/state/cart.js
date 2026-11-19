import { usd } from '../../utils/money.js'

let items = []
export const persist = ['items']

const count = () => items.reduce((sum, item) => sum + item.quantity, 0)
const total = () => usd(items.reduce((sum, item) => sum + item.quantity * item.price, 0))
const empty = () => items.length === 0
const lines = () => items.map(item => ({ ...item, subtotal: usd(item.quantity * item.price), full: item.quantity >= item.stock }))

function add({ id, name, price, stock }, quantity) {
  const item = items.find(entry => entry.id === id)
  if (item) item.quantity = Math.min(item.quantity + quantity, item.stock)
  else items.push({ id, name, price, stock, quantity: Math.min(quantity, stock) })
}

function change(id, delta) {
  const item = items.find(entry => entry.id === id)
  if (item.quantity + delta < 1) remove(id)
  else item.quantity = Math.min(item.quantity + delta, item.stock)
}

function remove(id) {
  items = items.filter(item => item.id !== id)
}

function clear() {
  items = []
}
