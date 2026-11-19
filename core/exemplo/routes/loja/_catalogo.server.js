import produtos from './_produtos.json' with { type: 'json' }
import { brl } from '../../utils/dinheiro.js'

export const CATEGORIAS = { mercearia: 'Mercearia', limpeza: 'Limpeza', hortifruti: 'Hortifruti' }

const exibir = p => ({
  ...p,
  url: `/loja/${p.id}`,
  precoBRL: brl(p.preco),
  categoriaNome: CATEGORIAS[p.categoria],
  esgotado: p.estoque === 0,
})

export const listar = categoria => produtos.filter(p => !CATEGORIAS[categoria] || p.categoria === categoria).map(exibir)

export function achar(id) {
  const produto = produtos.find(p => p.id === id)
  return produto ? exibir(produto) : null
}
