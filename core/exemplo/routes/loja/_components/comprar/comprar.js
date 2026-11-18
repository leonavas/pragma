export const state = ({ props }) => ({
  qtd: 1,
  get esgotado() { return Number(props.estoque) === 0 },
  get semMenos() { return this.qtd <= 1 },
  get semMais() { return this.qtd >= Number(props.estoque) },
})

export const actions = {
  menos(ctx) { ctx.state.qtd-- },
  mais(ctx) { ctx.state.qtd++ },
  adicionar(ctx) {
    const { id, nome, preco, estoque } = ctx.props
    ctx.shared.carrinho.adicionar({ id, nome, preco: Number(preco), estoque: Number(estoque) }, ctx.state.qtd)
    ctx.log.info('produto adicionado ao carrinho', { 'produto.id': id, 'carrinho.item.quantidade': ctx.state.qtd })
    ctx.state.qtd = 1
  },
}
