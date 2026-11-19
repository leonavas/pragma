export default function ($) {
  $.ticks = 0

  async function mount() {
    await null
    self.__ticks = 0
    const t = setInterval(() => { $.ticks = ++self.__ticks }, 20)
    return () => {
      clearInterval(t)
      self.__cleaned = true
    }
  }

  return { mount }
}
