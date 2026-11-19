import { carrinho } from '/assets/lib/state/carrinho.js'
import { formatar } from '/assets/utils/fmt.js'

export const state = () => ({ aberto: false, itens: [] })
export const actions = {
  alternar(ctx) {
    ctx.$.aberto = !ctx.$.aberto
    ctx.log.info(formatar(carrinho.total))
  },
}
