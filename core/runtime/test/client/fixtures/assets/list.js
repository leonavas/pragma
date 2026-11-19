export default function ($) {
  $.sel = 0
  $.items = Array.from({ length: 1000 }, (_, i) => ({ id: i, name: 'item ' + i, done: i % 2 === 0 }))

  function mount() {
    self.__ctx = { $, update: fn => fn($) }
  }

  return { mount }
}
