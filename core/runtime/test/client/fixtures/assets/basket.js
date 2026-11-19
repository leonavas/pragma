export default function ($) {
  $.items = [{ id: 1, name: 'Pen' }, { id: 2, name: 'Ink' }]

  function remove(id) {
    $.items = $.items.filter(item => item.id !== id)
  }

  return { remove }
}
