const num = (v, padrao) => (v ? Number(v) : padrao)

export const state = ({ $ }) => ({
  n: num($.inicial, 0),
  get passo() { return num($.passo, 1) },
  get semMenos() { return this.n - this.passo < num($.min, -Infinity) },
  get semMais() { return this.n + this.passo > num($.max, Infinity) },
})

export const actions = {
  menos(ctx) { ctx.$.n -= ctx.$.passo },
  mais(ctx) { ctx.$.n += ctx.$.passo },
}
