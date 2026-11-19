const toNumber = (prop, fallback) => (prop ? Number(prop) : fallback)

export const state = ({ $ }) => ({
  count: toNumber($.start, 0),
  get stepSize() { return toNumber($.step, 1) },
  get atMin() { return this.count - this.stepSize < toNumber($.min, -Infinity) },
  get atMax() { return this.count + this.stepSize > toNumber($.max, Infinity) },
})

export const actions = {
  decrement(ctx) { ctx.$.count -= ctx.$.stepSize },
  increment(ctx) { ctx.$.count += ctx.$.stepSize },
}
