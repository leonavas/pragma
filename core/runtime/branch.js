import { H, L, RD, ctx, visit, clone, of, sib, get } from './dom.js'

// <if> / <else-if> / <else>, and <empty> after a <for>: the body sits right after its anchor.
function branch(t, d, s, last, id) {
  const chained = d.t > 2 && last, on = !chained && (d.t === 3 || !get(s, d.p) === !!d.n)
  let n = sib(t)
  if (!of(n, id)) n = null
  if (!on) {
    if (n) H.drop(n)
  } else if (n) {
    if (n[L]) H.stay(n)
    visit(n, s)
  } else {
    t.after((n = clone(t, d, id)))
    visit(n, s)
    ctx.focus = (n.matches('[autofocus]') ? n : n.querySelector('[autofocus]')) ?? ctx.focus
  }
  t[RD] = 1
  return on ? [n, true, id] : [t, chained, id]
}

export default function install() {
  H.branch = branch
}
