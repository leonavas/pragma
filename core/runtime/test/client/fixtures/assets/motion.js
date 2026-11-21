export default function ($) {
  $.on = true
  $.items = ['a', 'b', 'c']
  $.plain = true

  function mount() {
    self.__ctx = { $, update: fn => fn($) }
  }

  return { mount }
}
