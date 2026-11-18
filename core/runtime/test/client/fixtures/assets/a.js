export const state = () => ({ ticks: 0 })

export function mount(ctx) {
  self.__ticks = 0
  const t = setInterval(() => ctx.update(s => { s.ticks = ++self.__ticks }), 20)
  return () => {
    clearInterval(t)
    self.__limpou = true
  }
}
