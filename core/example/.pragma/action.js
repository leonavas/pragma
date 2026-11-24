import { traceHeaders } from './trace.js'
import { actionUrl } from './contract.js'

// Server actions: loaded on the first call() / stream().
const parse = t => { try { return JSON.parse(t) } catch { return t || null } }
const isObject = v => v !== null && typeof v === 'object'
const redirectOf = r => (isObject(r) && Object.keys(r).length === 1 && typeof r.redirect?.location === 'string' ? r.redirect.location : null)

export class ActionError extends Error {
  constructor(name, status, body) {
    super(typeof body?.message === 'string' ? body.message : typeof body === 'string' && body ? body : `Action "${name}" failed (HTTP ${status})`)
    this.name = 'ActionError'
    this.status = status
    this.body = body
  }
}

const post = (url, name, input, accept, signal) => fetch(actionUrl(url, name), {
  method: 'POST',
  headers: { accept, 'content-type': 'application/json', ...traceHeaders() },
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
