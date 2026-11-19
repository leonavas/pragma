export default function ($, $pragma) {
  $.name = 'Ana'
  $.active = true
  $.link = '/x'
  $.disabled = false
  $.title = null
  $.visible = true
  $.person = { name: 'Bia' }
  $.items = [
    { id: 1, name: 'one', ok: true, tags: [{ t: 'a' }, { t: 'b' }] },
    { id: 2, name: 'two', ok: false, tags: [] },
  ]
  $.colors = ['blue', 'green', 'purple']
  $.color = 'green'
  $.accepted = false
  $.qty = 2
  $.size = 'p'
  $.saving = false
  $.response = null
  $.error = null
  $.tick = 0
  $.sent = false

  function toggle() { $.visible = !$.visible }
  function remove(item) { $.items = $.items.filter(i => i !== item) }
  async function save() {
    $.saving = true
    $.response = await $pragma.call('save', { name: $.name })
    $.saving = false
  }
  async function fail() {
    try { await $pragma.call('fail') } catch (e) { $.error = e.message }
  }
  function buy() { $pragma.shared.cart.items.push({ id: $pragma.shared.cart.items.length + 1 }) }
  function submit(event) { $.sent = event.defaultPrevented }

  function mount(el) {
    self.__ctx = { $, update: fn => fn($) }
    self.__pageRoot = el.tagName
  }

  return { toggle, remove, save, fail, buy, submit, mount }
}
