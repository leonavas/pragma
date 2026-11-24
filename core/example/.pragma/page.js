import { render, patch, resume, sliced, startSlice, spent, hooks, scopeOf, input, eventCall, useBodies, DEV, PAGE } from './directives.js'
import { log } from './log.js'
import { reactive, readers, onChange, persist, persistStores, create } from './state.js'
import { fetchData, fetchLayout, load, loadResult, call, stream } from './loader.js'
import { DATA_ID, MAIN, PERSIST_SUFFIX, matchPattern, coerce, instantiate, isPlain, readonlyViews } from './contract.js'

// The build sets `overlays` in the route on pages with a dialog or popover; enter() then loads overlay.js (not modulepreloaded).
let overlayModule = null, overlayLoad = null
const overlays = () => (overlayLoad ??= import('./overlay.js').then(m => (m.initOverlays(), (overlayModule = m))))
const open = (...a) => overlays().then(m => m.open(...a))
const close = (...a) => overlays().then(m => m.close(...a))
const confirm = (...a) => overlays().then(m => m.confirm(...a))
export const closeOverlays = () => overlayModule?.closeAll() ?? 0
export const popOverlay = () => overlayModule?.handlePopState() ?? false

// The router is not needed for first paint: boot.js loads it right after the page mounts.
let routerLoad = null
export const router = () => (routerLoad ??= import('./router.js'))
const navigate = (to, options) => router().then(r => r.navigate(to, options))
const redirect = (to, replace) => router().then(r => r.redirect(to, replace))

const C = Symbol(), O = Symbol(), mounted = new Set(), comps = {}, types = new Set()
const shared = {}
let cur, queued, whole, slices = 0, mounts = [], layout = null, layoutSeq = 0
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
  render(document.body, { __proto__: null, shared, layout: reactive(layout), params: cur.params, query: cur.query, page: cur.page })
  persistStores()
  runMounts()
  later()
}

// `mount` calls share the slice budget with the render: once it is spent, at least one more runs and
// the rest wait for the next task, in order.
function runMounts() {
  let ran = false
  while (mounts.length) {
    if (ran && spent()) return
    const o = mounts.shift()
    if (!mounted.has(o)) continue
    ran = true
    try { cleanup(o, o.mount?.(o.el)) } catch (e) { reportError(e) }
  }
}

// Work left by a slice (list items, mounts) continues in a background-priority task, after the browser
// could paint and answer input. A state change runs a full render, which starts the slices over.
const task = self.scheduler?.postTask ? fn => scheduler.postTask(fn, { priority: 'background' }) : fn => setTimeout(fn)

function later() {
  if ((sliced() || mounts.length) && !slices) slices = 1, task(nextSlice)
}

function nextSlice() {
  slices = 0
  if (!cur) return
  startSlice()
  runMounts()
  if (sliced() && !spent()) {
    resume()
    runMounts()
  }
  later()
}

// The overlay manager calls this right before the first open: the content renders, then it shows.
hooks.reveal = () => {
  whole = true
  flush()
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
  get layout() { return reactive(layout) },
  get params() { return cur?.params },
  get query() { return cur?.query },
  get page() { return cur?.page },
  call: (name, data) => call(cur.url, name, data, navigate).then(r => (refreshLayout(), r)),
  stream: (name, data, options) => refreshAfter(stream(cur.url, name, data, options, navigate)),
}

// `layout` comes from routes/_layout.server.js: on the full page load, then again after each action.
export function loadLayout(pending, doc) {
  const n = ++layoutSeq
  if (!pending) {
    const embedded = doc.getElementById(DATA_ID)
    layout = embedded ? JSON.parse(embedded.textContent).layout ?? null : layout
    return
  }
  return load(pending).then(result => {
    if (!result || n !== layoutSeq) return
    if (result.redirect) return redirect(result.redirect, true)
    if (result.error) return log.error('layout load failed', { 'error.message': result.error.message })
    layout = result.layout ?? null
    schedule()
  })
}

const refreshLayout = () => cur?.route.layout && loadLayout(fetchLayout(cur.url))

async function* refreshAfter(lines) {
  try { yield* lines } finally { refreshLayout() }
}

function instance(el, mod, w) {
  const target = {}, o = { el, w, target }
  o.scope = reactive(target)
  o.mount = create(target, () => instantiate(mod, target, o.scope, services, (k, d, name) => {
    const v = coerce(o.w && Object.hasOwn(o.w, k) ? o.w[k] : el.getAttribute('data-prop-' + k) ?? undefined, d)
    return DEV ? readonly(v, name) : v
  }))
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
  if (route.overlays && !overlayLoad) {
    // overlay.js is not modulepreloaded: a click on an @open button before it arrives runs again once it is in.
    const early = ev => !overlayModule && ev.target.closest?.('[data-open]') && overlays().then(() => ev.target.click())
    document.addEventListener('click', early)
    overlays().then(() => document.removeEventListener('click', early))
  }
  const p = (cur = { route, url, params: matchPattern(route.pattern, url.pathname) ?? {}, query: Object.fromEntries(url.searchParams), page: { loading: false, error: null } })
  const done = loadData(p, doc, pending)
  p.o = instance(document.querySelector(MAIN), mods.page ?? {}, null)
  whole = true
  flush()
  return done
}
