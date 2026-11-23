import { log, traceHeaders, newTrace } from './log.js'
import { fetchData } from './loader.js'
import { MAIN, MANIFEST, ROUTE_CSS, ROUTE_ID, SHARED_CSS, matchRoute, pageUrl, sortRoutes } from './contract.js'

const MAX_HOPS = 20
const pages = new Map(), seen = new Set(), positions = new Map()
const noop = () => {}
let page, manifest, here, key, seq = 0, io, mo, hops = 0

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

const own = el => (el.localName === 'meta' && el.name !== 'viewport' && !el.hasAttribute('charset')) || (el.localName === 'link' && el.rel !== 'stylesheet' && el.rel !== 'modulepreload')

function syncHead(doc) {
  document.title = doc.title
  const h = document.head
  const old = [...h.children].filter(own), next = [...doc.head.children].filter(own)
  if (old.length === next.length && old.every((el, i) => el.isEqualNode(next[i]))) return
  const at = h.querySelector(`link[rel=stylesheet], [${SHARED_CSS}], [${ROUTE_CSS}]`)
  for (const el of old) el.remove()
  for (const el of next) h.insertBefore(document.importNode(el, true), at)
}

function loadRouteCss(href) {
  const h = document.head, target = new URL(href, location.href).href
  const existing = [...h.querySelectorAll(`link[${ROUTE_CSS}]`)].find(el => el.href === target)
  if (existing) return { el: existing, created: false, ready: Promise.resolve() }
  const el = document.createElement('link')
  el.rel = 'stylesheet'
  el.href = href
  el.setAttribute(ROUTE_CSS, '')
  const ready = new Promise(resolve => { el.onload = el.onerror = resolve })
  const sheets = h.querySelectorAll('link[rel=stylesheet]')
  sheets.length ? sheets[sheets.length - 1].after(el) : h.append(el)
  return { el, created: true, ready }
}

function syncRouteCss(keep, nextStyle) {
  const h = document.head
  for (const el of h.querySelectorAll(`[${ROUTE_CSS}]`)) if (el !== keep) el.remove()
  if (nextStyle) {
    const style = document.importNode(nextStyle, true)
    const anchor = h.querySelector(`[${SHARED_CSS}], link[rel=stylesheet]`)
    anchor ? anchor.after(style) : h.append(style)
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
  if (!e) return
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

function watchAdded(records) {
  for (const r of records) {
    if (r.type === 'attributes') watch(r.target)
    else for (const n of r.addedNodes) if (n.nodeType === 1) {
      if (n.localName === 'a') watch(n)
      for (const a of n.querySelectorAll('a[href]')) watch(a)
    }
  }
}

function observe() {
  if (navigator.connection?.saveData) return
  ;(self.requestIdleCallback ?? setTimeout)(() => {
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
  let e, doc, route, mods, pending, css, moved
  try {
    e = await entry(url)
    if (e) {
      pending = e.loader && !e.ssr ? fetchData(url) : undefined
      pending?.catch(noop)
      for (const m of e.modules ?? []) import(m).catch(noop)
      doc = new DOMParser().parseFromString(await (e.ssr ? html(url, true) : text(pageUrl(e.html))), 'text/html')
      route = JSON.parse(doc.getElementById(ROUTE_ID).textContent)
      const nextCss = doc.head.querySelector(`link[rel=stylesheet][${ROUTE_CSS}]`)?.getAttribute('href')
      if (nextCss) css = loadRouteCss(nextCss)
      mods = await page.importRoute(route)
      if (css) await css.ready
    }
  } catch (err) {
    if (err.moved) moved = err.moved
    else log.error('navigation failed', { 'url.path': url.pathname, 'error.message': err.message })
    e = null
  }
  if (n !== seq || !e) {
    if (css?.created) css.el.remove()
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
    page.leave()
    const main = document.querySelector(MAIN), next = doc.querySelector(MAIN)
    main.replaceChildren(...next.childNodes)
    main.dataset.page = next.dataset.page
    syncHead(doc)
    syncRouteCss(css?.el ?? null, doc.head.querySelector(`style[${ROUTE_CSS}]`))
    const done = page.enter(route, mods, url, doc, pending)
    const y = pop ? positions.get(key) ?? 0 : 0
    if (!pop && url.hash) document.getElementById(decodeURIComponent(url.hash.slice(1)))?.scrollIntoView()
    else scrollTo(0, y)
    if (y) done.then(() => n === seq && scrollTo(0, y))
    if (!main.hasAttribute('tabindex')) main.tabIndex = -1
    main.focus({ preventScroll: true })
    mark()
  }
  if (document.startViewTransition && !matchMedia('(prefers-reduced-motion: reduce)').matches) await document.startViewTransition(swap).updateCallbackDone
  else swap()
  log.info('navigation', { 'url.path': url.pathname, 'pragma.route': route.pattern, 'pragma.navigation': pop ? 'pop' : replace ? 'replace' : 'push', 'duration_ms': Math.round(performance.now() - t0) })
  observe()
}

export function startRouter(pageApi) {
  page = pageApi
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
  document.readyState === 'complete' ? observe() : addEventListener('load', observe, { once: true })
}
