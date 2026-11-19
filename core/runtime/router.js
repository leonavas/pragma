import { log, traceHeaders, newTrace } from './log.js'
import { enter, leave, importRoute, overlay } from './page.js'
import { fetchData } from './loader.js'

const pages = new Map(), seen = new Set(), positions = new Map()
const noop = () => {}
let manifest, here, key, seq = 0, io

const where = u => u.pathname + u.search
const newKey = () => Math.random().toString(36).slice(2)

export function match(pattern, path) {
  const a = pattern.split('/'), b = path.replace(/(.)\/$/, '$1').split('/'), params = {}
  if (a.length !== b.length) return null
  for (let i = 0; i < a.length; i++) {
    if (a[i][0] === ':') params[a[i].slice(1)] = decodeURIComponent(b[i])
    else if (a[i] !== b[i]) return null
  }
  return params
}

function entry(url) {
  manifest ??= fetch('/_pragma/manifest.json', { headers: traceHeaders() })
    .then(r => r.json())
    .then(m => m.sort((x, y) => x.pattern.split(':').length - y.pattern.split(':').length))
    .catch(e => { manifest = null; throw e })
  return manifest.then(m => m.find(e => match(e.pattern, url.pathname)))
}

function text(u) {
  let p = pages.get(u)
  if (!p) {
    p = fetch(u, { headers: traceHeaders() }).then(r => (r.ok ? r.text() : Promise.reject(new Error(`HTTP ${r.status}`))))
    pages.set(u, p)
    p.catch(() => pages.delete(u))
  }
  return p
}

const link = el => {
  const a = el?.closest?.('a[href]')
  return a && !a.target && !a.hasAttribute('download') && !a.hasAttribute('data-reload') && a.origin === location.origin ? a : null
}

async function prefetch(href) {
  const url = new URL(href)
  if (seen.has(where(url)) || where(url) === here) return
  seen.add(where(url))
  const e = await entry(url).catch(noop)
  if (!e) return
  if (!e.ssr) text(e.html).catch(noop)
  for (const m of e.modules ?? []) import(m).catch(noop)
}

function mark() {
  for (const a of document.querySelectorAll('a[href]')) {
    if (!link(a)) continue
    if (a.pathname === location.pathname && !a.search && !a.hash) a.setAttribute('aria-current', 'page')
    else if (a.getAttribute('aria-current') === 'page') a.removeAttribute('aria-current')
  }
}

function observe() {
  ;(self.requestIdleCallback ?? setTimeout)(() => {
    io?.disconnect()
    io = new IntersectionObserver(es => {
      for (const x of es) if (x.isIntersecting) io.unobserve(x.target), prefetch(x.target.href)
    })
    for (const a of document.querySelectorAll('a[href]')) if (link(a)) io.observe(a)
  })
}

export async function navigate(to, pop) {
  const url = new URL(to, location.href), n = ++seq, t0 = performance.now()
  newTrace()
  let e, doc, route, mods, pending
  try {
    e = await entry(url)
    if (e) {
      pending = e.loader && !e.ssr ? fetchData(url) : undefined
      pending?.catch(noop)
      for (const m of e.modules ?? []) import(m).catch(noop)
      doc = new DOMParser().parseFromString(await (e.ssr ? fetch(url, { headers: traceHeaders() }).then(r => r.text()) : text(e.html)), 'text/html')
      route = JSON.parse(doc.getElementById('pragma-route').textContent)
      mods = await importRoute(route)
    }
  } catch (err) {
    log.error('navigation failed', { 'url.path': url.pathname, 'error.message': err.message })
    e = null
  }
  if (n !== seq) return
  if (!e) return pop ? location.reload() : location.assign(url)

  positions.set(key, scrollY)
  const stacked = overlay?.closeAll() ?? 0
  if (pop) key = history.state?.k
  else history[stacked ? 'replaceState' : 'pushState']({ k: (key = newKey()) }, '', url)
  here = where(url)

  const swap = () => {
    leave()
    const main = document.querySelector('main[data-page]')
    main.replaceChildren(...doc.querySelector('main[data-page]').childNodes)
    document.title = doc.title
    const done = enter(route, mods, url, match(route.pattern, url.pathname) ?? {}, doc, pending)
    const y = pop ? positions.get(key) ?? 0 : 0
    if (!pop && url.hash) document.getElementById(decodeURIComponent(url.hash.slice(1)))?.scrollIntoView()
    else scrollTo(0, y)
    if (y) done.then(() => n === seq && scrollTo(0, y))
    if (!main.hasAttribute('tabindex')) main.tabIndex = -1
    main.focus({ preventScroll: true })
    mark()
  }
  if (document.startViewTransition) await document.startViewTransition(swap).updateCallbackDone
  else swap()
  log.info('navigation', { 'url.path': url.pathname, 'pragma.route': route.pattern, 'pragma.navigation': pop ? 'pop' : 'push', 'duration_ms': Math.round(performance.now() - t0) })
  observe()
}

export function startRouter() {
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
    navigate(a.href)
  })
  addEventListener('popstate', ev => {
    if (overlay?.handlePopState(ev) || where(location) === here) return
    navigate(location.href, true)
  })
  const hover = ev => {
    const a = link(ev.target)
    if (a) prefetch(a.href)
  }
  document.addEventListener('pointerenter', hover, true)
  document.addEventListener('focusin', hover)
  observe()
}
