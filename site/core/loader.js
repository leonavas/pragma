import { log, traceHeaders } from './log.js'

let leaving = false
addEventListener('beforeunload', () => { leaving = true })
addEventListener('pagehide', () => { leaving = true })
addEventListener('pageshow', () => { leaving = false })

const headers = extra => ({ accept: 'application/json', ...traceHeaders(), ...extra })

export const fetchData = url => fetch('/_data' + url.pathname + url.search, { headers: headers() })

export async function load(p, doc, pending) {
  if (!p.route.loader) return
  const el = p.route.ssr && doc.getElementById('pragma-data')
  let r
  if (el) r = JSON.parse(el.textContent)
  else {
    p.page.loading = true
    try {
      const res = await (pending ?? fetchData(p.url))
      r = await res.json().catch(() => ({}))
      if (!res.ok && !r.error) r = { error: { message: `Failed to load data (HTTP ${res.status})` } }
    } catch (e) {
      if (leaving || e.name === 'AbortError') return
      r = { error: { message: e.message } }
    }
  }
  p.data = r.data
  p.page.error = r.error ?? null
  p.page.loading = false
  if (r.error) log.error('loader failed', { 'url.path': p.url.pathname, 'error.message': r.error.message })
}

export async function call(url, name, input) {
  const res = await fetch(`/_action${url.pathname.replace(/\/$/, '')}/${name}`, {
    method: 'POST',
    headers: headers({ 'content-type': 'application/json' }),
    body: JSON.stringify(input ?? null),
  })
  const r = await res.json().catch(() => null)
  if (!res.ok) throw new Error(r?.error?.message ?? `Action "${name}" failed (HTTP ${res.status})`)
  return r
}
