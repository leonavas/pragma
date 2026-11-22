export default function ($) {
  $.got = ''

  function nums(...values) {
    $.got = values.map(v => typeof v + ':' + v).join('|')
  }

  return { nums }
}
