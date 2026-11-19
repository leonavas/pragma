export const state = () => ({ n: 0 })
export const actions = {
  inc(ctx) { ctx.$.n++ },
}
export function mount() {
  self.__mounted = (self.__mounted ?? 0) + 1
  return () => { self.__unmounted = (self.__unmounted ?? 0) + 1 }
}
