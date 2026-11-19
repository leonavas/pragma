import { cart } from '../lib/state/cart.js'
import { format } from '../utils/fmt.js'

export const state = () => ({ open: false, items: [] })
export const actions = {
  toggle(ctx) {
    ctx.$.open = !ctx.$.open
    ctx.log.info(format(cart.total))
  },
}
