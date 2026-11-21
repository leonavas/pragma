import { render, hooks, scopeOf, input, eventCall } from './directives.js'
import { log } from './log.js'
import { reactive, onChange, persist, persistStores } from './state.js'
import { coerce } from './props.js'
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
  // `function mount(el)`: el is the component root, or the page's `<main data-page>`
  for (const o of mounts.splice(0)) {
    if (!mounted.has(o)) continue
    try { cleanup(o, o.mount?.(o.el)) } catch (e) { reportError(e) }
  }
}

// `async function mount(el)`: the cleanup is what the promise resolves to; if the component is gone
// by then, it runs at once.
function cleanup(o, off) {
  if (typeof off?.then !== 'function') return void (o.off = off)
  off.then(fn => {
    if (typeof fn !== 'function') return
    if (mounted.has(o)) o.off = fn
    else try { fn() } catch (e) { reportError(e) }
  }, reportError)
}

// Dev: a prop object is read-only all the way down (writing to it would change the caller's data).
const views = new WeakMap()
function readonly(v, name) {
  if (v === null || typeof v !== 'object') return v
  const proto = Object.getPrototypeOf(v)
  if (proto !== Object.prototype && proto !== null && !Array.isArray(v)) return v
  let view = views.get(v)
  if (!view) {
    const deny = () => {
      throw new TypeError(`pragma: "${name}" is a prop: props are read-only, also inside; copy what you change into a let (\`let rows = ${name}.map(row => ({ ...row }))\`)`)
    }
    const get = (t, k, r) => (typeof k === 'symbol' ? Reflect.get(t, k, r) : readonly(Reflect.get(t, k, r), name))
    view = new Proxy(v, { get, set: deny, deleteProperty: deny, defineProperty: deny })
    views.set(v, view)
  }
  return view
}

// What `import { … } from 'pragma'` gives a script (the build rewrites `data` to `$pragma.data`): live
// values, so `data` is the current page's once the loader is done.
const services = {
  shared, log, navigate, open, close, confirm,
  get data() { return reactive(cur?.data) },
  get params() { return cur?.params },
  get query() { return cur?.query },
  get page() { return cur?.page },
  call: (name, data) => call(cur.url, name, data),
}

// A page or component instance: `scope` is what its HTML reads as `$x` (state, props, functions).
// Props come from data-with-<name> (values from the caller's scope, `w`) or data-prop-<name> (text).
function instance(el, mod, w) {
  const target = {}, o = { el, w, target }
  o.scope = reactive(target)
  const prop = (name, d) => {
    const k = name.toLowerCase()
    Object.defineProperty(target, name, {
      enumerable: true,
      get: () => {
        const v = coerce(o.w && Object.hasOwn(o.w, k) ? o.w[k] : el.getAttribute('data-prop-' + k) ?? undefined, d)
        return self.__pragmaDev ? readonly(v, name) : v
      },
    })
  }
  const api = mod.default?.(o.scope, { __proto__: services, prop }) ?? {}
  for (const k in api) if (k !== 'mount') target[k] = api[k]
  o.mount = api.mount
  mounted.add(o)
  mounts.push(o)
  return (el[C] = o)
}

function store(name, mod) {
  const target = {}, scope = reactive(target)
  const api = mod.default?.(scope, services) ?? {}
  for (const k in api) target[k] = api[k]
  persist(target, name, mod.persist)
  return scope
}

function unmount(o) {
  mounted.delete(o)
  try { o.off?.() } catch (e) { reportError(e) }
}

hooks.on = t => types.has(t) || (types.add(t), document.addEventListener(t, onEvent, true))
hooks.remove = n => { for (const o of mounted) if (n.contains(o.el)) unmount(o) }
hooks.enter = (el, s, x, w = null) => {
  if (x === 2) return { __proto__: s, data: cur.data, state: cur.o.scope, [O]: cur.o }
  const mod = comps[el.dataset.c]
  if (!mod) return s
  const o = el[C] ?? instance(el, mod, w)
  o.w = w
  return { __proto__: s, state: o.scope, [O]: o }
}

function onEvent(e) {
  if (e.type === 'input' && input(e)) schedule()
  const el = e.target.closest?.(`[data-on-${e.type}]`)
  if (!el) return
  const o = scopeOf(el)?.[O] ?? cur.o, [name, args, store] = eventCall(el, e.type, e), fn = store ?? o.target[name]
  if (typeof fn !== 'function') {
    if (self.__pragmaDev) throw new Error(`pragma: @${e.type}="${name}" needs a function, but "${name}" holds ${fn === null ? 'null' : typeof fn} (in the ${name.includes('.') ? 'store' : 'script'}): use a function or a const holding one (\`const saveLater = debounce(save, 300)\`)`)
    return
  }
  if (e.type === 'submit') e.preventDefault()
  let r
  try { r = fn(...args) } finally { schedule() }
  if (typeof r?.then === 'function') r.finally(schedule)
}

onChange(schedule)

export async function importRoute(r) {
  const all = o => Promise.all(Object.entries(o ?? {}).map(async ([k, u]) => [k, await import(u)]))
  const [page, c, s] = await Promise.all([r.page && import(r.page), all(r.components), all(r.stores)])
  return { page, comps: Object.fromEntries(c), stores: Object.fromEntries(s) }
}

export function leave() {
  if (!cur) return
  for (const o of mounted) if (cur.o.el.contains(o.el)) unmount(o)
}

export function enter(route, mods, url, params, doc, pending) {
  Object.assign(comps, mods.comps)
  for (const [name, mod] of Object.entries(mods.stores)) shared[name] ??= store(name, mod)
  if (document.querySelector('dialog,[popover]')) overlays()
  const p = (cur = { route, url, params, query: Object.fromEntries(url.searchParams), page: { loading: false, error: null } })
  // with ssr the loader sets `data` synchronously, so the page script sees it as the server did
  const done = load(p, doc, pending).then(() => cur === p && schedule())
  p.o = instance(document.querySelector('main[data-page]'), mods.page ?? {}, null)
  flush()
  return done
}
