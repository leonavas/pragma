export default function ($) {
  $.n = 0

  function inc() { $.n++ }

  function mount(el) {
    self.__mounted = (self.__mounted ?? 0) + 1
    ;(self.__mountRoots ??= []).push(el.dataset.c)
    return () => { self.__unmounted = (self.__unmounted ?? 0) + 1 }
  }

  return { inc, mount }
}
