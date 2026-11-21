import { test } from 'node:test'
import assert from 'node:assert/strict'
import { createHandler } from '../../server/handler.js'
import { error, redirect } from '../../server/http.js'
import { createCookies, parseCookies } from '../../server/cookies.js'

const SHELL = `<!doctype html><html><head><title>t</title></head><body><main data-page></main></body></html>`

const files = {
  '/index.html': '<h1>home</h1>',
  '/app/index.html': '<h1>app</h1>',
  '/_404.html': '<h1>missing</h1>',
  '/assets/app.css': 'body{}',
  '/_pragma/manifest.json': '[]',
}

const ASSETS = {
  fetch: async (request) => {
    const path = new URL(request.url).pathname
    const key = files[path] !== undefined ? path : files[path + '.html'] !== undefined ? path + '.html' : files[path + '/index.html'] !== undefined ? path + '/index.html' : null
    if (!key) return new Response('nope', { status: 404 })
    const type = key.endsWith('.html') ? 'text/html; charset=utf-8' : 'text/plain'
    return new Response(request.method === 'HEAD' ? null : files[key], { headers: { 'content-type': type } })
  },
}

const quiet = (t) => t.mock.method(console, 'log', () => {})

function app({ middleware, server = {}, endpoint } = {}) {
  const routes = [
    { pattern: '/account', server, ssr: false, template: null, shell: null },
    { pattern: '/report/:id', server, ssr: true, template: [['html', '<p>'], ['text', '{data.title}'], ['html', '</p>']], shell: SHELL },
  ]
  if (endpoint) routes.push({ pattern: '/files/:name', endpoint, file: 'routes/files/[name].server.js' })
  return createHandler({ routes, middleware, service: 'hooks-test' })
}

const call = (handler, path, init, ctx) => handler.fetch(new Request('http://app.test' + path, init), { ASSETS }, ctx)

test('redirect() and error() validate their arguments and throw', () => {
  assert.throws(() => redirect(303, '/login'), (e) => e.status === 303 && e.location === '/login')
  assert.throws(() => redirect(200, '/x'), /status must be 301, 302, 303, 307 or 308/)
  assert.throws(() => error(404, 'Gone'), (e) => e.status === 404 && e.body.message === 'Gone')
  assert.throws(() => error(403, { message: 'No', code: 'locked' }), (e) => e.body.code === 'locked')
  assert.throws(() => error(302, 'x'), /status must be 400-599/)
})

test('cookies: defaults, __Host- names, delete, get after set', () => {
  const request = new Request('https://app.test/', { headers: { cookie: 'sid=a%20b; theme=dark' } })
  const { cookies, setCookieLines } = createCookies(request, new URL(request.url))
  assert.equal(cookies.get('sid'), 'a b')
  cookies.set('__Host-session', 'xyz', { maxAge: 60 })
  cookies.set('theme', 'light', { httpOnly: false, sameSite: 'strict' })
  cookies.delete('sid')
  assert.equal(cookies.get('theme'), 'light')
  assert.equal(cookies.get('sid'), undefined)
  assert.deepEqual(cookies.getAll().map((c) => c.name).sort(), ['__Host-session', 'theme'])
  assert.deepEqual(setCookieLines(), [
    '__Host-session=xyz; Max-Age=60; Path=/; HttpOnly; Secure; SameSite=Lax',
    'theme=light; Path=/; Secure; SameSite=Strict',
    'sid=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax',
  ])
  assert.throws(() => cookies.set('__Host-x', '1', { domain: 'app.test' }), /cannot have a domain/)
  assert.throws(() => cookies.set('bad name', '1'), /invalid cookie name/)
  assert.deepEqual(parseCookies('a=1; a=2; b="q"'), { a: '1', b: 'q' })
})

test('cookies: plain http on localhost is not Secure, except prefixed names', () => {
  const request = new Request('http://localhost:5180/')
  const { cookies, setCookieLines } = createCookies(request, new URL(request.url))
  cookies.set('sid', '1')
  cookies.set('__Host-sid', '2')
  assert.deepEqual(setCookieLines(), ['sid=1; Path=/; HttpOnly; SameSite=Lax', '__Host-sid=2; Path=/; HttpOnly; Secure; SameSite=Lax'])
})

test('load gets cookies, locals, url, setHeaders; cookies and headers reach the /_data response', async (t) => {
  quiet(t)
  const handler = app({
    middleware: async ({ locals, next }) => {
      locals.user = { id: 7 }
      return next()
    },
    server: {
      load({ cookies, locals, url, setHeaders }) {
        cookies.set('seen', '1')
        setHeaders({ 'Cache-Control': 'private, max-age=5' })
        return { user: locals.user.id, path: url.pathname, theme: cookies.get('theme') }
      },
    },
  })
  const res = await call(handler, '/_data/account', { headers: { cookie: 'theme=dark' } })
  assert.equal(res.status, 200)
  assert.deepEqual(await res.json(), { data: { user: 7, path: '/account', theme: 'dark' } })
  assert.equal(res.headers.get('cache-control'), 'private, max-age=5')
  assert.deepEqual(res.headers.getSetCookie(), ['seen=1; Path=/; HttpOnly; Secure; SameSite=Lax'])
})

test('setHeaders rejects set-cookie', async (t) => {
  quiet(t)
  const handler = app({ server: { load: ({ setHeaders }) => setHeaders({ 'Set-Cookie': 'a=1' }) } })
  const res = await call(handler, '/_data/account')
  assert.equal(res.status, 500)
  assert.match((await res.json()).error.message, /use cookies.set/)
})

test('wire contract: redirect in load/action answers 200 {redirect}; error answers its status {error}', async (t) => {
  quiet(t)
  const handler = app({
    server: {
      load: () => redirect(303, '/login'),
      actions: {
        save: ({ cookies }) => {
          cookies.set('flash', 'saved')
          redirect(303, '/account?ok=1')
        },
        deny: () => error(403, 'Not yours'),
        raw: () => new Response(null, { status: 302, headers: { location: '/elsewhere' } }),
      },
    },
  })
  const data = await call(handler, '/_data/account')
  assert.equal(data.status, 200)
  assert.deepEqual(await data.json(), { redirect: { status: 303, location: '/login' } })

  const saved = await call(handler, '/_action/account/save', { method: 'POST' })
  assert.equal(saved.status, 200)
  assert.deepEqual(await saved.json(), { redirect: { status: 303, location: '/account?ok=1' } })
  assert.match(saved.headers.getSetCookie()[0], /^flash=saved;/)

  const denied = await call(handler, '/_action/account/deny', { method: 'POST' })
  assert.equal(denied.status, 403)
  assert.deepEqual(await denied.json(), { error: { message: 'Not yours' } })

  const raw = await call(handler, '/_action/account/raw', { method: 'POST' })
  assert.equal(raw.status, 200)
  assert.deepEqual(await raw.json(), { redirect: { status: 302, location: '/elsewhere' } })
})

test('SSR page: redirect is a real 30x with cookies; error renders the shell with its status', async (t) => {
  quiet(t)
  let mode = 'redirect'
  const handler = app({
    server: {
      load({ cookies }) {
        cookies.set('next', '/report/1')
        if (mode === 'redirect') redirect(307, '/login')
        error(410, 'Report removed')
      },
    },
  })
  const res = await call(handler, '/report/1')
  assert.equal(res.status, 307)
  assert.equal(res.headers.get('location'), '/login')
  assert.match(res.headers.getSetCookie()[0], /^next=%2Freport%2F1;/)
  mode = 'error'
  const gone = await call(handler, '/report/1')
  assert.equal(gone.status, 410)
  assert.match(await gone.text(), /"message":"Report removed"/)
})

test('middleware runs for prebuilt pages; redirect, 404 page and pass-through', async (t) => {
  quiet(t)
  const seen = []
  const handler = app({
    middleware: async ({ url, cookies, next, setHeaders }) => {
      seen.push(url.pathname)
      setHeaders({ 'x-frame-options': 'DENY' })
      if (url.pathname.startsWith('/app') && !cookies.get('sid')) redirect(303, '/login')
      if (url.pathname === '/secret') error(404)
      const res = await next()
      res.headers.set('x-mw', 'yes')
      return res
    },
  })
  const blocked = await call(handler, '/app')
  assert.equal(blocked.status, 303)
  assert.equal(blocked.headers.get('location'), '/login')
  assert.equal(blocked.headers.get('x-frame-options'), 'DENY')

  const open = await call(handler, '/app', { headers: { cookie: 'sid=1' } })
  assert.equal(open.status, 200)
  assert.equal(await open.text(), '<h1>app</h1>')
  assert.equal(open.headers.get('x-mw'), 'yes')
  assert.equal(open.headers.get('x-frame-options'), 'DENY')

  const missing = await call(handler, '/secret')
  assert.equal(missing.status, 404)
  assert.equal(await missing.text(), '<h1>missing</h1>')

  const data = await call(handler, '/_data/app/x')
  assert.deepEqual(await data.json(), { redirect: { status: 303, location: '/login' } })

  await call(handler, '/assets/app.css')
  await call(handler, '/_pragma/manifest.json')
  assert.deepEqual(seen, ['/app', '/app', '/secret', '/app/x'])
})

test('middleware returning a 30x Response for a data request becomes the JSON redirect', async (t) => {
  quiet(t)
  const handler = app({
    middleware: async () => new Response(null, { status: 302, headers: { location: '/login' } }),
    server: { load: () => ({}) },
  })
  const res = await call(handler, '/_data/account')
  assert.equal(res.status, 200)
  assert.deepEqual(await res.json(), { redirect: { status: 302, location: '/login' } })
})

test('middleware must return a Response', async (t) => {
  const lines = []
  t.mock.method(console, 'log', (line) => lines.push(JSON.parse(line)))
  const handler = app({ middleware: async () => undefined })
  const res = await call(handler, '/app')
  assert.equal(res.status, 500)
  assert.ok(lines.some((l) => /handle must return a Response/.test(l.body)))
})

test('endpoint: method handlers, params, binary body, HEAD from GET, 405 with Allow', async (t) => {
  quiet(t)
  const endpoint = {
    GET: ({ params, request }) => {
      const range = request.headers.get('range')
      const bytes = new Uint8Array([1, 2, 3, 4])
      if (range) return new Response(bytes.slice(1, 3), { status: 206, headers: { 'content-range': 'bytes 1-2/4' } })
      return new Response(bytes, { headers: { 'content-type': 'application/octet-stream', 'x-name': params.name } })
    },
    DELETE: ({ cookies }) => {
      cookies.delete('upload')
      return new Response(null, { status: 204 })
    },
  }
  const handler = app({ endpoint })
  const res = await call(handler, '/files/a%20b.pdf')
  assert.equal(res.status, 200)
  assert.equal(res.headers.get('x-name'), 'a b.pdf')
  assert.deepEqual([...new Uint8Array(await res.arrayBuffer())], [1, 2, 3, 4])

  const partial = await call(handler, '/files/x', { headers: { range: 'bytes=1-2' } })
  assert.equal(partial.status, 206)
  assert.deepEqual([...new Uint8Array(await partial.arrayBuffer())], [2, 3])

  const head = await call(handler, '/files/x', { method: 'HEAD' })
  assert.equal(head.status, 200)
  assert.equal(await head.text(), '')

  const removed = await call(handler, '/files/x', { method: 'DELETE' })
  assert.equal(removed.status, 204)
  assert.match(removed.headers.getSetCookie()[0], /^upload=; Max-Age=0/)

  const put = await call(handler, '/files/x', { method: 'PUT' })
  assert.equal(put.status, 405)
  assert.equal(put.headers.get('allow'), 'GET, HEAD, DELETE')
})

test('endpoint: non-Response return is a 500 that names the file and the fix', async (t) => {
  const lines = []
  t.mock.method(console, 'log', (line) => lines.push(JSON.parse(line)))
  const handler = app({ endpoint: { GET: () => ({ products: [] }) } })
  const res = await call(handler, '/files/x')
  assert.equal(res.status, 500)
  assert.match((await res.json()).error.message, /routes\/files\/\[name\]\.server\.js: GET returned object; an endpoint must return a Response/)
})

test('endpoint: redirect is a real 30x, error is JSON with the status', async (t) => {
  quiet(t)
  const handler = app({ endpoint: { GET: ({ params }) => (params.name === 'old' ? redirect(301, '/files/new') : error(404, 'No file')) } })
  const moved = await call(handler, '/files/old')
  assert.equal(moved.status, 301)
  assert.equal(moved.headers.get('location'), '/files/new')
  const missing = await call(handler, '/files/x')
  assert.equal(missing.status, 404)
  assert.deepEqual(await missing.json(), { error: { message: 'No file' } })
})

test('streaming bodies from actions and endpoints pass through untouched, with cookies', async (t) => {
  quiet(t)
  const stream = () =>
    new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode('{"n":1}\n'))
        controller.enqueue(new TextEncoder().encode('{"n":2}\n'))
        controller.close()
      },
    })
  const handler = app({
    server: {
      actions: {
        chat: ({ cookies }) => {
          cookies.set('turn', '2')
          return new Response(stream(), { headers: { 'content-type': 'application/x-ndjson' } })
        },
      },
    },
    endpoint: { GET: () => new Response(stream(), { headers: { 'content-type': 'application/x-ndjson' } }) },
  })
  const res = await call(handler, '/_action/account/chat', { method: 'POST' })
  assert.equal(res.headers.get('content-type'), 'application/x-ndjson')
  assert.match(res.headers.getSetCookie()[0], /^turn=2;/)
  assert.equal(await res.text(), '{"n":1}\n{"n":2}\n')
  assert.equal(await (await call(handler, '/files/x')).text(), '{"n":1}\n{"n":2}\n')
})

test('cookies apply to immutable responses (fetch results) by copying them', async (t) => {
  quiet(t)
  const frozen = Response.redirect('http://app.test/elsewhere', 302)
  const handler = app({
    endpoint: {
      GET: ({ cookies }) => {
        cookies.set('a', '1')
        return frozen
      },
    },
  })
  const res = await call(handler, '/files/x')
  assert.equal(res.status, 302)
  assert.match(res.headers.getSetCookie()[0], /^a=1;/)
})

test('waitUntil hands the promise to ctx and logs a rejection', async (t) => {
  const lines = []
  t.mock.method(console, 'log', (line) => lines.push(JSON.parse(line)))
  const waited = []
  const handler = app({
    server: {
      load({ waitUntil }) {
        waitUntil(Promise.reject(new Error('email down')))
        return {}
      },
    },
  })
  await call(handler, '/_data/account', undefined, { waitUntil: (p) => waited.push(p) })
  await Promise.all(waited)
  assert.ok(waited.length >= 1)
  assert.ok(lines.some((l) => l.body === 'waitUntil failed' && l.attributes['exception.message'] === 'email down'))
})
