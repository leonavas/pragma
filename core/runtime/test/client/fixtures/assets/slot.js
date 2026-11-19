export default function ($, $pragma) {
  $.title = 'from page'
  $.field = ''
  $.visible = true
  $.clicks = 0
  $.picked = ''
  $.marked = false
  $.editing = false
  $.items = [{ id: 1, name: 'one', max: 3 }, { id: 2, name: 'two', max: 7 }]

  const double = () => $pragma.shared.account.total() * 2

  function page() { $.clicks++; $pragma.shared.account.n++ }
  function pick(item) { $.picked = item.name }

  function mount() {
    self.__slot = { $, update: fn => fn($) }
  }

  return { double, page, pick, mount }
}
