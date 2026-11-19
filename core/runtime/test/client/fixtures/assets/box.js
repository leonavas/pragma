export default function ($, $pragma) {
  $pragma.prop('initial', 0)
  $pragma.prop('max', '')
  $pragma.prop('row', null)

  $.title = 'from box'
  $.n = $.initial

  function inc() { $.n++ }
  function box() {}

  function mount() {
    (self.__boxes ??= []).push($)
  }

  return { inc, box, mount }
}
