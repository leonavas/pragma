import { achar } from './_catalogo.server.js'

export async function load({ params }) {
  return { produto: achar(params.id) }
}

export const ssr = true
