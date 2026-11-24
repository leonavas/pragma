import { isPlain } from './contract.js'

// Deep Proxy over plain objects and arrays, with read tracking. A reader is a binding record
// [el, binding, scope], a model record or an owner record { el, s, o, dp } (see dom.js); a write
// calls S.n(target, key), which schedules exactly the readers of that key.
const proxies = new WeakMap(), raws = new WeakMap()
export const S = { n() {}, c: null } // n: notify; c: target being created (its writes do not notify)

let reads = new WeakMap(), reader = null
export const readBy = r => { reader = r }
export const forgetReads = () => { reads = new WeakMap() }
export const readers = (t, k) => reads.get(t)?.get(k)

function track(t, k) {
  if (reader === null || typeof k === 'symbol') return
  let m = reads.get(t)
  if (!m) reads.set(t, (m = new Map()))
  const s = m.get(k)
  s ? s.add(reader) : m.set(k, new Set([reader]))
}

const handler = {
  get(t, k, r) {
    track(t, k)
    const v = Reflect.get(t, k, r)
    return typeof k === 'symbol' ? v : reactive(v)
  },
  has(t, k) {
    track(t, k)
    return k in t
  },
  set(t, k, v) {
    v = raws.get(v) ?? v
    const had = Object.hasOwn(t, k), old = t[k], d = Object.getOwnPropertyDescriptor(t, k)
    if (d?.get && !d.set) throw new TypeError(`pragma: "${String(k)}" is a prop and props are read-only`)
    t[k] = v
    if (t !== S.c && (!had || old !== v || (Array.isArray(t) && k === 'length'))) S.n(t, k)
    return true
  },
  deleteProperty(t, k) {
    const had = Object.hasOwn(t, k)
    delete t[k]
    if (had) S.n(t, k)
    return true
  },
}

export function reactive(v) {
  if (v === null || typeof v !== 'object' || raws.has(v) || !isPlain(v)) return v
  let p = proxies.get(v)
  if (!p) proxies.set(v, (p = new Proxy(v, handler))), raws.set(p, v)
  return p
}

// While a script creates its own state (the top-level `let`s of a component, page or store),
// its writes do not notify: the render that created the instance reads them next.
export function create(t, fn) {
  const o = S.c
  S.c = t
  try { return fn() } finally { S.c = o }
}
