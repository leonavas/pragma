export const state = () => ({
  sel: 0,
  items: Array.from({ length: 1000 }, (_, i) => ({ id: i, name: 'item ' + i, done: i % 2 === 0 })),
})

export function mount(ctx) {
  self.__ctx = ctx
}
