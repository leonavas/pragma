import { achar } from './_catalogo.server.js'

const curtidas = new Map()

export async function load({ params }) {
  const produto = achar(params.id)
  return { produto: produto && { ...produto, curtidas: curtidas.get(produto.id) ?? 0 } }
}

export const actions = {
  async curtir({ params, log }) {
    if (!achar(params.id)) throw new Error('Produto não encontrado.')
    const total = (curtidas.get(params.id) ?? 0) + 1
    curtidas.set(params.id, total)
    log.info('produto curtido', { 'produto.id': params.id, 'produto.curtidas': total })
    return { curtidas: total }
  },
}

export const ssr = true
