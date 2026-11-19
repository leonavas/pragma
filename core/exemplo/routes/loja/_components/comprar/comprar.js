export const state = ({ $ }) => ({
  qtd: 1,
  get esgotado() { return Number($.estoque) === 0 },
  get semMenos() { return this.qtd <= 1 },
  get semMais() { return this.qtd >= Number($.estoque) },
})

export const actions = {
  menos(ctx) { ctx.$.qtd-- },
  mais(ctx) { ctx.$.qtd++ },
  adicionar(ctx) {
    const { id, nome, preco, estoque } = ctx.$
    ctx.shared.carrinho.adicionar({ id, nome, preco: Number(preco), estoque: Number(estoque) }, ctx.$.qtd)
    ctx.log.info('produto adicionado ao carrinho', { 'produto.id': id, 'carrinho.item.quantidade': ctx.$.qtd })
    ctx.$.qtd = 1
  },
}
