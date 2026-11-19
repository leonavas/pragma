const money = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' })

let lines = []

const count = () => lines.reduce((sum, line) => sum + line.quantity, 0)
const total = () => lines.reduce((sum, line) => sum + line.quantity * line.price, 0)
const totalLabel = () => money.format(total())
const empty = () => lines.length === 0

function add(product) {
  const line = lines.find(entry => entry.id === product.id)
  if (line) line.quantity += 1
  else lines.push({ id: product.id, name: product.name, price: product.price, quantity: 1 })
}

function remove(line) {
  lines = lines.filter(entry => entry.id !== line.id)
}

function clear() {
  lines = []
}
