import { carrinho } from '../lib/state/carrinho.js'
import { formatar } from '../utils/fmt.js'

export const state = () => ({ aberto: false, itens: [] })
export const actions = {
  alternar(ctx) {
    ctx.state.aberto = !ctx.state.aberto
    ctx.log.info(formatar(carrinho.total))
  },
}
