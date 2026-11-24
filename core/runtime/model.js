import { H, B, D, S, get } from './dom.js'

// data-model (record kind 7): state -> control on render, control -> state on input.
function model(el, b, s) {
  const v = get(s, b[1]), t = el.type
  if (t === 'checkbox' || t === 'radio') {
    const c = t === 'radio' ? v === el.value : !!v
    if (el.checked !== c) el.checked = c
    return
  }
  const same = t === 'number' || t === 'range' ? (v == null ? el.value === '' : el.valueAsNumber === v) : el.value === String(v ?? '')
  if (!same) el.value = v ?? ''
}

// Writes through the Proxy: the write schedules the readers of that key.
function input(e) {
  const el = e.target, p = el[D]?.m, t = el.type
  if (!p || !el[S] || (t === 'radio' && !el.checked)) return
  get(el[S], p.slice(0, -1))[p.at(-1)] = t === 'checkbox' ? el.checked : t === 'number' || t === 'range' ? (el.value === '' ? null : el.valueAsNumber) : el.value
}

export default function install() {
  B[7] = model
  H.input = input
}
