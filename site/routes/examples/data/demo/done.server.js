import { redirect } from 'pragma/server'

export const ssr = true

export async function load({ query }) {
  if (!query.id) redirect(303, '/examples/data/demo/order')
  return { id: query.id }
}
