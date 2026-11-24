import { B, DEV, fail, get } from './dom.js'
import { cssValue } from './contract.js'

// :--name="path" (binding kind 6): a CSS custom property on the element.
function cssVar(el, [, p, , a], s) {
  const v = cssValue(get(s, p), v => (DEV ? fail(`:--${a} got "${v}": a CSS value cannot contain ; { } or <`) : null))
  if (el.style.getPropertyValue('--' + a) !== (v ?? '')) v == null ? el.style.removeProperty('--' + a) : el.style.setProperty('--' + a, v)
}

export default function install() {
  B[6] = cssVar
}
