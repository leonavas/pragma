import { cart } from '/assets/lib/state/cart.js'
import { format } from '/assets/utils/fmt.js'

export const state = () => ({ open: false, items: [] })
export const actions = {
  toggle(ctx) {
    ctx.$.open = !ctx.$.open
    ctx.log.info(format(cart.total))
  },
}
