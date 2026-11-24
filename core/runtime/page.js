import { H, K, D, S as SCOPE, DEV, render, update, useTables, get, ctx } from './dom.js'
import { reactive, readers, create, S } from './state.js'
import { MAIN, coerce, isPlain, matchPattern, readonlyViews } from './contract.js'

export { K }

// Per-use modules (R.use) install their hooks once; nothing is ever uninstalled.
const installed = new WeakSet()
export function install(use = []) {
  for (const f of use) if (!installed.has(f)) installed.add(f), f(K)
}

// Lazy modules: loaded on first use (or in idle after the first mount), never modulepreloaded.
// A failed load (offline, or the document unloading while it is in flight) is forgotten, so the
// next call tries again.
const once = f => {
  let p
  return () => (p ??= f().catch(e => { p = null; throw e }))
}
// Loads started by the runtime itself (idle, enter, the log queue) have no caller to reject to.
const quiet = load => () => load().catch(() => {})
// A service whose module is not loaded yet: the call loads it, then forwards.
const later = (load, names, pick = m => m) => Object.fromEntries(names.map(k => [k, (...a) => load().then(m => pick(m)[k](...a))]))

let loaded
const router = once(() => import('./router.js').then(m => (m.startRouter(api), m)))
const overlays = once(() => import('./overlay.js').then(m => (m.initOverlays(), (K.overlay = m), schedule(), m)))
const actions = once(() => import('./action.js'))
const { navigate, redirect } = later(router, ['navigate', 'redirect'])
const { open, close, confirm } = later(overlays, ['open', 'close', 'confirm'])

// `log` is a queue until log.js arrives; initClientLog replays it.
const queue = []
let real = null
const logs = once(() => import('./log.js').then(m => (m.initClientLog(location.hostname, queue), (real = m.log))))
const log = { flush: () => real?.flush() }
for (const l of ['debug', 'info', 'warn', 'error']) log[l] = (msg, attrs) => (real ? real[l](msg, attrs) : (queue.push([l, msg, attrs]), quiet(logs)(), undefined))

const api = { install, enter, leave, log, closeOverlays: () => K.overlay?.closeAll() ?? 0, popOverlay: () => K.overlay?.handlePopState() ?? false, get loaded() { return loaded } }

const C = Symbol(), O = Symbol(), mounted = new Set(), comps = {}, types = new Set(), shared = {}, dirty = new Set()
let cur, queued, whole, mounts = [], layout = null, slicing = 0

Object.defineProperties(K, Object.getOwnPropertyDescriptors({
  log, redirect, schedule,
  get cur() { return cur },
  set layout(v) { layout = v },
}))

// schedule(target, key): the readers of that key are dirty (the disconnected ones are pruned).
// schedule(): one whole render (enter, loader data, layout, overlay.js, reveal, storage).
export function schedule(t, k) {
  if (t === undefined) whole = 1
  else {
    const set = readers(t, k)
    if (!set) return
    for (const r of set) (r.el ?? r[0]).isConnected ? dirty.add(r) : set.delete(r)
    if (!dirty.size) return
  }
  if (!queued) queued = 1, queueMicrotask(flush)
}
S.n = schedule

const now = () => performance.now()
const SLICE_MS = 12

function flush() {
  queued = 0
  if (!cur) return
  ctx.end = now() + SLICE_MS
  if (whole) {
    whole = 0
    dirty.clear()
    render(document.body, { __proto__: null, shared, layout: reactive(layout), params: cur.params, query: cur.query, page: cur.page })
  } else {
    const rs = [...dirty]
    dirty.clear()
    update(rs)
  }
  runMounts()
  H.flushed()
  next()
}
H.reveal = () => {
  whole = 1
  flush()
}

// `mount` calls share the slice budget with the render: once it is spent, at least one more runs
// and the rest wait for the next task, in order.
function runMounts() {
  let ran = false
  while (mounts.length) {
    if (ran && now() > ctx.end) return
    const o = mounts.shift()
    if (!mounted.has(o)) continue
    ran = true
    try { off(o, o.mount?.(o.el)) } catch (e) { reportError(e) }
  }
}

// Work left by a slice (list items, mounts) continues in a background task, after the browser
// could paint and answer input.
const task = self.scheduler?.postTask ? fn => scheduler.postTask(fn, { priority: 'background' }) : fn => setTimeout(fn)
function next() {
  if ((K.slices?.pending() || mounts.length) && !slicing) slicing = 1, task(slice)
}
function slice() {
  slicing = 0
  if (!cur) return
  ctx.end = now() + SLICE_MS
  runMounts()
  if (K.slices?.pending() && now() <= ctx.end) {
    K.slices.resume()
    runMounts()
  }
  next()
}

function off(o, f) {
  if (typeof f?.then !== 'function') return void (o.off = f)
  f.then(f => {
    if (typeof f !== 'function') return
    if (mounted.has(o)) o.off = f
    else try { f() } catch (e) { reportError(e) }
  }, reportError)
}

let frames = [], frameReq = 0
function frame(fn) {
  frames.push(fn)
  frameReq ||= requestAnimationFrame(t => {
    frameReq = 0
    const f = frames
    frames = []
    for (const fn of f) try { fn(t) } catch (e) { reportError(e) }
  })
  return () => {
    const i = frames.indexOf(fn)
    if (i >= 0) frames.splice(i, 1)
  }
}

const refresh = () => K.net?.refreshLayout()
async function* stream(name, input, options) {
  const m = await actions()
  try { yield* m.stream(cur.url, name, input, options, navigate) } finally { refresh() }
}

const services = {
  shared, log, navigate, open, close, confirm, frame, stream,
  get data() { return reactive(cur?.data) },
  get layout() { return reactive(layout) },
  get params() { return cur?.params },
  get query() { return cur?.query },
  get page() { return cur?.page },
  call: (name, input) => actions().then(m => m.call(cur.url, name, input, navigate)).then(r => (refresh(), r)),
}

const readonly = readonlyViews(isPlain, name => `pragma: "${name}" is a prop: props are read-only, also inside; copy what you change into a let (\`let rows = ${name}.map(row => ({ ...row }))\`)`)

// A component (or the page) instance: its script runs once with `state` (reactive) and services.
function instance(el, factory, w) {
  const target = {}, o = { el, w, target, scope: reactive(target) }
  const prop = (name, d) => {
    const k = name.toLowerCase()
    Object.defineProperty(target, name, {
      enumerable: true,
      get: () => {
        const v = coerce(o.w && Object.hasOwn(o.w, k) ? o.w[k] : el[D]?.p?.[k], d)
        return DEV ? readonly(v, name) : v
      },
    })
  }
  const a = create(target, () => factory?.(o.scope, { __proto__: services, prop })) ?? {}
  for (const k in a) if (k !== 'mount') target[k] = a[k]
  o.mount = a.mount
  mounted.add(o)
  mounts.push(o)
  return (el[C] = o)
}

function store(name, [factory, keys]) {
  const target = {}, scope = reactive(target)
  const a = create(target, () => factory?.(scope, services)) ?? {}
  for (const k in a) target[k] = a[k]
  if (keys?.length) K.persist ? K.persist(target, name, keys) : DEV && fail('persist')
  return scope
}
const fail = what => { throw new Error(`pragma: ${what} needs ${what}.js (build bug)`) }

function unmount(o) {
  mounted.delete(o)
  try { o.off?.() } catch (e) { reportError(e) }
}

H.on = t => types.has(t) || (types.add(t), document.addEventListener(t, onEvent, true))
H.remove = n => { for (const o of mounted) if (n.contains(o.el)) unmount(o) }
H.enter = (el, s, d, w = null) => {
  if (d.x === 2) return { __proto__: s, data: reactive(cur.data), state: cur.o.scope, [O]: cur.o }
  const f = comps[d.c]
  if (!f) return s
  const o = el[C] ?? instance(el, f, w)
  o.w = w
  return { __proto__: s, state: o.scope, [O]: o }
}

// Delegation: the first element from the target up whose descriptor handles this event type.
// Writes in the handler go through the Proxy and schedule their own readers.
function onEvent(e) {
  if (e.type === 'input') H.input(e)
  for (let el = e.target; el?.nodeType === 1; el = el.parentElement) {
    const h = el[D]?.e?.[e.type]
    if (h) return fire(el, h, e)
  }
}

function fire(el, [p, args], e) {
  const s = el[SCOPE], o = s?.[O] ?? cur.o
  let fn, self
  if (p.length === 1) fn = o.target[p[0]]
  else self = s && get(s, p.slice(0, -1)), (fn = self?.[p.at(-1)])
  if (typeof fn !== 'function') {
    if (DEV) throw new Error(`pragma: @${e.type}="${p.join('.')}" needs a function, but it holds ${fn === null ? 'null' : typeof fn}: use a function or a const holding one (\`const saveLater = debounce(save, 300)\`)`)
    return
  }
  if (e.type === 'submit') e.preventDefault()
  fn.apply(self, args.map(a => (a === 0 ? e : a === 1 ? el : Array.isArray(a) ? s && get(s, a) : a.v)))
}

export function leave() {
  K.cssBlocks = null
  K.cssMiss = false
  if (cur) for (const o of mounted) if (cur.o.el.contains(o.el)) unmount(o)
}

// Runs a route on the live document: boot (from start) or SPA navigation (from the router, after
// it installed R.use).
export function enter(R, url, doc, pending) {
  R.applyBlockCss?.(K, schedule)
  useTables(R, !cur)
  Object.assign(comps, R.comps)
  for (const n in R.stores) shared[n] ??= store(n, R.stores[n])
  if (R.overlays) quiet(overlays)()
  const p = (cur = { route: R, url, params: matchPattern(R.pattern, url.pathname) ?? {}, query: Object.fromEntries(url.searchParams), page: reactive({ loading: false, error: null }) })
  const done = R.loader ? K.net.loadData(p, doc, pending) : Promise.resolve()
  p.o = instance(document.querySelector(MAIN), R.page, null)
  whole = 1
  flush()
  return done
}

let booted = 0
const idle = self.requestIdleCallback ?? setTimeout

export function start(R, applyBlockCss) {
  if (applyBlockCss) R.applyBlockCss = applyBlockCss
  install(R.use)
  if (booted) return
  booted = 1
  const url = new URL(location.href), net = K.net
  const pending = R.loader && !R.ssr ? self.__pd ?? net.fetchData(url) : undefined
  const layoutDone = R.layout ? net.loadLayout(R.ssr ? null : self.__pl ?? net.fetchLayout(url), document) : null
  self.__pd = self.__pl = null
  loaded = Promise.all([enter(R, url, document, pending), layoutDone]).catch(() => {})
  // After the first mount: logging, then the router (links, prefetch, back/forward).
  idle(() => {
    quiet(logs)()
    quiet(router)()
  })
}
