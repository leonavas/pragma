import { buscar } from '../../utils/db.js'

export async function load({ params }) {
  return { produto: await buscar(params.id) }
}
