import { listar } from './_catalogo.server.js'

const ATRASO_MS = 400

export async function load({ query, log }) {
  await new Promise(r => setTimeout(r, ATRASO_MS))
  if (query.falhar === '1') throw new Error('Não foi possível carregar os produtos.')
  const produtos = listar(query.categoria)
  log.debug('catálogo carregado', { 'produto.total': produtos.length })
  return { produtos }
}
