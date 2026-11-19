import { render, hooks, scopeOf, input } from './directives.js'
import { log } from './log.js'
import { persistStores, onStoreChange } from './state.js'
import { load, call } from './loader.js'
import { navigate } from './router.js'

let ov
export let overlay
// overlay.js is only fetched on pages with overlays; the build modulepreloads it there.
export const overlays = () => (ov ??= import('./overlay.js').then(m => (m.initOverlays({ log }), (overlay = m))))
const open = (...a) => overlays().then(m => m.open(...a))
const close = (...a) => overlays().then(m => m.close(...a))
const confirm = (...a) => overlays().then(m => m.confirm(...a))

const C = Symbol(), O = Symbol(), mounted = new Set(), comps = {}, types = new Set()
export const shared = {}
let cur, queued, mounts = []

export function schedule() {
  if (queued) return
  queued = true
  queueMicrotask(flush)
}

function flush() {
  queued = false
  if (!cur) return
  render(document.body, { __proto__: null, shared, params: cur.params, query: cur.query, page: cur.page })
  persistStores()
  for (const o of mounts.splice(0)) {
    if (!mounted.has(o)) continue
    try { o.off = o.mod.mount?.(o.ctx) } catch (e) { reportError(e) }
  }
}

function owner(el, mod) {
  const o = { el, mod }
  o.ctx = {
    el, shared, log, open, close, confirm, navigate,
    get data() { return cur.data },
    get params() { return cur.params },
    get query() { return cur.query },
    get page() { return cur.page },
    call: (name, data) => call(cur.url, name, data),
    update(fn) { fn(o.state); schedule() },
    $: new Proxy({}, {
      get: (_, key) => o.state && key in o.state ? o.state[key] : el.dataset[key],
      set: (_, key, value) => (o.state[key] = value, true),
      has: (_, key) => (o.state && key in o.state) || key in el.dataset,
    }),
  }
  o.state = mod.state?.(o.ctx) ?? {}
  mounted.add(o)
  mounts.push(o)
  return (el[C] = o)
}

function unmount(o) {
  mounted.delete(o)
  try { o.off?.() } catch (e) { reportError(e) }
}

hooks.on = t => types.has(t) || (types.add(t), document.addEventListener(t, onEvent, true))
hooks.remove = n => { for (const o of mounted) if (n.contains(o.el)) unmount(o) }
hooks.enter = (el, s, x) => {
  if (x === 2) return { __proto__: s, data: cur.data, state: cur.o.state, [O]: cur.o }
  const mod = comps[el.dataset.c]
  if (!mod) return s
  const o = el[C] ?? owner(el, mod)
  return { __proto__: s, state: o.state, [O]: o }
}

function onEvent(e) {
  if (e.type === 'input' && input(e)) schedule()
  const el = e.target.closest?.(`[data-on-${e.type}]`)
  if (!el) return
  const o = scopeOf(el)?.[O] ?? cur.o, name = el.getAttribute('data-on-' + e.type), fn = o.mod.actions?.[name]
  if (!fn) {
    if (self.__pragmaDev) throw new Error(`pragma: unknown action "${name}"`)
    return
  }
  if (e.type === 'submit') e.preventDefault()
  let r
  try { r = fn({ __proto__: o.ctx, event: e, el, scope: scopeOf(el) }) } finally { schedule() }
  if (typeof r?.then === 'function') r.finally(schedule)
}

onStoreChange(schedule)

export async function importRoute(r) {
  const all = o => Promise.all(Object.entries(o ?? {}).map(async ([k, u]) => [k, await import(u)]))
  const [page, c, s] = await Promise.all([r.page && import(r.page), all(r.components), all(r.stores)])
  return { page, comps: Object.fromEntries(c), stores: Object.fromEntries(s.map(([k, m]) => [k, m.default])) }
}

export function leave() {
  if (!cur) return
  for (const o of mounted) if (cur.o.el.contains(o.el)) unmount(o)
}

export function enter(route, mods, url, params, doc, pending) {
  Object.assign(comps, mods.comps)
  Object.assign(shared, mods.stores)
  if (document.querySelector('dialog,[popover]')) overlays()
  const p = (cur = { route, url, params, query: Object.fromEntries(url.searchParams), page: { loading: false, error: null } })
  p.o = owner(document.querySelector('main[data-page]'), mods.page ?? {})
  const done = load(p, doc, pending).then(() => cur === p && schedule())
  flush()
  return done
}
