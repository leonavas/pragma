// Reactive values: a deep Proxy over plain objects and arrays. Every change calls `notify(target, key)`,
// which page.js turns into one render per microtask. Functions, DOM nodes and class instances are not wrapped.
const proxies = new WeakMap(), raws = new WeakMap(), persisted = []
let notify = () => {}

// What the last full render read: per (object, key), the bindings that read it, or ANY when a block,
// a prop or a component read it (a change there can change the page's structure).
const ANY = 0
let reads = new WeakMap(), reader = null
export const readBy = r => { reader = r }
export const forgetReads = () => { reads = new WeakMap() }
export function readers(target, key) {
  const r = reads.get(target)?.get(key)
  return r === ANY ? undefined : r
}

function read(target, key) {
  let m = reads.get(target)
  if (!m) reads.set(target, (m = new Map()))
  const r = m.get(key)
  if (r === ANY) return
  if (reader === ANY) m.set(key, ANY)
  else if (r) r.add(reader)
  else m.set(key, new Set([reader]))
}

export const onChange = fn => { notify = fn }
export const raw = v => raws.get(v) ?? v

function wrappable(v) {
  if (v === null || typeof v !== 'object' || raws.has(v)) return false
  const proto = Object.getPrototypeOf(v)
  return proto === Object.prototype || proto === null || Array.isArray(v)
}

const handler = {
  get(target, key, receiver) {
    const v = Reflect.get(target, key, receiver)
    if (typeof key === 'symbol') return v
    if (reader !== null) read(target, key)
    return reactive(v)
  },
  set(target, key, v) {
    const value = raw(v), had = Object.hasOwn(target, key), old = target[key]
    const desc = Object.getOwnPropertyDescriptor(target, key)
    if (desc?.get && !desc.set) throw new TypeError(`pragma: "${String(key)}" is a prop and props are read-only`)
    target[key] = value
    if (!had || old !== value || (Array.isArray(target) && key === 'length')) notify(target, key)
    return true
  },
  deleteProperty(target, key) {
    const had = Object.hasOwn(target, key)
    delete target[key]
    if (had) notify()
    return true
  },
}

export function reactive(v) {
  if (!wrappable(v)) return v
  let p = proxies.get(v)
  if (!p) {
    p = new Proxy(v, handler)
    proxies.set(v, p)
    raws.set(p, v)
  }
  return p
}

// Store fields listed in `export const persist = [...]` live in localStorage['pragma:<store>'].
const key = name => 'pragma:' + name
const pick = (target, keys) => JSON.stringify(Object.fromEntries(keys.filter(k => k in target).map(k => [k, target[k]])))

function restore(target, keys, saved) {
  if (saved && typeof saved === 'object') for (const k of keys) if (k in saved) target[k] = saved[k]
}

export function persist(target, name, keys) {
  if (!keys?.length) return
  try { restore(target, keys, JSON.parse(localStorage.getItem(key(name)))) } catch {}
  persisted.push({ target, keys, k: key(name), j: pick(target, keys) })
}

export function persistStores() {
  for (const p of persisted) {
    const j = pick(p.target, p.keys)
    if (j === p.j) continue
    p.j = j
    try { localStorage.setItem(p.k, j) } catch {}
  }
}

addEventListener('storage', e => {
  const p = persisted.find(p => p.k === e.key)
  if (!p || e.newValue == null) return
  try { restore(p.target, p.keys, JSON.parse(e.newValue)) } catch { return }
  p.j = e.newValue
  notify()
})
