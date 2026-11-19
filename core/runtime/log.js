const SEV = { debug: 5, info: 9, warn: 13, error: 17 }
const hex = (n) => Array.from(crypto.getRandomValues(new Uint8Array(n)), (b) => (b + 256).toString(16).slice(1)).join('')
const exc = (e) => ({ 'exception.type': e?.name, 'exception.message': e?.message ?? String(e), 'exception.stacktrace': e?.stack })

let queue = []
let endpoint = '/_otel'
let resource = { 'service.name': 'pragma-client' }
let min = SEV.info
let traceId = hex(16)
let timer
let started

export const newTrace = () => {
  traceId = hex(16)
}

export const traceHeaders = () => ({ traceparent: `00-${traceId}-${hex(8)}-01` })

function emit(level, body, attributes) {
  if (globalThis.__pragmaDev) console[level](body, ...(attributes ? [attributes] : []))
  if (SEV[level] < min || !endpoint) return
  queue.push({
    timeUnixNano: Date.now() + '000000',
    severityNumber: SEV[level],
    severityText: level.toUpperCase(),
    body: String(body),
    attributes: { 'url.path': location.pathname, ...attributes },
    traceId,
  })
  if (queue.length > 49) flush()
  else timer ||= setTimeout(flush, 2000)
}

export const log = {
  debug: (msg, attrs) => emit('debug', msg, attrs),
  info: (msg, attrs) => emit('info', msg, attrs),
  warn: (msg, attrs) => emit('warn', msg, attrs),
  error: (msg, attrs) => emit('error', msg, attrs),
}

export function flush(beacon) {
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

export function initClientLog(opts = {}) {
  endpoint = opts.endpoint ?? endpoint
  resource = { 'service.name': opts.service ?? resource['service.name'] }
  min = SEV[opts.level] ?? min
  if (started) return
  started = 1
  addEventListener('error', (e) => emit('error', e.message, exc(e.error ?? e)))
  addEventListener('unhandledrejection', (e) => emit('error', e.reason?.message ?? String(e.reason), exc(e.reason)))
  addEventListener('visibilitychange', () => document.visibilityState === 'hidden' && flush(true))
}
