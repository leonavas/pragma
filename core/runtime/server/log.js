const SEVERITY = { DEBUG: 5, INFO: 9, WARN: 13, ERROR: 17 }

const hex = (bytes) => Array.from(crypto.getRandomValues(new Uint8Array(bytes)), (b) => b.toString(16).padStart(2, '0')).join('')
const nonZero = (id) => /[1-9a-f]/.test(id)

export const newTraceId = () => hex(16)
export const newSpanId = () => hex(8)
const nowUnixNano = () => `${Date.now()}000000`

export function parseTraceparent(header) {
  const m = /^([0-9a-f]{2})-([0-9a-f]{32})-([0-9a-f]{16})-([0-9a-f]{2})(-.*)?$/.exec(header?.trim() ?? '')
  if (!m || m[1] === 'ff' || (m[1] === '00' && m[5]) || !nonZero(m[2]) || !nonZero(m[3])) return null
  return { traceId: m[2], spanId: m[3], flags: m[4] }
}

function exceptionAttributes(error) {
  if (!(error instanceof Error)) return { 'exception.message': String(error) }
  const attrs = { 'exception.type': error.name, 'exception.message': error.message }
  if (error.stack) attrs['exception.stacktrace'] = error.stack
  return attrs
}

function parsePairs(text) {
  const out = {}
  for (const pair of (text ?? '').split(',')) {
    const i = pair.indexOf('=')
    if (i > 0) out[decodeURIComponent(pair.slice(0, i).trim())] = decodeURIComponent(pair.slice(i + 1).trim())
  }
  return out
}

export function resourceFrom(env = {}, service) {
  const resource = parsePairs(env.OTEL_RESOURCE_ATTRIBUTES)
  resource['service.name'] = env.OTEL_SERVICE_NAME ?? resource['service.name'] ?? service ?? 'pragma-app'
  return resource
}

export function minSeverity(env = {}) {
  return SEVERITY[String(env.PRAGMA_LOG_LEVEL ?? 'info').toUpperCase()] ?? SEVERITY.INFO
}

export function createLogger({ resource = { 'service.name': 'pragma-app' }, traceId, spanId, min = SEVERITY.INFO, sink } = {}) {
  const records = []
  const emit = (text, msg, attrs) => {
    if (SEVERITY[text] < min) return
    const attributes = attrs instanceof Error ? exceptionAttributes(attrs) : typeof attrs === 'object' ? { ...attrs } : {}
    const rec = { timeUnixNano: nowUnixNano(), severityNumber: SEVERITY[text], severityText: text, body: String(msg), attributes }
    if (traceId) rec.traceId = traceId
    if (spanId) rec.spanId = spanId
    rec.resource = resource
    if (sink) sink(rec)
    else records.push(rec)
  }
  return {
    records,
    debug: (msg, attrs) => emit('DEBUG', msg, attrs),
    info: (msg, attrs) => emit('INFO', msg, attrs),
    warn: (msg, attrs) => emit('WARN', msg, attrs),
    error: (msg, attrs) => emit('ERROR', msg, attrs),
  }
}

export const writeLine = (rec) => console.log(JSON.stringify(rec))

function anyValue(v) {
  if (typeof v === 'string') return { stringValue: v }
  if (typeof v === 'boolean') return { boolValue: v }
  if (typeof v === 'number') return Number.isInteger(v) ? { intValue: String(v) } : { doubleValue: v }
  if (typeof v === 'bigint') return { intValue: String(v) }
  if (Array.isArray(v)) return { arrayValue: { values: v.map(anyValue) } }
  if (v && typeof v === 'object') return { kvlistValue: { values: keyValues(v) } }
  return {}
}

const keyValues = (obj) => Object.entries(obj).filter(([, v]) => v != null).map(([key, v]) => ({ key, value: anyValue(v) }))

export function toOtlp(records) {
  const groups = new Map()
  for (const r of records) {
    const key = JSON.stringify(r.resource ?? {})
    if (!groups.has(key)) groups.set(key, { resource: r.resource ?? {}, logRecords: [] })
    const lr = {
      timeUnixNano: r.timeUnixNano,
      observedTimeUnixNano: r.observedTimeUnixNano ?? r.timeUnixNano,
      severityNumber: r.severityNumber,
      severityText: r.severityText,
      body: { stringValue: r.body },
      attributes: keyValues(r.attributes ?? {}),
    }
    if (r.traceId) lr.traceId = r.traceId
    if (r.spanId) lr.spanId = r.spanId
    groups.get(key).logRecords.push(lr)
  }
  return {
    resourceLogs: [...groups.values()].map((g) => ({
      resource: { attributes: keyValues(g.resource) },
      scopeLogs: [{ scope: { name: 'pragma' }, logRecords: g.logRecords }],
    })),
  }
}

function otlpEndpoint(env = {}) {
  if (env.OTEL_EXPORTER_OTLP_LOGS_ENDPOINT) return env.OTEL_EXPORTER_OTLP_LOGS_ENDPOINT
  if (env.OTEL_EXPORTER_OTLP_ENDPOINT) return env.OTEL_EXPORTER_OTLP_ENDPOINT.replace(/\/+$/, '') + '/v1/logs'
  return null
}

export function flush(records, env = {}) {
  if (!records.length) return Promise.resolve()
  const endpoint = otlpEndpoint(env)
  if (!endpoint) {
    for (const r of records) writeLine(r)
    return Promise.resolve()
  }
  return fetch(endpoint, {
    method: 'POST',
    headers: { ...parsePairs(env.OTEL_EXPORTER_OTLP_HEADERS), 'content-type': 'application/json' },
    body: JSON.stringify(toOtlp(records)),
  }).then(
    (res) => res.ok || fallback(records, `OTLP responded ${res.status}`),
    (err) => fallback(records, err),
  )
}

function fallback(records, error) {
  writeLine({ timeUnixNano: nowUnixNano(), severityNumber: SEVERITY.WARN, severityText: 'WARN', body: 'failed to export OTLP logs', attributes: exceptionAttributes(error), resource: records[0].resource })
  for (const r of records) writeLine(r)
}

const LIMIT = { records: 500, body: 8192, attrs: 64, value: 4096 }
const isHex = (v, n) => typeof v === 'string' && v.length === n && /^[0-9a-f]+$/.test(v) && nonZero(v)

function cleanAttributes(attrs) {
  const out = {}
  if (!attrs || typeof attrs !== 'object') return out
  for (const [k, v] of Object.entries(attrs).slice(0, LIMIT.attrs)) {
    if (typeof v === 'string') out[k.slice(0, 128)] = v.slice(0, LIMIT.value)
    else if (typeof v === 'boolean' || Number.isFinite(v)) out[k.slice(0, 128)] = v
  }
  return out
}

export function fromClient(payload) {
  if (!payload || !Array.isArray(payload.logs)) throw new Error('invalid log batch')
  const resource = {}
  for (const [k, v] of Object.entries(payload.resource ?? {})) if (typeof v === 'string') resource[k.slice(0, 128)] = v.slice(0, 256)
  resource['service.name'] ??= 'pragma-client'
  const observed = nowUnixNano()
  return payload.logs.slice(0, LIMIT.records).filter((r) => r && typeof r === 'object').map((r) => {
    const n = r.severityNumber
    const severityNumber = Number.isInteger(n) && n >= 1 && n <= 24 ? n : SEVERITY.INFO
    const rec = {
      timeUnixNano: typeof r.timeUnixNano === 'string' && /^\d{1,20}$/.test(r.timeUnixNano) ? r.timeUnixNano : observed,
      observedTimeUnixNano: observed,
      severityNumber,
      severityText: typeof r.severityText === 'string' ? r.severityText.slice(0, 16) : Object.keys(SEVERITY).findLast((k) => SEVERITY[k] <= severityNumber) ?? 'TRACE',
      body: String(r.body ?? '').slice(0, LIMIT.body),
      attributes: cleanAttributes(r.attributes),
    }
    if (isHex(r.traceId, 32)) rec.traceId = r.traceId
    if (isHex(r.spanId, 16)) rec.spanId = r.spanId
    rec.resource = resource
    return rec
  })
}
