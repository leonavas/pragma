const persisted = []
const key = k => 'pragma:' + k
let notify = () => {}

export const onStoreChange = fn => { notify = fn }

// Getters são valores derivados: não vão para o localStorage nem são sobrescritos por ele.
const plain = s => JSON.stringify(s, function (k, v) { return this === s && Object.getOwnPropertyDescriptor(s, k)?.get ? undefined : v })
const restore = (s, v) => { for (const k in v) if (!Object.getOwnPropertyDescriptor(s, k)?.get) s[k] = v[k] }

export function store(initial, { persist } = {}) {
  if (persist) {
    try { restore(initial, JSON.parse(localStorage.getItem(key(persist)))) } catch {}
    persisted.push({ s: initial, k: key(persist), j: plain(initial) })
  }
  return initial
}

export function persistStores() {
  for (const p of persisted) {
    const j = plain(p.s)
    if (j === p.j) continue
    p.j = j
    try { localStorage.setItem(p.k, j) } catch {}
  }
}

addEventListener('storage', e => {
  const p = persisted.find(p => p.k === e.key)
  if (!p || e.newValue == null) return
  restore(p.s, JSON.parse(e.newValue))
  p.j = e.newValue
  notify()
})
