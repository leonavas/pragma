import { traceHeaders } from './log.js'
import { actionUrl, dataUrl, layoutUrl } from './contract.js'

let leaving = false
addEventListener('beforeunload', () => { leaving = true })
addEventListener('pagehide', () => { leaving = true })
addEventListener('pageshow', () => { leaving = false })

const headers = extra => ({ accept: 'application/json', ...traceHeaders(), ...extra })
const parse = t => { try { return JSON.parse(t) } catch { return t || null } }
const isObject = v => v !== null && typeof v === 'object'
const redirectOf = r => (isObject(r) && Object.keys(r).length === 1 && typeof r.redirect?.location === 'string' ? r.redirect.location : null)

class ActionError extends Error {
  constructor(name, status, body) {
    super(typeof body?.message === 'string' ? body.message : typeof body === 'string' && body ? body : `Action "${name}" failed (HTTP ${status})`)
    this.name = 'ActionError'
    this.status = status
    this.body = body
  }
}

export const fetchData = url => fetch(dataUrl(url), { headers: headers() })
export const fetchLayout = url => fetch(layoutUrl(url), { headers: headers() })

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

const post = (url, name, input, accept, signal) => fetch(actionUrl(url, name), {
  method: 'POST',
  headers: headers({ accept, 'content-type': 'application/json' }),
  body: JSON.stringify(input ?? null),
  signal,
})

async function failure(res, name) {
  const r = parse(await res.text().catch(() => ''))
  return new ActionError(name, res.status, isObject(r) && 'error' in r ? r.error : r)
}

export async function call(url, name, input, follow) {
  const res = await post(url, name, input, 'application/json')
  if (!res.ok) throw await failure(res, name)
  const r = parse(await res.text()), to = redirectOf(r)
  if (to) return void (await follow(to))
  return r
}

export async function* stream(url, name, input, { signal } = {}, follow) {
  const res = await post(url, name, input, 'application/x-ndjson, application/json', signal)
  if (!res.ok) throw await failure(res, name)
  if (!(res.headers.get('content-type') ?? '').includes('ndjson')) {
    const r = parse(await res.text()), to = redirectOf(r)
    if (to) return void (await follow(to))
    if (r != null) yield r
    return
  }
  const reader = res.body.getReader(), text = new TextDecoder()
  let buffer = ''
  try {
    for (;;) {
      const { done, value } = await reader.read()
      buffer += text.decode(value, { stream: !done })
      const lines = buffer.split('\n')
      buffer = done ? '' : lines.pop()
      for (const line of lines) if (line.trim()) yield JSON.parse(line)
      if (done) return
    }
  } finally {
    reader.cancel().catch(() => {})
  }
}
