export default function ($) {
  $.x = 0
  $.y = 'a'
  $.open = false
  $.items = Array.from({ length: 1000 }, (_, i) => ({ id: i, name: 'item ' + i, done: i % 2 === 0 }))
  self.__rows = 0

  const rows = () => (self.__rows++, $.items)
  const far = () => $.x > 30
  const label = () => (far() ? 'far ' + $.y : 'near')

  function mount() {
    self.__ctx = { $, update: fn => fn($) }
  }

  return { mount, rows, far, label }
}
