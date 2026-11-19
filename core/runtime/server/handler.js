import { render, renderHead } from './render.js'
import { createLogger, flush, fromClient, minSeverity, newSpanId, newTraceId, parseTraceparent, resourceFrom } from './log.js'

const OTEL_MAX_BYTES = 256 * 1024

const split = (path) => path.split('/').filter(Boolean)

export function compileRoutes(routes) {
  return routes
    .map((route) => {
      const segs = split(route.pattern)
      return { route, segs, rank: segs.map((s) => (s[0] === ':' ? '1' : '0')).join('') }
    })
    .sort((a, b) => (a.rank < b.rank ? -1 : a.rank > b.rank ? 1 : 0))
}

export function matchRoute(table, path) {
  let parts
  try {
    parts = split(path).map(decodeURIComponent)
  } catch {
    return null
  }
  next: for (const { route, segs } of table) {
    if (segs.length !== parts.length) continue
    const params = {}
    for (let i = 0; i < segs.length; i++) {
      if (segs[i][0] === ':') params[segs[i].slice(1)] = parts[i]
      else if (segs[i] !== parts[i]) continue next
    }
    return { route, params }
  }
  return null
}

const json = (body, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' },
  })

const fail = (message, status) => json({ error: { message } }, status)

function later(promise, ctx) {
  if (ctx?.waitUntil) ctx.waitUntil(promise)
  else return promise
}

export function createHandler({ routes, service }) {
  const table = compileRoutes(routes)
  let pages

  // Static files first; then a prebuilt param page from the manifest; then _404.html with status 404.
  async function asset(request, env) {
    const res = await env.ASSETS.fetch(request)
    if (res.status !== 404 || (request.method !== 'GET' && request.method !== 'HEAD')) return res
    const at = (path) => new URL(path.replace(/index\.html$/, '').replace(/\.html$/, ''), request.url)
    pages ??= env.ASSETS.fetch(new Request(new URL('/_pragma/manifest.json', request.url)))
      .then((r) => (r.ok ? r.json() : []))
      .then(compileRoutes)
      .catch(() => [])
    const m = matchRoute(await pages, new URL(request.url).pathname)
    if (m?.route.html && !m.route.ssr) {
      const page = await env.ASSETS.fetch(new Request(at(m.route.html), request))
      if (page.ok) return page
    }
    const missing = await env.ASSETS.fetch(new Request(at('/_404.html'), request))
    if (!missing.ok) return res
    return new Response(request.method === 'HEAD' ? null : missing.body, { status: 404, headers: missing.headers })
  }

  return {
    async fetch(request, env = {}, ctx) {
      const url = new URL(request.url)
      const path = url.pathname
      if (path === '/_otel') return ingest(request, env, ctx)

      let kind = null
      let target = path
      let actionName
      if (path === '/_data' || path.startsWith('/_data/')) {
        kind = 'data'
        target = path.slice(6) || '/'
      } else if (path.startsWith('/_action/')) {
        kind = 'action'
        const rest = path.slice(8)
        const i = rest.lastIndexOf('/')
        target = rest.slice(0, i) || '/'
        actionName = rest.slice(i + 1)
      } else if (request.method === 'GET' || request.method === 'HEAD') {
        kind = 'page'
      }
      const m = matchRoute(table, target)
      // The router fetches prebuilt HTML (/shop/index.html): that is a file, not an SSR page.
      if (kind === 'page' && (path.endsWith('.html') || !(m?.route.ssr && m.route.template && m.route.shell))) kind = null
      if (!kind) return env.ASSETS ? asset(request, env) : new Response('Not found', { status: 404 })

      const start = performance.now()
      const parent = parseTraceparent(request.headers.get('traceparent'))
      const log = createLogger({
        resource: resourceFrom(env, service),
        traceId: parent?.traceId ?? newTraceId(),
        spanId: newSpanId(),
        min: minSeverity(env),
      })
      const query = Object.fromEntries(url.searchParams)

      let res
      try {
        if (kind === 'data') res = await data(m, { query, request, env, log })
        else if (kind === 'action') res = await action(m, actionName, { query, request, env, log })
        else res = await page(m, { query, request, env, log })
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err)
        log.error(message, err)
        res = fail(message, 500)
      }

      const status = res.status
      const attrs = {
        'http.request.method': request.method,
        'url.path': path,
        'http.response.status_code': status,
        'http.server.request.duration': (performance.now() - start) / 1000,
      }
      if (m) attrs['http.route'] = m.route.pattern
      if (status >= 500) attrs['error.type'] = String(status)
      log[status >= 500 ? 'error' : 'info'](`${request.method} ${m?.route.pattern ?? path}`, attrs)
      await later(flush(log.records, env), ctx)
      return res
    },
  }
}

async function data(m, { query, request, env, log }) {
  if (request.method !== 'GET' && request.method !== 'HEAD') return fail('Method not allowed', 405)
  const load = m?.route.server?.load
  if (!load) return fail('Route has no data', 404)
  return json({ data: (await load({ params: m.params, query, request, env, log })) ?? null })
}

async function action(m, name, { query, request, env, log }) {
  if (request.method !== 'POST') return fail('Method not allowed', 405)
  let decoded
  try {
    decoded = decodeURIComponent(name)
  } catch {
    return fail('Unknown action', 404)
  }
  const actions = m?.route.server?.actions
  if (!actions || !Object.hasOwn(actions, decoded) || typeof actions[decoded] !== 'function') {
    return fail(`Unknown action: ${decoded}`, 404)
  }
  let input = null
  const text = await request.text()
  if (text) {
    try {
      input = JSON.parse(text)
    } catch {
      return fail('Action body is not valid JSON', 400)
    }
  }
  const result = await actions[decoded]({ input, params: m.params, query, request, env, log })
  return result instanceof Response ? result : json(result ?? null)
}

async function page(m, { query, request, env, log }) {
  const { route, params } = m
  let payload
  let body
  let status = 200
  const scope = { data: null, params, query, page: { loading: false, error: null } }
  try {
    const value = route.server?.load ? await route.server.load({ params, query, request, env, log }) : null
    payload = { data: value ?? null }
    scope.data = payload.data
    body = render(route.template, scope)
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err)
    log.error(message, err)
    payload = { error: { message } }
    body = undefined
    status = 500
  }
  return new Response(request.method === 'HEAD' ? null : shellWith(route.shell, body, payload, scope), {
    status,
    headers: { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'private, no-cache' },
  })
}

export function shellWith(shell, body, payload, scope) {
  let html = shell.replace(/<script>self\.__pd=[^<]*<\/script>\s*/, '')
  if (scope) html = renderHead(html, scope)
  if (body !== undefined) {
    const open = /<main\b[^>]*\bdata-page\b[^>]*>/.exec(html)
    const end = open ? html.indexOf('</main>', open.index) : -1
    if (end < 0) throw new Error('shell has no <main data-page>…</main>')
    html = html.slice(0, open.index + open[0].length) + body + html.slice(end)
  }
  const script = `<script type="application/json" id="pragma-data">${JSON.stringify(payload).replace(/</g, '\\u003c')}</script>`
  const i = html.lastIndexOf('</body>')
  return i < 0 ? html + script : html.slice(0, i) + script + html.slice(i)
}

async function ingest(request, env, ctx) {
  if (request.method !== 'POST') return fail('Method not allowed', 405)
  if (Number(request.headers.get('content-length') ?? 0) > OTEL_MAX_BYTES) return fail('Batch too large', 413)
  const text = await request.text()
  if (text.length > OTEL_MAX_BYTES) return fail('Batch too large', 413)
  let records
  try {
    records = fromClient(JSON.parse(text))
  } catch {
    return fail('Invalid log batch', 400)
  }
  await later(flush(records, env), ctx)
  return new Response(null, { status: 204 })
}
