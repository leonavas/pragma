export default function ($, $pragma) {
  $.locked = true
  function open() { $pragma.open('m') }

  function mount() {
    self.__overlay = { $, update: fn => fn($) }
  }

  return { open, mount }
}
