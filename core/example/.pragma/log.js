import { OTEL } from './contract.js'
import { traceId } from './trace.js'

export { traceHeaders, newTrace } from './trace.js'

const SEV = { debug: 5, info: 9, warn: 13, error: 17 }
const BATCH = 50, FLUSH_MS = 2000
const exc = (e) => ({ 'exception.type': e?.name, 'exception.message': e?.message ?? String(e), 'exception.stacktrace': e?.stack })

let queue = []
let endpoint = OTEL
let resource = { 'service.name': 'pragma-client' }
let timer

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

// Loaded in idle after the first mount (or on the first log call): `queued` holds the
// [level, msg, attrs] the kernel's stub stored until then.
export function initClientLog(service, queued = []) {
  resource = { 'service.name': service }
  for (const [level, msg, attrs] of queued.splice(0)) emit(level, msg, attrs)
  addEventListener('error', (e) => emit('error', e.message, exc(e.error ?? e)))
  addEventListener('unhandledrejection', (e) => emit('error', e.reason?.message ?? String(e.reason), exc(e.reason)))
  addEventListener('visibilitychange', () => document.visibilityState === 'hidden' && flush(true))
}
