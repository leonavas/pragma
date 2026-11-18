import { contem, marcar } from '../../utils/texto.js'

const CATEGORIAS = ['todas', 'mercearia', 'limpeza', 'hortifruti']

export const state = ctx => ({
  busca: '',
  get categoria() { return marcar(CATEGORIAS, CATEGORIAS.includes(ctx.query.categoria) ? ctx.query.categoria : 'todas') },
  get produtos() { return ctx.data.produtos.filter(p => contem(p.nome, this.busca)) },
})
