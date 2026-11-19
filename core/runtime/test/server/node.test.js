import { test, before, after } from 'node:test'
import assert from 'node:assert/strict'
import { once } from 'node:events'
import { createHandler } from '../../server/handler.js'
import { serve } from '../../server/node.mjs'

const root = new URL('./fixtures/client/', import.meta.url)

const handler = createHandler({
  routes: [
    { pattern: '/shop/:id', server: { load: ({ params }) => ({ id: params.id }), actions: { echo: ({ input }) => input } }, ssr: false, template: null, shell: null },
    {
      pattern: '/blog/:slug',
      server: { load: ({ params }) => ({ slug: params.slug }) },
      ssr: true,
      template: [['html', '<h1>'], ['text', 'data.slug'], ['html', '</h1>']],
      shell: '<!doctype html><html><head></head><body><main data-page></main></body></html>',
    },
  ],
})

let server
let base

before(async (t) => {
  console.log = () => {}
  server = serve(handler, { root, port: 0 })
  await once(server, 'listening')
  base = `http://127.0.0.1:${server.address().port}`
})

after(() => server.close())

const get = (path, init) => fetch(base + path, init)

test('serves hashed assets as immutable with the right MIME', async () => {
  const css = await get('/assets/app-3f9a1c.css')
  assert.equal(css.status, 200)
  assert.equal(css.headers.get('content-type'), 'text/css; charset=utf-8')
  assert.equal(css.headers.get('cache-control'), 'public, max-age=31536000, immutable')
  assert.equal(await css.text(), 'body{margin:0}\n')
  const js = await get('/assets/index-77aa01.js')
  assert.equal(js.headers.get('content-type'), 'text/javascript; charset=utf-8')
})

test('HTML is served no-cache, including folder index', async () => {
  for (const [path, title] of [['/', 'Home'], ['/index.html', 'Home'], ['/shop', 'Shop'], ['/shop/', 'Shop']]) {
    const res = await get(path)
    assert.equal(res.status, 200, path)
    assert.equal(res.headers.get('content-type'), 'text/html; charset=utf-8')
    assert.equal(res.headers.get('cache-control'), 'no-cache')
    assert.match(await res.text(), new RegExp(`<title>${title}</title>`))
  }
  const manifest = await get('/_pragma/manifest.json')
  assert.equal(manifest.headers.get('content-type'), 'application/json; charset=utf-8')
  assert.equal(manifest.headers.get('cache-control'), 'no-cache')
})

test('param route falls back to its HTML via the manifest', async () => {
  const res = await get('/shop/42')
  assert.equal(res.status, 200)
  assert.match(await res.text(), /<title>Product<\/title>/)
})

test('missing paths respond _404.html with 404', async () => {
  for (const path of ['/nothing', '/shop/42/extra', '/assets/missing.css', '/..%2f..%2fetc/passwd', '/%2e%2e/package.json']) {
    const res = await get(path)
    assert.equal(res.status, 404, path)
    assert.match(await res.text(), /Page not found/)
  }
})

test('ETag allows 304 and HEAD sends no body', async () => {
  const first = await get('/assets/app-3f9a1c.css')
  await first.arrayBuffer()
  const etag = first.headers.get('etag')
  assert.ok(etag)
  assert.equal((await get('/assets/app-3f9a1c.css', { headers: { 'if-none-match': etag } })).status, 304)
  const head = await get('/', { method: 'HEAD' })
  assert.equal(head.status, 200)
  assert.equal(await head.text(), '')
})

test('/_data and /_action go through the handler', async () => {
  const data = await get('/_data/shop/7')
  assert.deepEqual(await data.json(), { data: { id: '7' } })
  const act = await get('/_action/shop/7/echo', { method: 'POST', body: JSON.stringify({ a: [1, 2] }), headers: { 'content-type': 'application/json' } })
  assert.deepEqual(await act.json(), { a: [1, 2] })
  assert.equal((await get('/_action/shop/7/nothing', { method: 'POST', body: '{}' })).status, 404)
})

test('ssr route is rendered by the handler', async () => {
  const res = await get('/blog/ol%C3%A1')
  assert.equal(res.status, 200)
  assert.equal(await res.text(), '<!doctype html><html><head></head><body><main data-page><h1>olá</h1></main><script type="application/json" id="pragma-data">{"data":{"slug":"olá"}}</script></body></html>')
})

test('startup log uses the app service name', async () => {
  const records = []
  const quiet = console.log
  console.log = (line) => records.push(JSON.parse(line))
  const named = serve(handler, { root, port: 0, service: 'my-app' })
  await once(named, 'listening')
  console.log = quiet
  named.close()
  const started = records.find((record) => record.body.startsWith('pragma serving at'))
  assert.equal(started.resource['service.name'], 'my-app')
})

test('/_otel goes through the handler', async () => {
  const res = await get('/_otel', { method: 'POST', body: JSON.stringify({ resource: {}, logs: [{ body: 'x' }] }) })
  assert.equal(res.status, 204)
})
