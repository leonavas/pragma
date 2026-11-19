import { find } from '../../utils/db.js'

export const ssr = true
export async function load({ params }) {
  return { post: await find(params.slug) }
}
