import { test } from 'node:test'
import assert from 'node:assert/strict'
import { createLogger, fromClient, newSpanId, newTraceId, parseTraceparent, resourceFrom, toOtlp } from '../../server/log.js'

test('parseTraceparent aceita só traceparent W3C válido', () => {
  const tp = '00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01'
  assert.deepEqual(parseTraceparent(tp), { traceId: '4bf92f3577b34da6a3ce929d0e0e4736', spanId: '00f067aa0ba902b7', flags: '01' })
  assert.ok(parseTraceparent('01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01-extra'))
  for (const bad of [
    null,
    '',
    'ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01',
    '00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-01',
    '00-00000000000000000000000000000000-00f067aa0ba902b7-01',
    '00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01',
    '00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01-extra',
    '00-4bf92f3577b34da6a3ce929d0e0e473-00f067aa0ba902b7-01',
  ]) assert.equal(parseTraceparent(bad), null, String(bad))
})

test('ids novos têm o tamanho certo e não repetem', () => {
  assert.match(newTraceId(), /^[0-9a-f]{32}$/)
  assert.match(newSpanId(), /^[0-9a-f]{16}$/)
  assert.notEqual(newTraceId(), newTraceId())
})

test('registro segue o OTel Logs Data Model', () => {
  const log = createLogger({ resource: { 'service.name': 's' }, traceId: 'a'.repeat(32), spanId: 'b'.repeat(16), min: 5 })
  log.debug('d')
  log.info('i', { 'http.request.method': 'GET' })
  log.warn('w')
  log.error('e', new RangeError('fora'))
  log.error('texto', 'não é objeto')
  const [d, i, w, e, t] = log.records
  assert.deepEqual([d, i, w, e].map((r) => [r.severityNumber, r.severityText]), [[5, 'DEBUG'], [9, 'INFO'], [13, 'WARN'], [17, 'ERROR']])
  assert.match(i.timeUnixNano, /^\d{19}$/)
  assert.ok(Math.abs(Number(i.timeUnixNano.slice(0, 13)) - Date.now()) < 5000)
  assert.equal(i.body, 'i')
  assert.deepEqual(i.attributes, { 'http.request.method': 'GET' })
  assert.equal(i.traceId, 'a'.repeat(32))
  assert.equal(i.spanId, 'b'.repeat(16))
  assert.deepEqual(i.resource, { 'service.name': 's' })
  assert.equal(e.attributes['exception.type'], 'RangeError')
  assert.equal(e.attributes['exception.message'], 'fora')
  assert.ok(e.attributes['exception.stacktrace'])
  assert.deepEqual(t.attributes, {})
})

test('nível mínimo padrão descarta DEBUG', () => {
  const log = createLogger({})
  log.debug('x')
  log.info('y')
  assert.equal(log.records.length, 1)
})

test('resourceFrom usa OTEL_SERVICE_NAME e OTEL_RESOURCE_ATTRIBUTES', () => {
  assert.deepEqual(resourceFrom({}, 'app'), { 'service.name': 'app' })
  assert.deepEqual(resourceFrom({ OTEL_RESOURCE_ATTRIBUTES: 'service.version=1.2,deployment.environment=prod' }), {
    'service.version': '1.2',
    'deployment.environment': 'prod',
    'service.name': 'pragma-app',
  })
  assert.equal(resourceFrom({ OTEL_SERVICE_NAME: 'x' }, 'app')['service.name'], 'x')
})

test('toOtlp agrupa por resource e converte AnyValue', () => {
  const r = { 'service.name': 's' }
  const body = toOtlp([
    { timeUnixNano: '1', severityNumber: 9, severityText: 'INFO', body: 'a', attributes: { s: 'x', n: 2, f: 1.5, b: true, l: [1], o: { k: 'v' }, z: null }, resource: r },
    { timeUnixNano: '2', severityNumber: 9, severityText: 'INFO', body: 'b', attributes: {}, resource: r, traceId: 'a'.repeat(32), spanId: 'b'.repeat(16) },
  ])
  assert.equal(body.resourceLogs.length, 1)
  const [a, b] = body.resourceLogs[0].scopeLogs[0].logRecords
  assert.deepEqual(a.attributes, [
    { key: 's', value: { stringValue: 'x' } },
    { key: 'n', value: { intValue: '2' } },
    { key: 'f', value: { doubleValue: 1.5 } },
    { key: 'b', value: { boolValue: true } },
    { key: 'l', value: { arrayValue: { values: [{ intValue: '1' }] } } },
    { key: 'o', value: { kvlistValue: { values: [{ key: 'k', value: { stringValue: 'v' } }] } } },
  ])
  assert.equal(a.traceId, undefined)
  assert.equal(b.traceId, 'a'.repeat(32))
  assert.equal(b.spanId, 'b'.repeat(16))
  assert.equal(b.observedTimeUnixNano, '2')
})

test('fromClient valida e limita o lote do cliente', () => {
  const [r] = fromClient({
    resource: { 'service.name': 'web', n: 1 },
    logs: [{ timeUnixNano: 'abc', severityNumber: 99, body: 'x'.repeat(10000), attributes: { a: 'b', o: {}, n: NaN, ok: 3 }, traceId: 'XYZ', spanId: 'c'.repeat(16) }, null],
  })
  assert.match(r.timeUnixNano, /^\d+$/)
  assert.equal(r.severityNumber, 9)
  assert.equal(r.severityText, 'INFO')
  assert.equal(r.body.length, 8192)
  assert.deepEqual(r.attributes, { a: 'b', ok: 3 })
  assert.equal(r.traceId, undefined)
  assert.equal(r.spanId, 'c'.repeat(16))
  assert.deepEqual(r.resource, { 'service.name': 'web' })
  assert.equal(fromClient({ logs: [{}] })[0].resource['service.name'], 'pragma-client')
  assert.throws(() => fromClient({}))
  assert.equal(fromClient({ logs: Array(900).fill({ body: 'x' }) }).length, 500)
})
