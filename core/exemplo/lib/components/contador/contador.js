const num = (v, padrao) => (v ? Number(v) : padrao)

export const state = ({ props }) => ({
  n: num(props.inicial, 0),
  get passo() { return num(props.passo, 1) },
  get semMenos() { return this.n - this.passo < num(props.min, -Infinity) },
  get semMais() { return this.n + this.passo > num(props.max, Infinity) },
})

export const actions = {
  menos(ctx) { ctx.state.n -= ctx.state.passo },
  mais(ctx) { ctx.state.n += ctx.state.passo },
}
