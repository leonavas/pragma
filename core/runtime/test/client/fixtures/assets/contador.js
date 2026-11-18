export const state = () => ({ n: 0 })
export const actions = {
  mais(ctx) { ctx.state.n++ },
}
export function mount() {
  self.__montados = (self.__montados ?? 0) + 1
  return () => { self.__desmontados = (self.__desmontados ?? 0) + 1 }
}
