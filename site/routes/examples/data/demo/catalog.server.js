import { error } from 'pragma/server'

const products = [
  { id: 1, name: 'Keyboard' },
  { id: 2, name: 'USB-C hub' },
]

export async function load({ query }) {
  await new Promise(done => setTimeout(done, 800))
  if (query.fail) error(503, 'The inventory is down.')
  return { products }
}
