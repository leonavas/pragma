const freeShippingFrom = 50

export const state = ctx => ({
  get remaining() { return Math.max(freeShippingFrom - ctx.shared.cart.total, 0) },
  get qualifies() { return this.remaining === 0 },
  get message() {
    return this.qualifies ? 'Free shipping unlocked.' : `Add $${this.remaining} more for free shipping.`
  },
})
