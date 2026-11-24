import { traceHeaders, newTrace } from './trace.js'
import { fetchData } from './net.js'
import { MAIN, MANIFEST, ROUTE_CSS, SHARED_CSS, matchRoute, pageUrl, sortRoutes } from './contract.js'

// Loaded by page.js in idle after the first mount (or on the first navigate/redirect). A click on a
// link before that is a normal document navigation. A route without JS (manifest `entry: null`)
// is always a document navigation.

const MAX_HOPS = 20
const pages = new Map(), seen = new Set(), positions = new Map()
const noop = () => {}
let page, log, manifest, here, key, seq = 0, io, mo, hops = 0

const where = u => u.pathname + u.search
const newKey = () => Math.random().toString(36).slice(2)

function entry(url) {
  manifest ??= fetch(MANIFEST, { headers: traceHeaders() })
    .then(r => r.json())
    .then(sortRoutes)
    .catch(e => { manifest = null; throw e })
  return manifest.then(table => matchRoute(table, url.pathname)?.route)
}

// The assets binding redirects a raw /a/[id].html to the encoded /a/%5Bid%5D: decoded, both are the same page.
const pagePath = u => decodeURI(new URL(u, location.href).pathname).replace(/(\/index)?\.html$/, '').replace(/\/$/, '')

function html(u, anyStatus = false) {
  return fetch(u, { headers: traceHeaders() }).then(r => {
    if (r.redirected && pagePath(r.url) !== pagePath(u)) return Promise.reject(Object.assign(new Error(`redirected to ${r.url}`), { moved: r.url }))
    return r.ok || anyStatus ? r.text() : Promise.reject(new Error(`HTTP ${r.status}`))
  })
}

function text(u) {
  let p = pages.get(u)
  if (!p) {
    p = html(u)
    pages.set(u, p)
    p.catch(() => pages.delete(u))
  }
  return p
}

const own = el => (el.localName === 'meta' && el.name !== 'viewport' && !el.hasAttribute('charset')) || (el.localName === 'link' && el.rel !== 'stylesheet' && el.rel !== 'modulepreload' && !el.hasAttribute(ROUTE_CSS))

function syncHead(doc) {
  document.title = doc.title
  const h = document.head
  const old = [...h.children].filter(own), next = [...doc.head.children].filter(own)
  if (old.length === next.length && old.every((el, i) => el.isEqualNode(next[i]))) return
  const at = h.querySelector(`link[rel=stylesheet], [${SHARED_CSS}], [${ROUTE_CSS}]`)
  for (const el of old) el.remove()
  for (const el of next) h.insertBefore(document.importNode(el, true), at)
}

// Stage the destination in source order. Its paint styles must be ready at the swap;
// the preloaded block sheet can finish later, under the destination kernel's clone gate.
function stageRouteCss(doc) {
  const head = document.head, staged = [], blocking = []
  let after = [...head.querySelectorAll(`link[rel=stylesheet], [${SHARED_CSS}], [${ROUTE_CSS}]`)].at(-1)
  for (const source of doc.head.querySelectorAll(`[${ROUTE_CSS}]`)) {
    const el = document.importNode(source, true)
    el.setAttribute('data-pragma-pending', '')
    el.media = 'not all'
    if (el.localName === 'link') {
      el.rel = 'stylesheet'
      if (el.hasAttribute('data-pragma-blocks')) {
        el.onload = el.onerror = e => { el.dataset.pragmaCssStatus = e.type }
      } else {
        blocking.push(new Promise(resolve => { el.onload = el.onerror = resolve }))
      }
    }
    after ? after.after(el) : head.append(el)
    after = el
    staged.push(el)
  }
  return {
    ready: Promise.all(blocking),
    discard() { for (const el of staged) el.remove() },
    activate() {
      for (const el of head.querySelectorAll(`[${ROUTE_CSS}]:not([data-pragma-pending])`)) el.remove()
      for (const el of staged) {
        el.removeAttribute('data-pragma-pending')
        el.media = 'all'
      }
    },
  }
}

const link = el => {
  const a = el?.closest?.('a[href]')
  return a && !a.target && !a.hasAttribute('download') && !a.hasAttribute('data-reload') && a.origin === location.origin ? a : null
}

// modulepreload downloads and compiles without running: a route module runs only on navigation.
function preload(m) {
  const href = new URL(m, location.href).href
  if ([...document.querySelectorAll('link[rel=modulepreload]')].some(el => el.href === href)) return
  const el = document.createElement('link')
  el.rel = 'modulepreload'
  el.href = href
  document.head.append(el)
}

async function prefetch(href) {
  const url = new URL(href)
  if (seen.has(where(url)) || where(url) === here) return
  seen.add(where(url))
  const e = await entry(url).catch(noop)
  if (!e?.entry) return
  if (!e.ssr) text(pageUrl(e.html)).catch(noop)
  for (const m of e.modules ?? []) preload(m)
}

function mark(path = location.pathname) {
  for (const a of document.querySelectorAll('a[href]')) {
    if (!link(a)) continue
    if (a.pathname === path && !a.search && !a.hash) a.setAttribute('aria-current', 'page')
    else if (a.getAttribute('aria-current') === 'page') a.removeAttribute('aria-current')
  }
}

const watch = el => {
  const a = link(el)
  if (a) io.observe(a)
}

// Added nodes are collected and scanned once when the browser is idle, not inside each render slice;
// a root inside another collected root is not scanned twice.
const idle = self.requestIdleCallback ?? setTimeout
let added = new Set(), scanning = false

function scan() {
  scanning = false
  const roots = added
  added = new Set()
  const covered = n => {
    for (let p = n.parentElement; p; p = p.parentElement) if (roots.has(p)) return true
    return false
  }
  for (const n of roots) {
    if (!n.isConnected || covered(n)) continue
    if (n.localName === 'a') watch(n)
    for (const a of n.querySelectorAll('a[href]')) watch(a)
  }
}

function watchAdded(records) {
  for (const r of records) {
    if (r.type === 'attributes') added.add(r.target)
    else for (const n of r.addedNodes) if (n.nodeType === 1) added.add(n)
  }
  if (added.size && !scanning) scanning = true, idle(scan)
}

function observe() {
  if (navigator.connection?.saveData) return
  idle(() => {
    io?.disconnect()
    io = new IntersectionObserver(es => {
      for (const x of es) if (x.isIntersecting) io.unobserve(x.target), prefetch(x.target.href)
    })
    for (const a of document.querySelectorAll('a[href]')) watch(a)
    if (!mo) {
      mo = new MutationObserver(watchAdded)
      mo.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['href'] })
    }
  })
}

export function navigate(to, { replace = false } = {}) {
  hops = 0
  return redirect(to, replace)
}

export function redirect(to, replace = false) {
  const url = new URL(to, location.href)
  if (url.origin === location.origin && ++hops <= MAX_HOPS) return go(url, false, replace)
  location[replace ? 'replace' : 'assign'](url)
}

async function go(to, pop, replace) {
  const url = new URL(to, location.href), n = ++seq, t0 = performance.now()
  newTrace()
  let e, doc, route, pending, css, moved
  try {
    e = await entry(url)
    if (e?.entry) {
      pending = e.loader && !e.ssr ? fetchData(url) : undefined
      pending?.catch(noop)
      for (const m of e.modules ?? []) preload(m)
      const R = import(e.entry)
      R.catch(noop)
      doc = new DOMParser().parseFromString(await (e.ssr ? html(url, true) : text(pageUrl(e.html))), 'text/html')
      css = stageRouteCss(doc)
      // Additive: the entry installs what this route needs into the live runtime (again: a no-op).
      route = (await R).default
      page.install(route.use)
      await css.ready
    } else e = null
  } catch (err) {
    if (err.moved) moved = err.moved
    else log.error('navigation failed', { 'url.path': url.pathname, 'error.message': err.message })
    e = null
  }
  if (n !== seq || !e) {
    css?.discard()
    if (n !== seq) return
    if (moved) return redirect(moved, pop || replace)
    return pop ? location.reload() : location[replace ? 'replace' : 'assign'](url)
  }

  positions.set(key, scrollY)
  const stacked = page.closeOverlays()
  if (pop) key = history.state?.k
  else history[stacked || replace ? 'replaceState' : 'pushState']({ k: (key = newKey()) }, '', url)
  here = where(url)

  const swap = () => {
    if (n !== seq) {
      css?.discard()
      return
    }
    page.leave()
    const main = document.querySelector(MAIN), next = doc.querySelector(MAIN)
    main.replaceChildren(...next.childNodes)
    main.dataset.page = next.dataset.page
    syncHead(doc)
    css.activate()
    const done = page.enter(route, url, doc, pending)
    const y = pop ? positions.get(key) ?? 0 : 0
    if (!pop && url.hash) document.getElementById(decodeURIComponent(url.hash.slice(1)))?.scrollIntoView()
    else scrollTo(0, y)
    if (y) done.then(() => n === seq && scrollTo(0, y))
    if (!main.hasAttribute('tabindex')) main.tabIndex = -1
    main.focus({ preventScroll: true })
    mark()
  }
  if (document.startViewTransition && !matchMedia('(prefers-reduced-motion: reduce)').matches) await transition(swap)
  else swap()
  log.info('navigation', { 'url.path': url.pathname, 'pragma.route': route.pattern, 'pragma.navigation': pop ? 'pop' : replace ? 'replace' : 'push', 'duration_ms': Math.round(performance.now() - t0) })
  observe()
}

// `view-transition-name` makes <main> a stacking context and a backdrop root (a `backdrop-filter`
// inside it stops blurring), so the page has its name only while a transition runs. A newer
// transition skips the older one; only the newest removes the attribute.
let active = null

function transition(swap) {
  const html = document.documentElement
  html.setAttribute('data-view-transition', '')
  const vt = (active = document.startViewTransition(swap))
  vt.finished.finally(() => {
    if (active !== vt) return
    active = null
    html.removeAttribute('data-view-transition')
  })
  return vt.updateCallbackDone
}

export function startRouter(pageApi) {
  page = pageApi
  log = page.log
  history.scrollRestoration = 'manual'
  key = history.state?.k ?? newKey()
  history.replaceState({ ...history.state, k: key }, '')
  here = where(location)
  mark()

  addEventListener('click', ev => {
    const a = link(ev.target)
    if (!a || ev.defaultPrevented || ev.button || ev.metaKey || ev.ctrlKey || ev.shiftKey || ev.altKey) return
    if (a.hash && where(a) === here) return
    ev.preventDefault()
    mark(a.pathname)
    navigate(a.href)
  })
  addEventListener('popstate', () => {
    if (page.popOverlay() || where(location) === here) return
    go(location.href, true)
  })
  const hover = ev => {
    const a = link(ev.target)
    if (a) prefetch(a.href)
  }
  document.addEventListener('pointerenter', hover, true)
  document.addEventListener('focusin', hover)
  // Viewport prefetch waits for the page load and the first /_data and /_layout: it must not compete with them.
  const loaded = new Promise(r => (document.readyState === 'complete' ? r() : addEventListener('load', r, { once: true })))
  Promise.all([loaded, page.loaded]).then(observe)
}
