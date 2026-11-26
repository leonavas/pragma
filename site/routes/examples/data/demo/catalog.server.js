import { error } from 'pragma/server'

const potions = [
  { id: 1, name: 'Elixir of Focus' },
  { id: 2, name: 'Cold Brew of Haste' },
  { id: 3, name: 'Donut of Morale' },
]

export async function load({ query }) {
  await new Promise(done => setTimeout(done, 800))
  if (query.fail) error(503, 'The cauldron tipped over.')
  return { potions }
}
