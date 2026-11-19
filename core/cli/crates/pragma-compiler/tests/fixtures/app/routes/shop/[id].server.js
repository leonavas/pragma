import { find } from '../../utils/db.js'

export async function load({ params }) {
  return { product: await find(params.id) }
}
