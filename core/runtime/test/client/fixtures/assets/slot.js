export const state = ctx => ({
  titulo: 'da página',
  campo: '',
  mostrar: true,
  cliques: 0,
  escolhido: '',
  marcado: false,
  editando: false,
  itens: [{ id: 1, nome: 'um', max: 3 }, { id: 2, nome: 'dois', max: 7 }],
  get dobro() { return ctx.shared.conta.total * 2 },
})

export const actions = {
  pagina(ctx) { ctx.state.cliques++; ctx.shared.conta.n++ },
  escolher(ctx) { ctx.state.escolhido = ctx.scope.item.nome },
}

export function mount(ctx) {
  self.__slot = ctx
}
