import { test } from 'node:test'
import assert from 'node:assert/strict'
import { compileRoutes, createHandler, earlyHints, matchRoute, prefersMarkdown } from '../../server/handler.js'

const SHELL = `<!doctype html>
<html lang="en">
<head>
  <title>Post</title>
  <script>self.__pd=fetch('/_data'+location.pathname+location.search,{headers:{accept:'application/json'}})</script>
  <script type="application/json" id="pragma-route">{"pattern":"/blog/:slug","ssr":true}</script>
</head>
<body><header>top</header><main data-page class="p-4"><template data-if="page.loading"><p class="skeleton"></p></template></main></body>
</html>`

const TEMPLATE = [
  ['html', '<h1>'],
  ['text', '{data.title}'],
  ['html', '</h1><ul><template data-tid="t1" data-for="tag in data.tags" data-key="tag.id"><li></li></template>'],
  ['for', 't1', 'tag', 'data.tags', 'tag.id', [['html', '<li>'], ['text', '{tag.name}'], ['html', '</li>']]],
  ['html', '</ul><p>'],
  ['text', '{params.slug}'],
  ['html', '</p>'],
]

const shop = {
  async load({ params, query }) {
    if (params.id === 'broken') throw new TypeError('database down')
    return { id: params.id, order: query.order ?? null }
  },
  actions: {
    async save({ input, params }) {
      return { ok: true, id: params.id, input }
    },
    async fail() {
      throw new Error('it failed')
    },
  },
}

const routes = [
  { pattern: '/', server: { load: () => ({ home: true }) }, ssr: false, template: null, shell: null },
  { pattern: '/shop/:id', server: shop, ssr: false, template: null, shell: null },
  { pattern: '/shop/new', server: { load: () => ({ new: true }) }, ssr: false, template: null, shell: null },
  {
    pattern: '/blog/:slug',
    server: {
      load({ params }) {
        if (params.slug === 'error') throw new Error('no post')
        return { title: '</script><b>hi</b>', tags: [{ id: 'a', name: 'A&B' }, { id: 'b', name: 'C' }] }
      },
    },
    ssr: true,
    template: TEMPLATE,
    shell: SHELL,
  },
]

const handler = createHandler({ routes, service: 'shop-test' })

function capture(t) {
  const lines = []
  t.mock.method(console, 'log', (line) => lines.push(JSON.parse(line)))
  return lines
}

const get = (path, init) => handler.fetch(new Request('http://x' + path, init), {})

test('static beats param and params are decoded', () => {
  const table = compileRoutes(routes)
  assert.equal(matchRoute(table, '/shop/new').route.pattern, '/shop/new')
  assert.deepEqual(matchRoute(table, '/shop/caf%C3%A9%20x').params, { id: 'café x' })
  assert.deepEqual(matchRoute(table, '/shop/a%2Fb').params, { id: 'a/b' })
  assert.equal(matchRoute(table, '/shop/42/').route.pattern, '/shop/:id')
  assert.equal(matchRoute(table, '/').route.pattern, '/')
  assert.equal(matchRoute(table, '/shop'), null)
  assert.equal(matchRoute(table, '/shop/%E0%A4%A'), null)
})

test('GET /_data returns {data} with params and query', async (t) => {
  capture(t)
  const res = await get('/_data/shop/42?order=price')
  assert.equal(res.status, 200)
  assert.match(res.headers.get('content-type'), /application\/json/)
  assert.deepEqual(await res.json(), { data: { id: '42', order: 'price' } })
  assert.deepEqual(await (await get('/_data/')).json(), { data: { home: true } })
  assert.deepEqual(await (await get('/_data/shop/new')).json(), { data: { new: true } })
})

test('GET /_data error: 500, {error} and ERROR log with exception.*', async (t) => {
  const lines = capture(t)
  const res = await get('/_data/shop/broken')
  assert.equal(res.status, 500)
  assert.deepEqual(await res.json(), { error: { message: 'database down' } })
  const err = lines.find((l) => l.severityText === 'ERROR' && l.attributes['exception.type'])
  assert.equal(err.severityNumber, 17)
  assert.equal(err.attributes['exception.type'], 'TypeError')
  assert.equal(err.attributes['exception.message'], 'database down')
  assert.match(err.attributes['exception.stacktrace'], /database down/)
  const span = lines.at(-1)
  assert.equal(span.attributes['http.response.status_code'], 500)
  assert.equal(span.attributes['error.type'], '500')
  assert.equal(span.traceId, err.traceId)
  assert.equal(span.spanId, err.spanId)
})

test('GET /_data for a route without load or unknown route is 404', async (t) => {
  capture(t)
  assert.equal((await get('/_data/nothing/here')).status, 404)
  assert.equal((await get('/_data/shop/1', { method: 'POST' })).status, 405)
})

test('POST /_action calls the action with JSON input', async (t) => {
  capture(t)
  const res = await get('/_action/shop/7/save', { method: 'POST', body: JSON.stringify({ qty: 2 }) })
  assert.equal(res.status, 200)
  assert.deepEqual(await res.json(), { ok: true, id: '7', input: { qty: 2 } })
  const empty = await get('/_action/shop/7/save', { method: 'POST' })
  assert.deepEqual(await empty.json(), { ok: true, id: '7', input: null })
})

test('/_action: unknown 404, invalid JSON 400, GET 405, error 500', async (t) => {
  capture(t)
  assert.equal((await get('/_action/shop/7/delete', { method: 'POST', body: '{}' })).status, 404)
  assert.equal((await get('/_action/shop/7/toString', { method: 'POST', body: '{}' })).status, 404)
  assert.equal((await get('/_action/nothing/save', { method: 'POST', body: '{}' })).status, 404)
  assert.equal((await get('/_action/shop/7/save', { method: 'POST', body: '{x' })).status, 400)
  assert.equal((await get('/_action/shop/7/save')).status, 405)
  const res = await get('/_action/shop/7/fail', { method: 'POST', body: '{}' })
  assert.equal(res.status, 500)
  assert.deepEqual(await res.json(), { error: { message: 'it failed' } })
})

test('SSR: renders into <main data-page>, escapes and embeds pragma-data', async (t) => {
  capture(t)
  const res = await get('/blog/hello')
  assert.equal(res.status, 200)
  assert.match(res.headers.get('content-type'), /text\/html/)
  const html = await res.text()
  assert.ok(html.includes('<main data-page class="p-4"><h1>&lt;/script&gt;&lt;b&gt;hi&lt;/b&gt;</h1>'))
  assert.ok(html.includes('</template><li data-of="t1" data-key="a">A&amp;B</li><li data-of="t1" data-key="b">C</li></ul><p>hello</p></main>'))
  assert.ok(!html.includes('skeleton'))
  assert.ok(!html.includes('self.__pd'))
  assert.ok(html.includes('<header>top</header>'))
  const m = /<script type="application\/json" id="pragma-data">(.*?)<\/script><\/body>/s.exec(html)
  assert.ok(m)
  assert.ok(!m[1].includes('<'))
  assert.ok(m[1].includes('\\u003c/script>'))
  assert.deepEqual(JSON.parse(m[1]), { data: { title: '</script><b>hi</b>', tags: [{ id: 'a', name: 'A&B' }, { id: 'b', name: 'C' }] } })
  assert.equal(await (await get('/blog/hello')).text(), html)
})

test('SSR: a .html path is a prebuilt file, not an SSR page', async () => {
  const env = { ASSETS: { fetch: async (r) => new Response('static ' + new URL(r.url).pathname) } }
  const res = await handler.fetch(new Request('http://x/blog/index.html'), env)
  assert.equal(await res.text(), 'static /blog/index.html')
})

test('SSR with load error: 500, shell intact and pragma-data with error', async (t) => {
  const lines = capture(t)
  const res = await get('/blog/error')
  assert.equal(res.status, 500)
  const html = await res.text()
  assert.ok(html.includes('<p class="skeleton">'))
  assert.ok(html.includes('<script type="application/json" id="pragma-data">{"error":{"message":"no post"}}</script>'))
  assert.ok(lines.some((l) => l.severityText === 'ERROR' && l.attributes['exception.message'] === 'no post'))
})

test('everything else goes to env.ASSETS', async (t) => {
  capture(t)
  const seen = []
  const env = { ASSETS: { fetch: (req) => (req.method !== 'HEAD' && seen.push(new URL(req.url).pathname), new Response('static')) } }
  for (const p of ['/', '/shop/42', '/assets/app.css']) {
    assert.equal(await (await handler.fetch(new Request('http://x' + p), env)).text(), 'static')
  }
  await handler.fetch(new Request('http://x/blog/hello', { method: 'POST' }), env)
  assert.deepEqual(seen, ['/', '/shop/42', '/assets/app.css', '/blog/hello'])
  assert.equal((await get('/anything')).status, 404)
})

test('per-request span continues the incoming traceparent', async (t) => {
  const lines = capture(t)
  const traceId = '4bf92f3577b34da6a3ce929d0e0e4736'
  const parent = '00f067aa0ba902b7'
  await get('/_data/shop/1', { headers: { traceparent: `00-${traceId}-${parent}-01` } })
  const span = lines.at(-1)
  assert.equal(span.body, 'GET /shop/:id')
  assert.equal(span.severityText, 'INFO')
  assert.equal(span.traceId, traceId)
  assert.match(span.spanId, /^[0-9a-f]{16}$/)
  assert.notEqual(span.spanId, parent)
  assert.equal(span.attributes['http.request.method'], 'GET')
  assert.equal(span.attributes['url.path'], '/_data/shop/1')
  assert.equal(span.attributes['http.route'], '/shop/:id')
  assert.equal(span.attributes['http.response.status_code'], 200)
  assert.equal(typeof span.attributes['http.server.request.duration'], 'number')
  assert.equal(span.resource['service.name'], 'shop-test')

  await get('/_data/shop/1', { headers: { traceparent: `00-${'0'.repeat(32)}-${parent}-01` } })
  assert.match(lines.at(-1).traceId, /^[0-9a-f]{32}$/)
  assert.notEqual(lines.at(-1).traceId, '0'.repeat(32))
})

test('load gets a log that inherits traceId/spanId', async (t) => {
  const lines = capture(t)
  const h = createHandler({
    routes: [{ pattern: '/x', server: { load: ({ log }) => (log.warn('careful', { 'app.n': 1 }), log.debug('hidden'), 1) } }],
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
    { timeUnixNano: '1700000000000000000', severityNumber: 17, severityText: 'ERROR', body: 'broke', attributes: { 'url.path': '/shop', 'exception.type': 'TypeError', x: { y: 1 } }, traceId: '4bf92f3577b34da6a3ce929d0e0e4736' },
  ],
}

test('POST /_otel without endpoint prints OTel JSON to stdout', async (t) => {
  const lines = capture(t)
  const res = await get('/_otel', { method: 'POST', body: JSON.stringify(batch) })
  assert.equal(res.status, 204)
  assert.equal(lines.length, 1)
  assert.equal(lines[0].body, 'broke')
  assert.equal(lines[0].severityNumber, 17)
  assert.equal(lines[0].traceId, batch.logs[0].traceId)
  assert.deepEqual(lines[0].attributes, { 'url.path': '/shop', 'exception.type': 'TypeError' })
  assert.deepEqual(lines[0].resource, { 'service.name': 'web' })
  assert.equal((await get('/_otel', { method: 'POST', body: 'x' })).status, 400)
  assert.equal((await get('/_otel')).status, 405)
  assert.equal((await get('/_otel', { method: 'POST', body: 'x'.repeat(300 * 1024) })).status, 413)
})

test('POST /_otel with OTEL_EXPORTER_OTLP_ENDPOINT forwards OTLP/HTTP JSON', async (t) => {
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
  assert.deepEqual(rec.body, { stringValue: 'broke' })
  assert.equal(rec.traceId, batch.logs[0].traceId)
  assert.deepEqual(rec.attributes[0], { key: 'url.path', value: { stringValue: '/shop' } })
})

test('OTLP export failure falls back to stdout', async (t) => {
  const lines = capture(t)
  t.mock.method(globalThis, 'fetch', async () => {
    throw new Error('refused')
  })
  const res = await handler.fetch(new Request('http://x/_otel', { method: 'POST', body: JSON.stringify(batch) }), { OTEL_EXPORTER_OTLP_ENDPOINT: 'http://col:4318' })
  assert.equal(res.status, 204)
  assert.equal(lines[0].severityText, 'WARN')
  assert.equal(lines[1].body, 'broke')
})

test('prefersMarkdown compares q values for text/markdown and text/html', () => {
  assert.equal(prefersMarkdown('text/markdown'), true)
  assert.equal(prefersMarkdown('text/markdown, text/html'), true)
  assert.equal(prefersMarkdown('text/markdown;q=0.9, text/html;q=0.9'), true)
  assert.equal(prefersMarkdown('text/html, text/markdown;q=0.5'), false)
  assert.equal(prefersMarkdown('text/html, */*'), false)
  assert.equal(prefersMarkdown(''), false)
})

test('handler serves markdown for Accept: text/markdown with Vary: Accept', async () => {
  const files = {
    '/': { body: '<h1>Home</h1>', type: 'text/html; charset=utf-8' },
    '/index.md': { body: '# Home\n', type: 'text/plain' },
    '/about': { body: '<h1>About</h1>', type: 'text/html; charset=utf-8' },
    '/about.md': { body: '# About\n', type: 'text/plain' },
    '/plain': { body: '<h1>Plain</h1>', type: 'text/html; charset=utf-8' },
  }
  const env = {
    ASSETS: {
      async fetch(req) {
        const { pathname } = new URL(req.url)
        const f = files[pathname]
        if (!f) return new Response('Not found', { status: 404 })
        return new Response(req.method === 'HEAD' ? null : f.body, {
          status: 200,
          headers: { 'content-type': f.type },
        })
      },
    },
  }
  const h = createHandler({ routes: [], service: 'web' })

  const md = await h.fetch(new Request('http://x/', { headers: { accept: 'text/markdown, text/html' } }), env)
  assert.equal(md.status, 200)
  assert.equal(md.headers.get('content-type'), 'text/markdown; charset=utf-8')
  assert.match(md.headers.get('vary') ?? '', /accept/i)
  assert.equal(await md.text(), '# Home\n')

  const html = await h.fetch(new Request('http://x/about', { headers: { accept: 'text/html, text/markdown;q=0.5' } }), env)
  assert.equal(html.status, 200)
  assert.equal(html.headers.get('content-type'), 'text/html; charset=utf-8')
  assert.match(html.headers.get('vary') ?? '', /accept/i)
  assert.equal(await html.text(), '<h1>About</h1>')

  const direct = await h.fetch(new Request('http://x/about.md'), env)
  assert.equal(direct.status, 200)
  assert.equal(direct.headers.get('content-type'), 'text/markdown; charset=utf-8')
  assert.equal(await direct.text(), '# About\n')

  const plain = await h.fetch(new Request('http://x/plain', { headers: { accept: 'text/markdown' } }), env)
  assert.equal(plain.status, 200)
  assert.equal(plain.headers.get('content-type'), 'text/html; charset=utf-8')
  assert.equal(plain.headers.get('vary'), null)
})

test('SSR 200 emits Link preload headers for stylesheets without duplicates or modulepreload', async (t) => {
  capture(t)
  const shell = `<!doctype html>
<html>
<head>
  <link rel="stylesheet" href="/assets/app.css">
  <link href="/assets/theme.css" rel="stylesheet">
  <link rel="stylesheet" href="/assets/app.css">
  <link rel="modulepreload" href="/assets/app.js" crossorigin>
  <link rel="icon" href="/favicon.ico">
</head>
<body><main data-page></main></body>
</html>`
  const h = createHandler({
    routes: [
      {
        pattern: '/post/:id',
        server: {
          load({ params }) {
            if (params.id === 'err') throw new Error('boom')
            return { title: 'ok' }
          },
        },
        ssr: true,
        template: [['html', '<h1>ok</h1>']],
        shell,
      },
    ],
    service: 'web',
  })

  const ok = await h.fetch(new Request('http://x/post/1'), {})
  assert.equal(ok.status, 200)
  assert.equal(
    ok.headers.get('link'),
    '</assets/app.css>; rel=preload; as=style, </assets/theme.css>; rel=preload; as=style',
  )

  const head = await h.fetch(new Request('http://x/post/1', { method: 'HEAD' }), {})
  assert.equal(head.status, 200)
  assert.equal(head.headers.get('link'), ok.headers.get('link'))

  const err = await h.fetch(new Request('http://x/post/err'), {})
  assert.equal(err.status, 500)
  assert.equal(err.headers.get('link'), null)

  const noLinks = await get('/blog/hello')
  assert.equal(noLinks.status, 200)
  assert.equal(noLinks.headers.get('link'), null)

  assert.equal(earlyHints('<link rel="stylesheet" href="/a.css"><link rel="modulepreload" href="/b.js">'), '</a.css>; rel=preload; as=style')
})

test('platform helpers db, kv, files are passed to load and actions and undefined when absent', async (t) => {
  capture(t)
  const queries = []
  const d1 = {
    prepare(sql) {
      const make = (args) => ({
        bind: (...next) => make(next),
        first: async () => (queries.push({ op: 'first', sql, args }), { id: args[0] ?? 1 }),
        all: async () => (queries.push({ op: 'all', sql, args }), { results: [{ id: 1 }, { id: 2 }] }),
        run: async () => (queries.push({ op: 'run', sql, args }), { success: true }),
      })
      return make([])
    },
  }
  const kvStore = new Map()
  const kvBinding = {
    get: async (k) => kvStore.get(k) ?? null,
    put: async (k, v) => { kvStore.set(k, v) },
    delete: async (k) => { kvStore.delete(k) },
  }
  const fileStore = new Map()
  const r2Binding = {
    get: async (k) => fileStore.get(k) ?? null,
    put: async (k, v) => { fileStore.set(k, v) },
    delete: async (k) => { fileStore.delete(k) },
  }

  const h = createHandler({
    routes: [
      {
        pattern: '/items/:id',
        server: {
          async load({ params, env, db, kv, files }) {
            if (!db && !kv && !files) return { present: false, hasEnv: Boolean(env) }
            const row = await db.get('SELECT * FROM items WHERE id = ?', params.id)
            const rows = await db.all('SELECT * FROM items')
            await kv.put('last', params.id)
            const last = await kv.get('last')
            await files.put('note.txt', 'hello')
            const file = await files.get('note.txt')
            return { present: true, row, rows, last, file }
          },
          actions: {
            async save({ input, db, kv, files }) {
              if (!db && !kv && !files) return { present: false }
              const res = await db.run('INSERT INTO items VALUES (?)', input.name)
              await kv.delete('last')
              await files.delete('note.txt')
              return { present: true, success: res.success, last: await kv.get('last'), file: await files.get('note.txt') }
            },
          },
        },
        ssr: true,
        template: [['html', '<p>ok</p>']],
        shell: SHELL,
      },
    ],
    service: 'web',
  })

  // Absent bindings -> db, kv, files are undefined
  const absentData = await (await h.fetch(new Request('http://x/_data/items/1'), {})).json()
  assert.deepEqual(absentData, { data: { present: false, hasEnv: true } })
  const absentAction = await (await h.fetch(new Request('http://x/_action/items/1/save', { method: 'POST', body: '{}' }), {})).json()
  assert.deepEqual(absentAction, { present: false })

  // Present bindings (using BUCKET alias for files)
  const env = { DB: d1, KV: kvBinding, BUCKET: r2Binding }
  const dataRes = await (await h.fetch(new Request('http://x/_data/items/42'), env)).json()
  assert.deepEqual(dataRes, {
    data: { present: true, row: { id: '42' }, rows: [{ id: 1 }, { id: 2 }], last: '42', file: 'hello' },
  })
  const actionRes = await (await h.fetch(new Request('http://x/_action/items/42/save', { method: 'POST', body: JSON.stringify({ name: 'x' }) }), env)).json()
  assert.deepEqual(actionRes, { present: true, success: true, last: null, file: null })

  // SSR load also receives db, kv, files
  const ssrRes = await h.fetch(new Request('http://x/items/9'), { DB: d1, KV: kvBinding, FILES: r2Binding })
  assert.equal(ssrRes.status, 200)
  const ssrHtml = await ssrRes.text()
  assert.ok(ssrHtml.includes('"last":"9"'))
  assert.ok(queries.some((q) => q.op === 'run' && q.args[0] === 'x'))
})
