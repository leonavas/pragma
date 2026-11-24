import { H } from './dom.js'
import { STORE_KEY } from './contract.js'

// Stores that export `persist` (a list of keys): restored synchronously when the store is created,
// before the first render and mount; saved after each flush that changed them.
const persisted = []
const pick = (t, keys) => JSON.stringify(Object.fromEntries(keys.filter(k => k in t).map(k => [k, t[k]])))
function restore(t, keys, saved) {
  if (saved && typeof saved === 'object') for (const k of keys) if (k in saved) t[k] = saved[k]
}

export default function install(K) {
  K.persist = (target, name, keys) => {
    const k = STORE_KEY(name)
    try { restore(target, keys, JSON.parse(localStorage.getItem(k))) } catch {}
    persisted.push({ target, keys, k, j: pick(target, keys) })
  }
  H.flushed = () => {
    for (const p of persisted) {
      const j = pick(p.target, p.keys)
      if (j === p.j) continue
      p.j = j
      try { localStorage.setItem(p.k, j) } catch {}
    }
  }
  // Another tab changed a store: restore it and render the whole page.
  addEventListener('storage', e => {
    const p = persisted.find(p => p.k === e.key)
    if (!p || e.newValue == null) return
    try { restore(p.target, p.keys, JSON.parse(e.newValue)) } catch { return }
    p.j = e.newValue
    K.schedule()
  })
}
