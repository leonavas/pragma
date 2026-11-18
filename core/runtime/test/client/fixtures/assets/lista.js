export const state = () => ({
  sel: 0,
  itens: Array.from({ length: 1000 }, (_, i) => ({ id: i, nome: 'item ' + i, feito: i % 2 === 0 })),
})

export function mount(ctx) {
  self.__ctx = ctx
}
