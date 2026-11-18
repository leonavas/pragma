import { store } from '../../core/state.js'

const nova = titulo => ({ id: crypto.randomUUID(), titulo, feita: false })
const igual = (a, b) => a.localeCompare(b, 'pt-BR', { sensitivity: 'accent' }) === 0

export default store({
  itens: [nova('Comprar café'), { ...nova('Pagar contas'), feita: true }, nova('Ligar para a farmácia')],
  get pendentes() { return this.itens.filter(t => !t.feita).length },
  get concluidas() { return this.itens.length - this.pendentes },
  get resumo() { return `${this.concluidas} de ${this.itens.length}` },
  existe(titulo, exceto) { return this.itens.some(t => t.id !== exceto && igual(t.titulo, titulo)) },
  adicionar(titulo) { this.itens.push(nova(titulo)) },
  achar(id) { return this.itens.find(t => t.id === id) },
  remover(id) { this.itens = this.itens.filter(t => t.id !== id) },
  limparConcluidas() { this.itens = this.itens.filter(t => !t.feita) },
}, { persist: 'tarefas' })
