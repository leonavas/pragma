import { listar } from './_catalogo.server.js'

export async function load({ query }) {
  return { produtos: listar(query.categoria) }
}
