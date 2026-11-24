import { traceHeaders } from './trace.js'
import { DATA_ID, dataUrl, layoutUrl } from './contract.js'

// Loader data (/_data) and layout data (/_layout): for a route with `load`, or any reactive route
// when routes/_layout.server.js exists.
let leaving = false, K, layoutSeq = 0

const parse = t => { try { return JSON.parse(t) } catch { return t || null } }
const isObject = v => v !== null && typeof v === 'object'
const redirectOf = r => (isObject(r) && Object.keys(r).length === 1 && typeof r.redirect?.location === 'string' ? r.redirect.location : null)

const get = u => fetch(u, { headers: { accept: 'application/json', ...traceHeaders() } })
export const fetchData = url => get(dataUrl(url))
export const fetchLayout = url => get(layoutUrl(url))

const problem = (body, status) => ({ ...(isObject(body) ? body : { message: body == null ? `Failed to load data (HTTP ${status})` : String(body) }), ...(status && { status }) })

export function loadResult(r) {
  const to = redirectOf(r)
  if (to) return { redirect: to }
  r = isObject(r) ? r : {}
  return { data: r.data, layout: r.layout, error: r.error ? problem(r.error, r.error.status ?? 500) : null }
}

export async function load(response) {
  let r
  try {
    const res = await response
    r = parse(await res.text())
    if (!res.ok) r = { error: problem(isObject(r) && 'error' in r ? r.error : null, res.status) }
  } catch (e) {
    if (leaving || e.name === 'AbortError') return null
    r = { error: { message: e.message } }
  }
  return loadResult(r)
}

function settle(p, result) {
  p.data = result.data
  p.page.error = result.error
  p.page.loading = false
  if (result.error) K.log.error('loader failed', { 'url.path': p.url.pathname, 'error.message': result.error.message })
}

// SSR data is set before the page script runs, so the script sees what the server rendered with.
function loadData(p, doc, pending) {
  const embedded = p.route.ssr && doc.getElementById(DATA_ID)
  if (embedded) {
    const result = loadResult(JSON.parse(embedded.textContent))
    if (!result.redirect) settle(p, result)
    return Promise.resolve().then(() => K.cur === p && result.redirect && K.redirect(result.redirect, true))
  }
  p.page.loading = true
  return load(pending ?? fetchData(p.url)).then(result => {
    if (!result) return K.cur === p && K.schedule()
    if (!result.redirect) settle(p, result)
    if (K.cur === p) return result.redirect ? K.redirect(result.redirect, true) : K.schedule()
  })
}

// `layout` comes from routes/_layout.server.js: on the full page load, then again after each action.
function loadLayout(pending, doc) {
  const n = ++layoutSeq
  if (!pending) {
    const embedded = doc.getElementById(DATA_ID)
    if (embedded) K.layout = JSON.parse(embedded.textContent).layout ?? null
    return
  }
  return load(pending).then(result => {
    if (!result || n !== layoutSeq) return
    if (result.redirect) return K.redirect(result.redirect, true)
    if (result.error) return K.log.error('layout load failed', { 'error.message': result.error.message })
    K.layout = result.layout ?? null
    K.schedule()
  })
}

const refreshLayout = () => K.cur?.route.layout && loadLayout(fetchLayout(K.cur.url))

export default function install(k) {
  K = k
  addEventListener('beforeunload', () => { leaving = true })
  addEventListener('pagehide', () => { leaving = true })
  addEventListener('pageshow', () => { leaving = false })
  K.net = { loadData, loadLayout, fetchData, fetchLayout, refreshLayout }
}
