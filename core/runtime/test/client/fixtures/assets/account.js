export const persist = ['n']

export default function ($) {
  $.n = 1

  const total = () => $.n * 10

  return { total }
}
