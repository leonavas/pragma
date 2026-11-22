export default function ($, pragma) {
  $.x = 0
  $.who = 'Ana'
  $.items = Array.from({ length: 1000 }, (_, i) => ({ id: i, name: 'item ' + i }))

  function mount() {
    self.__ctx = { $, frame: pragma.frame }
  }

  return { mount }
}
