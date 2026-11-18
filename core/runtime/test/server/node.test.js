import { test, before, after } from 'node:test'
import assert from 'node:assert/strict'
import { once } from 'node:events'
import { createHandler } from '../../server/handler.js'
import { serve } from '../../server/node.mjs'

const root = new URL('./fixtures/client/', import.meta.url)

const handler = createHandler({
  routes: [
    { pattern: '/loja/:id', server: { load: ({ params }) => ({ id: params.id }), actions: { eco: ({ input }) => input } }, ssr: false, template: null, shell: null },
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

test('serve asset com hash como immutable e MIME certo', async () => {
  const css = await get('/assets/app-3f9a1c.css')
  assert.equal(css.status, 200)
  assert.equal(css.headers.get('content-type'), 'text/css; charset=utf-8')
  assert.equal(css.headers.get('cache-control'), 'public, max-age=31536000, immutable')
  assert.equal(await css.text(), 'body{margin:0}\n')
  const js = await get('/assets/index-77aa01.js')
  assert.equal(js.headers.get('content-type'), 'text/javascript; charset=utf-8')
})

test('HTML sai com no-cache, inclusive index de pasta', async () => {
  for (const [path, title] of [['/', 'Início'], ['/index.html', 'Início'], ['/loja', 'Loja'], ['/loja/', 'Loja']]) {
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

test('rota com param cai no HTML dela pelo manifesto', async () => {
  const res = await get('/loja/42')
  assert.equal(res.status, 200)
  assert.match(await res.text(), /<title>Produto<\/title>/)
})

test('o que não existe responde _404.html com 404', async () => {
  for (const path of ['/nada', '/loja/42/extra', '/assets/nao.css', '/..%2f..%2fetc/passwd', '/%2e%2e/package.json']) {
    const res = await get(path)
    assert.equal(res.status, 404, path)
    assert.match(await res.text(), /Página não encontrada/)
  }
})

test('ETag permite 304 e HEAD não manda corpo', async () => {
  const first = await get('/assets/app-3f9a1c.css')
  await first.arrayBuffer()
  const etag = first.headers.get('etag')
  assert.ok(etag)
  assert.equal((await get('/assets/app-3f9a1c.css', { headers: { 'if-none-match': etag } })).status, 304)
  const head = await get('/', { method: 'HEAD' })
  assert.equal(head.status, 200)
  assert.equal(await head.text(), '')
})

test('/_data e /_action passam pelo handler', async () => {
  const data = await get('/_data/loja/7')
  assert.deepEqual(await data.json(), { data: { id: '7' } })
  const act = await get('/_action/loja/7/eco', { method: 'POST', body: JSON.stringify({ a: [1, 2] }), headers: { 'content-type': 'application/json' } })
  assert.deepEqual(await act.json(), { a: [1, 2] })
  assert.equal((await get('/_action/loja/7/nada', { method: 'POST', body: '{}' })).status, 404)
})

test('rota ssr é renderizada pelo handler', async () => {
  const res = await get('/blog/ol%C3%A1')
  assert.equal(res.status, 200)
  assert.equal(await res.text(), '<!doctype html><html><head></head><body><main data-page><h1>olá</h1></main><script type="application/json" id="pragma-data">{"data":{"slug":"olá"}}</script></body></html>')
})

test('/_otel passa pelo handler', async () => {
  const res = await get('/_otel', { method: 'POST', body: JSON.stringify({ resource: {}, logs: [{ body: 'x' }] }) })
  assert.equal(res.status, 204)
})
