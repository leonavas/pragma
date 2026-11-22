export const persist = ['mode']

export default function ($) {
  $.mode = 'narrow'

  function set(mode) {
    $.mode = mode
  }

  return { set }
}
