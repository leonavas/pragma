import { test, before } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { gzipSync } from 'node:zlib'
import { parseTraceparent } from '../../server/log.js'

const listeners = {}
const posts = []
const beacons = []
let mod

before(async () => {
  globalThis.addEventListener = (type, fn) => (listeners[type] = fn)
  globalThis.location = { pathname: '/loja/1' }
  globalThis.document = { visibilityState: 'visible' }
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: { sendBeacon: (url, blob) => (beacons.push({ url, blob }), true) },
  })
  globalThis.fetch = async (url, init) => (posts.push({ url, init }), new Response(null, { status: 204 }))
  mod = await import('../../log.js')
  mod.initClientLog({ endpoint: '/_otel', service: 'loja-web' })
})

test('log.js do cliente tem menos de 1,5 KB gzip', () => {
  const size = gzipSync(readFileSync(new URL('../../log.js', import.meta.url)), { level: 9 }).length
  assert.ok(size < 1536, `${size} bytes`)
})

test('traceHeaders gera traceparent W3C válido e continua o trace', () => {
  const a = mod.traceHeaders().traceparent
  const b = mod.traceHeaders().traceparent
  assert.match(a, /^00-[0-9a-f]{32}-[0-9a-f]{16}-01$/)
  const pa = parseTraceparent(a)
  const pb = parseTraceparent(b)
  assert.ok(pa && pb)
  assert.equal(pa.traceId, pb.traceId)
  assert.notEqual(pa.spanId, pb.spanId)
  mod.newTrace()
  assert.notEqual(parseTraceparent(mod.traceHeaders().traceparent).traceId, pa.traceId)
})

test('envia em lote por fetch keepalive no formato do /_otel', () => {
  posts.length = 0
  mod.log.debug('some')
  mod.log.info('oi', { 'app.x': 1 })
  mod.log.warn('cuidado')
  mod.flush()
  assert.equal(posts.length, 1)
  const { url, init } = posts[0]
  assert.equal(url, '/_otel')
  assert.equal(init.method, 'POST')
  assert.equal(init.keepalive, true)
  const body = JSON.parse(init.body)
  assert.deepEqual(body.resource, { 'service.name': 'loja-web' })
  assert.equal(body.logs.length, 2)
  const [info, warn] = body.logs
  assert.equal(info.severityNumber, 9)
  assert.equal(info.severityText, 'INFO')
  assert.equal(info.body, 'oi')
  assert.deepEqual(info.attributes, { 'url.path': '/loja/1', 'app.x': 1 })
  assert.match(info.timeUnixNano, /^\d{19}$/)
  assert.match(info.traceId, /^[0-9a-f]{32}$/)
  assert.equal(warn.severityNumber, 13)
  mod.flush()
  assert.equal(posts.length, 1)
})

test('lote cheio dispara o envio sozinho', () => {
  posts.length = 0
  for (let i = 0; i < 50; i++) mod.log.info('n' + i)
  assert.equal(posts.length, 1)
  assert.equal(JSON.parse(posts[0].init.body).logs.length, 50)
})

test('captura error e unhandledrejection com exception.*', () => {
  posts.length = 0
  const err = new TypeError('x is undefined')
  listeners.error({ message: 'Uncaught TypeError: x is undefined', error: err })
  listeners.unhandledrejection({ reason: new Error('falhou') })
  listeners.unhandledrejection({ reason: 'texto' })
  mod.flush()
  const [a, b, c] = JSON.parse(posts[0].init.body).logs
  assert.equal(a.severityText, 'ERROR')
  assert.equal(a.attributes['exception.type'], 'TypeError')
  assert.equal(a.attributes['exception.message'], 'x is undefined')
  assert.ok(a.attributes['exception.stacktrace'])
  assert.equal(b.body, 'falhou')
  assert.equal(c.body, 'texto')
  assert.equal(c.attributes['exception.message'], 'texto')
})

test('visibilitychange para hidden usa sendBeacon', async () => {
  posts.length = 0
  mod.log.error('saindo')
  document.visibilityState = 'hidden'
  listeners.visibilitychange()
  document.visibilityState = 'visible'
  assert.equal(posts.length, 0)
  assert.equal(beacons.length, 1)
  assert.equal(beacons[0].url, '/_otel')
  assert.equal(beacons[0].blob.type, 'application/json')
  assert.equal(JSON.parse(await beacons[0].blob.text()).logs[0].body, 'saindo')
})

test('o lote do cliente é aceito pelo /_otel do servidor', async (t) => {
  const { createHandler } = await import('../../server/handler.js')
  const lines = []
  t.mock.method(console, 'log', (l) => lines.push(JSON.parse(l)))
  posts.length = 0
  mod.log.warn('fim', { 'app.n': 2 })
  mod.flush()
  const res = await createHandler({ routes: [] }).fetch(new Request('http://x/_otel', { method: 'POST', body: posts[0].init.body }), {})
  assert.equal(res.status, 204)
  assert.equal(lines[0].body, 'fim')
  assert.equal(lines[0].resource['service.name'], 'loja-web')
  assert.deepEqual(lines[0].attributes, { 'url.path': '/loja/1', 'app.n': 2 })
})
