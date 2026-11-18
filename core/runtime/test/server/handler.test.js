import { test } from 'node:test'
import assert from 'node:assert/strict'
import { compileRoutes, createHandler, matchRoute } from '../../server/handler.js'

const SHELL = `<!doctype html>
<html lang="pt-BR">
<head>
  <title>Post</title>
  <script>self.__pd=fetch('/_data'+location.pathname+location.search,{headers:{accept:'application/json'}})</script>
  <script type="application/json" id="pragma-route">{"pattern":"/blog/:slug","ssr":true}</script>
</head>
<body><header>topo</header><main data-page class="p-4"><template data-if="page.loading"><p class="skeleton"></p></template></main></body>
</html>`

const TEMPLATE = [
  ['html', '<h1>'],
  ['text', 'data.titulo'],
  ['html', '</h1><ul><template data-tid="t1" data-for="tag in data.tags" data-key="tag.id"><li></li></template>'],
  ['for', 't1', 'tag', 'data.tags', 'tag.id', [['html', '<li>'], ['text', 'tag.nome'], ['html', '</li>']], []],
  ['html', '</ul><p>'],
  ['text', 'params.slug'],
  ['html', '</p>'],
]

const loja = {
  async load({ params, query }) {
    if (params.id === 'quebra') throw new TypeError('banco fora')
    return { id: params.id, ordem: query.ordem ?? null }
  },
  actions: {
    async salvar({ input, params }) {
      return { ok: true, id: params.id, input }
    },
    async falhar() {
      throw new Error('não deu')
    },
  },
}

const routes = [
  { pattern: '/', server: { load: () => ({ home: true }) }, ssr: false, template: null, shell: null },
  { pattern: '/loja/:id', server: loja, ssr: false, template: null, shell: null },
  { pattern: '/loja/novo', server: { load: () => ({ novo: true }) }, ssr: false, template: null, shell: null },
  {
    pattern: '/blog/:slug',
    server: {
      load({ params }) {
        if (params.slug === 'erro') throw new Error('sem post')
        return { titulo: '</script><b>oi</b>', tags: [{ id: 'a', nome: 'A&B' }, { id: 'b', nome: 'C' }] }
      },
    },
    ssr: true,
    template: TEMPLATE,
    shell: SHELL,
  },
]

const handler = createHandler({ routes, service: 'loja-teste' })

function capture(t) {
  const lines = []
  t.mock.method(console, 'log', (line) => lines.push(JSON.parse(line)))
  return lines
}

const get = (path, init) => handler.fetch(new Request('http://x' + path, init), {})

test('estático vence param e params são decodificados', () => {
  const table = compileRoutes(routes)
  assert.equal(matchRoute(table, '/loja/novo').route.pattern, '/loja/novo')
  assert.deepEqual(matchRoute(table, '/loja/caf%C3%A9%20x').params, { id: 'café x' })
  assert.deepEqual(matchRoute(table, '/loja/a%2Fb').params, { id: 'a/b' })
  assert.equal(matchRoute(table, '/loja/42/').route.pattern, '/loja/:id')
  assert.equal(matchRoute(table, '/').route.pattern, '/')
  assert.equal(matchRoute(table, '/loja'), null)
  assert.equal(matchRoute(table, '/loja/%E0%A4%A'), null)
})

test('GET /_data devolve {data} com params e query', async (t) => {
  capture(t)
  const res = await get('/_data/loja/42?ordem=preco')
  assert.equal(res.status, 200)
  assert.match(res.headers.get('content-type'), /application\/json/)
  assert.deepEqual(await res.json(), { data: { id: '42', ordem: 'preco' } })
  assert.deepEqual(await (await get('/_data/')).json(), { data: { home: true } })
  assert.deepEqual(await (await get('/_data/loja/novo')).json(), { data: { novo: true } })
})

test('GET /_data com erro: 500, {error} e log ERROR com exception.*', async (t) => {
  const lines = capture(t)
  const res = await get('/_data/loja/quebra')
  assert.equal(res.status, 500)
  assert.deepEqual(await res.json(), { error: { message: 'banco fora' } })
  const err = lines.find((l) => l.severityText === 'ERROR' && l.attributes['exception.type'])
  assert.equal(err.severityNumber, 17)
  assert.equal(err.attributes['exception.type'], 'TypeError')
  assert.equal(err.attributes['exception.message'], 'banco fora')
  assert.match(err.attributes['exception.stacktrace'], /banco fora/)
  const span = lines.at(-1)
  assert.equal(span.attributes['http.response.status_code'], 500)
  assert.equal(span.attributes['error.type'], '500')
  assert.equal(span.traceId, err.traceId)
  assert.equal(span.spanId, err.spanId)
})

test('GET /_data de rota sem load ou inexistente dá 404', async (t) => {
  capture(t)
  assert.equal((await get('/_data/nada/aqui')).status, 404)
  assert.equal((await get('/_data/loja/1', { method: 'POST' })).status, 405)
})

test('POST /_action chama a action com input JSON', async (t) => {
  capture(t)
  const res = await get('/_action/loja/7/salvar', { method: 'POST', body: JSON.stringify({ qtd: 2 }) })
  assert.equal(res.status, 200)
  assert.deepEqual(await res.json(), { ok: true, id: '7', input: { qtd: 2 } })
  const vazio = await get('/_action/loja/7/salvar', { method: 'POST' })
  assert.deepEqual(await vazio.json(), { ok: true, id: '7', input: null })
})

test('/_action: inexistente 404, JSON inválido 400, GET 405, erro 500', async (t) => {
  capture(t)
  assert.equal((await get('/_action/loja/7/apagar', { method: 'POST', body: '{}' })).status, 404)
  assert.equal((await get('/_action/loja/7/toString', { method: 'POST', body: '{}' })).status, 404)
  assert.equal((await get('/_action/nada/salvar', { method: 'POST', body: '{}' })).status, 404)
  assert.equal((await get('/_action/loja/7/salvar', { method: 'POST', body: '{x' })).status, 400)
  assert.equal((await get('/_action/loja/7/salvar')).status, 405)
  const res = await get('/_action/loja/7/falhar', { method: 'POST', body: '{}' })
  assert.equal(res.status, 500)
  assert.deepEqual(await res.json(), { error: { message: 'não deu' } })
})

test('SSR: renderiza no <main data-page>, escapa e embute pragma-data', async (t) => {
  capture(t)
  const res = await get('/blog/ola')
  assert.equal(res.status, 200)
  assert.match(res.headers.get('content-type'), /text\/html/)
  const html = await res.text()
  assert.ok(html.includes('<main data-page class="p-4"><h1>&lt;/script&gt;&lt;b&gt;oi&lt;/b&gt;</h1>'))
  assert.ok(html.includes('</template><li data-of="t1" data-key="a">A&amp;B</li><li data-of="t1" data-key="b">C</li></ul><p>ola</p></main>'))
  assert.ok(!html.includes('skeleton'))
  assert.ok(!html.includes('self.__pd'))
  assert.ok(html.includes('<header>topo</header>'))
  const m = /<script type="application\/json" id="pragma-data">(.*?)<\/script><\/body>/s.exec(html)
  assert.ok(m)
  assert.ok(!m[1].includes('<'))
  assert.ok(m[1].includes('\\u003c/script>'))
  assert.deepEqual(JSON.parse(m[1]), { data: { titulo: '</script><b>oi</b>', tags: [{ id: 'a', nome: 'A&B' }, { id: 'b', nome: 'C' }] } })
  assert.equal(await (await get('/blog/ola')).text(), html)
})

test('SSR: caminho .html é arquivo pré-gerado, não página SSR', async () => {
  const env = { ASSETS: { fetch: async (r) => new Response('estatico ' + new URL(r.url).pathname) } }
  const res = await handler.fetch(new Request('http://x/blog/index.html'), env)
  assert.equal(await res.text(), 'estatico /blog/index.html')
})

test('SSR com erro no load: 500, shell intacto e pragma-data com error', async (t) => {
  const lines = capture(t)
  const res = await get('/blog/erro')
  assert.equal(res.status, 500)
  const html = await res.text()
  assert.ok(html.includes('<p class="skeleton">'))
  assert.ok(html.includes('<script type="application/json" id="pragma-data">{"error":{"message":"sem post"}}</script>'))
  assert.ok(lines.some((l) => l.severityText === 'ERROR' && l.attributes['exception.message'] === 'sem post'))
})

test('o resto vai para env.ASSETS', async (t) => {
  capture(t)
  const seen = []
  const env = { ASSETS: { fetch: (req) => (seen.push(new URL(req.url).pathname), new Response('estático')) } }
  for (const p of ['/', '/loja/42', '/assets/app.css']) {
    assert.equal(await (await handler.fetch(new Request('http://x' + p), env)).text(), 'estático')
  }
  await handler.fetch(new Request('http://x/blog/ola', { method: 'POST' }), env)
  assert.deepEqual(seen, ['/', '/loja/42', '/assets/app.css', '/blog/ola'])
  assert.equal((await get('/qualquer')).status, 404)
})

test('span por request continua o traceparent recebido', async (t) => {
  const lines = capture(t)
  const traceId = '4bf92f3577b34da6a3ce929d0e0e4736'
  const parent = '00f067aa0ba902b7'
  await get('/_data/loja/1', { headers: { traceparent: `00-${traceId}-${parent}-01` } })
  const span = lines.at(-1)
  assert.equal(span.body, 'GET /loja/:id')
  assert.equal(span.severityText, 'INFO')
  assert.equal(span.traceId, traceId)
  assert.match(span.spanId, /^[0-9a-f]{16}$/)
  assert.notEqual(span.spanId, parent)
  assert.equal(span.attributes['http.request.method'], 'GET')
  assert.equal(span.attributes['url.path'], '/_data/loja/1')
  assert.equal(span.attributes['http.route'], '/loja/:id')
  assert.equal(span.attributes['http.response.status_code'], 200)
  assert.equal(typeof span.attributes['http.server.request.duration'], 'number')
  assert.equal(span.resource['service.name'], 'loja-teste')

  await get('/_data/loja/1', { headers: { traceparent: `00-${'0'.repeat(32)}-${parent}-01` } })
  assert.match(lines.at(-1).traceId, /^[0-9a-f]{32}$/)
  assert.notEqual(lines.at(-1).traceId, '0'.repeat(32))
})

test('load recebe log que herda traceId/spanId', async (t) => {
  const lines = capture(t)
  const h = createHandler({
    routes: [{ pattern: '/x', server: { load: ({ log }) => (log.warn('cuidado', { 'app.n': 1 }), log.debug('oculto'), 1) } }],
  })
  await h.fetch(new Request('http://x/_data/x'), { OTEL_SERVICE_NAME: 'svc', OTEL_RESOURCE_ATTRIBUTES: 'deployment.environment=prod' })
  assert.equal(lines.length, 2)
  const [warn, span] = lines
  assert.equal(warn.severityNumber, 13)
  assert.deepEqual(warn.attributes, { 'app.n': 1 })
  assert.equal(warn.traceId, span.traceId)
  assert.equal(warn.spanId, span.spanId)
  assert.deepEqual(span.resource, { 'deployment.environment': 'prod', 'service.name': 'svc' })
})

const batch = {
  resource: { 'service.name': 'web' },
  logs: [
    { timeUnixNano: '1700000000000000000', severityNumber: 17, severityText: 'ERROR', body: 'quebrou', attributes: { 'url.path': '/loja', 'exception.type': 'TypeError', x: { y: 1 } }, traceId: '4bf92f3577b34da6a3ce929d0e0e4736' },
  ],
}

test('POST /_otel sem endpoint imprime JSON OTel no stdout', async (t) => {
  const lines = capture(t)
  const res = await get('/_otel', { method: 'POST', body: JSON.stringify(batch) })
  assert.equal(res.status, 204)
  assert.equal(lines.length, 1)
  assert.equal(lines[0].body, 'quebrou')
  assert.equal(lines[0].severityNumber, 17)
  assert.equal(lines[0].traceId, batch.logs[0].traceId)
  assert.deepEqual(lines[0].attributes, { 'url.path': '/loja', 'exception.type': 'TypeError' })
  assert.deepEqual(lines[0].resource, { 'service.name': 'web' })
  assert.equal((await get('/_otel', { method: 'POST', body: 'x' })).status, 400)
  assert.equal((await get('/_otel')).status, 405)
  assert.equal((await get('/_otel', { method: 'POST', body: 'x'.repeat(300 * 1024) })).status, 413)
})

test('POST /_otel com OTEL_EXPORTER_OTLP_ENDPOINT repassa OTLP/HTTP JSON', async (t) => {
  const lines = capture(t)
  const sent = []
  t.mock.method(globalThis, 'fetch', async (url, init) => (sent.push({ url, init }), new Response(null, { status: 200 })))
  const waits = []
  const env = { OTEL_EXPORTER_OTLP_ENDPOINT: 'http://col:4318/', OTEL_EXPORTER_OTLP_HEADERS: 'x-api-key=abc' }
  const res = await handler.fetch(new Request('http://x/_otel', { method: 'POST', body: JSON.stringify(batch) }), env, { waitUntil: (p) => waits.push(p) })
  assert.equal(res.status, 204)
  await Promise.all(waits)
  assert.equal(lines.length, 0)
  assert.equal(sent.length, 1)
  assert.equal(sent[0].url, 'http://col:4318/v1/logs')
  assert.equal(sent[0].init.headers['x-api-key'], 'abc')
  assert.equal(sent[0].init.headers['content-type'], 'application/json')
  const body = JSON.parse(sent[0].init.body)
  const rl = body.resourceLogs[0]
  assert.deepEqual(rl.resource.attributes, [{ key: 'service.name', value: { stringValue: 'web' } }])
  const rec = rl.scopeLogs[0].logRecords[0]
  assert.equal(rec.timeUnixNano, '1700000000000000000')
  assert.equal(rec.severityNumber, 17)
  assert.deepEqual(rec.body, { stringValue: 'quebrou' })
  assert.equal(rec.traceId, batch.logs[0].traceId)
  assert.deepEqual(rec.attributes[0], { key: 'url.path', value: { stringValue: '/loja' } })
})

test('falha no export OTLP cai para o stdout', async (t) => {
  const lines = capture(t)
  t.mock.method(globalThis, 'fetch', async () => {
    throw new Error('recusado')
  })
  const res = await handler.fetch(new Request('http://x/_otel', { method: 'POST', body: JSON.stringify(batch) }), { OTEL_EXPORTER_OTLP_ENDPOINT: 'http://col:4318' })
  assert.equal(res.status, 204)
  assert.equal(lines[0].severityText, 'WARN')
  assert.equal(lines[1].body, 'quebrou')
})
