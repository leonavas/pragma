import { marcar } from '../../utils/texto.js'

const CATEGORIAS = ['todas', 'mercearia', 'limpeza', 'hortifruti']

export const state = ctx => ({
  get categoria() { return marcar(CATEGORIAS, CATEGORIAS.includes(ctx.query.categoria) ? ctx.query.categoria : 'todas') },
})
