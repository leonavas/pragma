import { render, patch, hooks, scopeOf, input, eventCall, useBodies, DEV, PAGE } from './directives.js'
import { log } from './log.js'
import { reactive, readers, onChange, persist, persistStores } from './state.js'
import { fetchData, load, loadResult, call, stream } from './loader.js'
import { navigate, redirect } from './router.js'
import { DATA_ID, MAIN, PERSIST_SUFFIX, matchPattern, coerce, instantiate, isPlain, readonlyViews } from './contract.js'

// The build sets `overlays` in the route and modulepreloads overlay.js only on pages with a dialog or popover.
let overlayModule = null, overlayLoad = null
const overlays = () => (overlayLoad ??= import('./overlay.js').then(m => (m.initOverlays(), (overlayModule = m))))
const open = (...a) => overlays().then(m => m.open(...a))
const close = (...a) => overlays().then(m => m.close(...a))
const confirm = (...a) => overlays().then(m => m.confirm(...a))
export const closeOverlays = () => overlayModule?.closeAll() ?? 0
export const popOverlay = () => overlayModule?.handlePopState() ?? false

const C = Symbol(), O = Symbol(), mounted = new Set(), comps = {}, types = new Set()
const shared = {}
let cur, queued, whole, mounts = []
const dirty = new Set()

function schedule(target, key) {
  if (!whole) {
    const r = target && readers(target, key)
    if (r) dirty.add(r)
    else whole = true
  }
  if (queued) return
  queued = true
  queueMicrotask(flush)
}

function flush() {
  queued = false
  if (!cur) return
  const groups = [...dirty]
  dirty.clear()
  if (!whole) {
    patch(groups)
    return persistStores()
  }
  whole = false
  render(document.body, { __proto__: null, shared, params: cur.params, query: cur.query, page: cur.page })
  persistStores()
  for (const o of mounts.splice(0)) {
    if (!mounted.has(o)) continue
    try { cleanup(o, o.mount?.(o.el)) } catch (e) { reportError(e) }
  }
}

function cleanup(o, off) {
  if (typeof off?.then !== 'function') return void (o.off = off)
  off.then(fn => {
    if (typeof fn !== 'function') return
    if (mounted.has(o)) o.off = fn
    else try { fn() } catch (e) { reportError(e) }
  }, reportError)
}

let frameCallbacks = [], frameRequest = 0

function runFrame(now) {
  frameRequest = 0
  const callbacks = frameCallbacks
  frameCallbacks = []
  for (const fn of callbacks) try { fn(now) } catch (e) { reportError(e) }
}

function frame(fn) {
  frameCallbacks.push(fn)
  frameRequest ||= requestAnimationFrame(runFrame)
  return () => {
    const i = frameCallbacks.indexOf(fn)
    if (i >= 0) frameCallbacks.splice(i, 1)
  }
}

const readonly = readonlyViews(isPlain, name => `pragma: "${name}" is a prop: props are read-only, also inside; copy what you change into a let (\`let rows = ${name}.map(row => ({ ...row }))\`)`)

const services = {
  shared, log, navigate, open, close, confirm, frame,
  get data() { return reactive(cur?.data) },
  get params() { return cur?.params },
  get query() { return cur?.query },
  get page() { return cur?.page },
  call: (name, data) => call(cur.url, name, data, navigate),
  stream: (name, data, options) => stream(cur.url, name, data, options, navigate),
}

function instance(el, mod, w) {
  const target = {}, o = { el, w, target }
  o.scope = reactive(target)
  o.mount = instantiate(mod, target, o.scope, services, (k, d, name) => {
    const v = coerce(o.w && Object.hasOwn(o.w, k) ? o.w[k] : el.getAttribute('data-prop-' + k) ?? undefined, d)
    return DEV ? readonly(v, name) : v
  })
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
  if (x === PAGE) return { __proto__: s, data: reactive(cur.data), state: cur.o.scope, [O]: cur.o }
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
    if (DEV) throw new Error(`pragma: @${e.type}="${name}" needs a function, but "${name}" holds ${fn === null ? 'null' : typeof fn} (in the ${name.includes('.') ? 'store' : 'script'}): use a function or a const holding one (\`const saveLater = debounce(save, 300)\`)`)
    return
  }
  if (e.type === 'submit') e.preventDefault()
  let r
  try { r = fn(...args) } finally { schedule() }
  if (typeof r?.then === 'function') r.finally(schedule)
}

onChange(schedule)

const importPart = async u => {
  const [file, name] = u.split('#')
  const m = await import(file)
  return name ? { default: m[name], persist: m[name + PERSIST_SUFFIX] } : m
}

export async function importRoute(r) {
  const all = o => Promise.all(Object.entries(o ?? {}).map(async ([k, u]) => [k, await importPart(u)]))
  const [page, c, s, bodies] = await Promise.all([r.page && importPart(r.page), all(r.components), all(r.stores), r.tpl && import(r.tpl)])
  return { page, comps: Object.fromEntries(c), stores: Object.fromEntries(s), bodies }
}

export function leave() {
  if (!cur) return
  for (const o of mounted) if (cur.o.el.contains(o.el)) unmount(o)
}

function settle(p, result) {
  p.data = result.data
  p.page.error = result.error
  p.page.loading = false
  if (result.error) log.error('loader failed', { 'url.path': p.url.pathname, 'error.message': result.error.message })
}

function loadData(p, doc, pending) {
  if (!p.route.loader) return Promise.resolve()
  const embedded = p.route.ssr && doc.getElementById(DATA_ID)
  // SSR data is set before the page script runs, so the script sees what the server rendered with.
  if (embedded) {
    const result = loadResult(JSON.parse(embedded.textContent))
    if (!result.redirect) settle(p, result)
    return Promise.resolve().then(() => cur === p && result.redirect && redirect(result.redirect, true))
  }
  p.page.loading = true
  return load(pending ?? fetchData(p.url)).then(result => {
    if (!result) return cur === p && schedule()
    if (!result.redirect) settle(p, result)
    if (cur === p) return result.redirect ? redirect(result.redirect, true) : schedule()
  })
}

export function enter(route, mods, url, doc, pending) {
  useBodies(mods.bodies, !cur)
  Object.assign(comps, mods.comps)
  for (const [name, mod] of Object.entries(mods.stores)) shared[name] ??= store(name, mod)
  if (route.overlays) overlays()
  const p = (cur = { route, url, params: matchPattern(route.pattern, url.pathname) ?? {}, query: Object.fromEntries(url.searchParams), page: { loading: false, error: null } })
  const done = loadData(p, doc, pending)
  p.o = instance(document.querySelector(MAIN), mods.page ?? {}, null)
  whole = true
  flush()
  return done
}
