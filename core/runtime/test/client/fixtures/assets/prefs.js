export default function ($) {
  $.got = ''
  $.parts = [{ id: 1, name: 'lid' }]

  function nums(...values) {
    $.got = values.map(v => typeof v + ':' + v).join('|')
  }

  function pick(part) {
    $.got = typeof part === 'object' ? part.name : typeof part
  }

  return { nums, pick }
}
