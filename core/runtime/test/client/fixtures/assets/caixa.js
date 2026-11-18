export const state = ctx => ({ titulo: 'da caixa', n: Number(ctx.props.inicial), max: ctx.props.max ?? '' })

export const actions = {
  mais(ctx) { ctx.state.n++ },
  caixa() {},
}
