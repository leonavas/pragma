import { store } from '../../core/state.js'
import { brl } from '../../utils/dinheiro.js'

function ajustar(item, qtd) {
  item.qtd = Math.min(qtd, item.estoque)
  item.cheio = item.qtd >= item.estoque
  item.subtotal = brl(item.qtd * item.preco)
}

export default store({
  itens: [],
  get quantidade() { return this.itens.reduce((n, i) => n + i.qtd, 0) },
  get total() { return brl(this.itens.reduce((n, i) => n + i.qtd * i.preco, 0)) },
  get vazio() { return this.itens.length === 0 },
  adicionar({ id, nome, preco, estoque }, qtd) {
    let item = this.itens.find(i => i.id === id)
    if (!item) this.itens.push((item = { id, nome, preco, estoque, qtd: 0 }))
    ajustar(item, item.qtd + qtd)
  },
  mudar(id, delta) {
    const item = this.itens.find(i => i.id === id)
    if (item.qtd + delta < 1) return this.remover(id)
    ajustar(item, item.qtd + delta)
  },
  remover(id) { this.itens = this.itens.filter(i => i.id !== id) },
  esvaziar() { this.itens = [] },
}, { persist: 'carrinho' })
