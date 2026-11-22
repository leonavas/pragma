import { OTEL } from './contract.js'

const SEV = { debug: 5, info: 9, warn: 13, error: 17 }
const BATCH = 50, FLUSH_MS = 2000
const hex = (n) => Array.from(crypto.getRandomValues(new Uint8Array(n)), (b) => (b + 256).toString(16).slice(1)).join('')
const exc = (e) => ({ 'exception.type': e?.name, 'exception.message': e?.message ?? String(e), 'exception.stacktrace': e?.stack })

let queue = []
let endpoint = OTEL
let resource = { 'service.name': 'pragma-client' }
let traceId = hex(16)
let timer

export const newTrace = () => {
  traceId = hex(16)
}

export const traceHeaders = () => ({ traceparent: `00-${traceId}-${hex(8)}-01` })

function emit(level, body, attributes) {
  if (globalThis.__pragmaDev) console[level](body, ...(attributes ? [attributes] : []))
  if (SEV[level] < SEV.info || !endpoint) return
  queue.push({
    timeUnixNano: Date.now() + '000000',
    severityNumber: SEV[level],
    severityText: level.toUpperCase(),
    body: String(body),
    attributes: { 'url.path': location.pathname, ...attributes },
    traceId,
  })
  if (queue.length >= BATCH) flush()
  else timer ||= setTimeout(flush, FLUSH_MS)
}

export const log = {
  debug: (msg, attrs) => emit('debug', msg, attrs),
  info: (msg, attrs) => emit('info', msg, attrs),
  warn: (msg, attrs) => emit('warn', msg, attrs),
  error: (msg, attrs) => emit('error', msg, attrs),
  flush: () => flush(),
}

function flush(beacon) {
  clearTimeout(timer)
  timer = 0
  if (!queue.length || !endpoint) return
  const body = JSON.stringify({ resource, logs: queue })
  queue = []
  if (beacon === true && navigator.sendBeacon?.(endpoint, new Blob([body], { type: 'application/json' }))) return
  fetch(endpoint, { method: 'POST', body, keepalive: true, headers: { 'content-type': 'application/json' } })
    .then((r) => (r.status === 404 || r.status === 405) && (endpoint = null))
    .catch(() => {})
}

export function initClientLog(service) {
  resource = { 'service.name': service }
  addEventListener('error', (e) => emit('error', e.message, exc(e.error ?? e)))
  addEventListener('unhandledrejection', (e) => emit('error', e.reason?.message ?? String(e.reason), exc(e.reason)))
  addEventListener('visibilitychange', () => document.visibilityState === 'hidden' && flush(true))
}
