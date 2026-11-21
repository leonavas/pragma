import { renderPage, renderHead } from './render.js'
import { createLogger, flush, fromClient, minSeverity, newSpanId, newTraceId, parseTraceparent, resourceFrom } from './log.js'
import { createPlatform } from './platform.js'
import { createCookies } from './cookies.js'
import { HttpError, Redirect } from './http.js'

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

export function prefersMarkdown(accept) {
  let markdown = 0
  let html = 0
  for (const part of (accept ?? '').split(',')) {
    const [name, ...params] = part.split(';').map((s) => s.trim().toLowerCase())
    const q = params.find((p) => p.startsWith('q='))
    const value = q ? Number(q.slice(2)) || 0 : 1
    if (name === 'text/markdown') markdown = Math.max(markdown, value)
    else if (name === 'text/html') html = Math.max(html, value)
  }
  return markdown > 0 && markdown >= html
}

function withVary(res) {
  const out = new Response(res.body, res)
  out.headers.append('vary', 'Accept')
  return out
}

function asMarkdown(res) {
  if (res.status !== 200) return res
  const out = new Response(res.body, res)
  out.headers.set('content-type', 'text/markdown; charset=utf-8')
  return out
}

async function findMarkdown(path, request, env) {
  for (const file of path ? [`${path}.md`, `${path}/index.md`] : ['/index.md']) {
    const res = await env.ASSETS.fetch(new Request(new URL(file, request.url), { method: 'HEAD' }))
    if (res.ok && !res.headers.get('content-type')?.startsWith('text/html')) return file
  }
  return null
}

function later(promise, ctx) {
  if (ctx?.waitUntil) ctx.waitUntil(promise)
  else return promise
}

export function createHandler({ routes, middleware, service }) {
  const table = compileRoutes(routes)
  const markdownFiles = new Map()
  let pages

  function markdownFor(path, request, env) {
    const key = path.replace(/\/+$/, '')
    let found = markdownFiles.get(key)
    if (!found) {
      if (markdownFiles.size >= 1000) markdownFiles.clear()
      found = findMarkdown(key, request, env).catch(() => null)
      markdownFiles.set(key, found)
    }
    return found
  }

  async function notFoundPage(request, env) {
    if (!env.ASSETS) return null
    const missing = await env.ASSETS.fetch(new Request(new URL('/_404', request.url), { headers: request.headers }))
    if (!missing.ok) return null
    return new Response(request.method === 'HEAD' ? null : missing.body, { status: 404, headers: missing.headers })
  }

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
    return (await notFoundPage(request, env)) ?? res
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
      }
      const m = matchRoute(table, target)
      if (!kind && m?.route.endpoint) kind = 'endpoint'
      else if (!kind && (request.method === 'GET' || request.method === 'HEAD')) kind = 'page'
      const md = kind === 'page' && env.ASSETS && !/\.[^/]*$/.test(path) ? await markdownFor(path, request, env) : null
      // The router fetches prebuilt HTML (/shop/index.html): that is a file, not an SSR page.
      if (kind === 'page' && (path.endsWith('.html') || !(m?.route.ssr && m.route.template && m.route.shell))) kind = null
      const guarded = Boolean(middleware) && !path.startsWith('/assets/') && !path.startsWith('/_pragma/')
      if (!kind && !guarded) return dispatch()

      const api = kind === 'data' || kind === 'action'
      const start = performance.now()
      const parent = parseTraceparent(request.headers.get('traceparent'))
      const log = createLogger({
        resource: resourceFrom(env, service),
        traceId: parent?.traceId ?? newTraceId(),
        spanId: newSpanId(),
        min: minSeverity(env),
      })
      const pageUrl = new URL(url.origin + target + url.search)
      const { cookies, setCookieLines } = createCookies(request, pageUrl)
      const headers = new Map()
      const event = {
        request,
        url: pageUrl,
        params: m?.params ?? {},
        query: Object.fromEntries(url.searchParams),
        env,
        log,
        cookies,
        locals: {},
        ...createPlatform(env),
        setHeaders(values) {
          for (const [key, value] of Object.entries(values ?? {})) {
            const name = key.toLowerCase()
            if (name === 'set-cookie') throw new Error('setHeaders cannot set set-cookie; use cookies.set(name, value, options)')
            headers.set(name, String(value))
          }
        },
        waitUntil(promise) {
          const tracked = Promise.resolve(promise)
            .catch((err) => log.error('waitUntil failed', err))
            .then(() => flush(log.records.splice(0), env))
          if (ctx?.waitUntil) ctx.waitUntil(tracked)
        },
      }

      async function serveFile() {
        if (!env.ASSETS) return new Response('Not found', { status: 404 })
        const res = await asset(request, env)
        if (path.endsWith('.md')) return asMarkdown(res)
        return md ? withVary(res) : res
      }

      async function dispatch() {
        if (md && prefersMarkdown(request.headers.get('accept'))) {
          return withVary(asMarkdown(await env.ASSETS.fetch(new Request(new URL(md, request.url), request))))
        }
        if (kind === 'data') return data(m, event)
        if (kind === 'action') return action(m, actionName, event)
        if (kind === 'endpoint') return endpoint(m, event)
        if (kind === 'page') {
          const res = await page(m, event)
          return md ? withVary(res) : res
        }
        return serveFile()
      }

      async function failure(err) {
        if (err instanceof Redirect) {
          if (api) return json({ redirect: { status: err.status, location: err.location } })
          return new Response(null, { status: err.status, headers: { location: err.location } })
        }
        if (err instanceof HttpError) {
          if (api || kind === 'endpoint') return json({ error: err.body }, err.status)
          if (err.status === 404) {
            const missing = await notFoundPage(request, env)
            if (missing) return missing
          }
          return new Response(request.method === 'HEAD' ? null : err.body.message, { status: err.status, headers: { 'content-type': 'text/plain; charset=utf-8' } })
        }
        const message = err instanceof Error ? err.message : String(err)
        log.error(message, err)
        if (api || kind === 'endpoint') return fail(message, 500)
        return new Response('Internal error', { status: 500, headers: { 'content-type': 'text/plain; charset=utf-8' } })
      }

      const resolve = () => dispatch().catch(failure)
      let res
      if (guarded) {
        let pending
        try {
          res = await middleware({ ...event, next: () => (pending ??= resolve()) })
          if (!(res instanceof Response)) throw new Error('routes/_middleware.server.js: handle must return a Response; return next() or a Response')
        } catch (err) {
          res = await failure(err)
        }
      } else {
        res = await resolve()
      }
      res = finish(res, { api, headers, cookies: setCookieLines() })

      const status = res.status
      if (kind) {
        const attrs = {
          'http.request.method': request.method,
          'url.path': path,
          'http.response.status_code': status,
          'http.server.request.duration': (performance.now() - start) / 1000,
        }
        if (m) attrs['http.route'] = m.route.pattern
        if (status >= 500) attrs['error.type'] = String(status)
        log[status >= 500 ? 'error' : 'info'](`${request.method} ${m?.route.pattern ?? path}`, attrs)
      }
      await later(flush(log.records.splice(0), env), ctx)
      return res
    },
  }
}

function finish(res, { api, headers, cookies }) {
  const location = res.headers.get('location')
  if (api && res.status >= 300 && res.status < 400 && location) {
    res = json({ redirect: { status: res.status, location } })
  }
  if (!headers.size && !cookies.length) return res
  const apply = (target) => {
    for (const [name, value] of headers) target.headers.set(name, value)
    for (const line of cookies) target.headers.append('set-cookie', line)
    return target
  }
  try {
    return apply(res)
  } catch {
    try {
      return apply(new Response(res.body, { status: res.status, statusText: res.statusText, headers: res.headers, webSocket: res.webSocket }))
    } catch {
      return res
    }
  }
}

async function data(m, event) {
  const { request } = event
  if (request.method !== 'GET' && request.method !== 'HEAD') return fail('Method not allowed', 405)
  const load = m?.route.server?.load
  if (!load) return fail('Route has no data', 404)
  return json({ data: (await load(event)) ?? null })
}

async function action(m, name, event) {
  const { request } = event
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
  const result = await actions[decoded]({ ...event, input })
  return result instanceof Response ? result : json(result ?? null)
}

const METHODS = ['GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS']

async function endpoint(m, event) {
  const { route } = m
  const method = event.request.method
  const handlers = route.endpoint
  const headFromGet = method === 'HEAD' && typeof handlers.HEAD !== 'function' && typeof handlers.GET === 'function'
  const handle = headFromGet ? handlers.GET : METHODS.includes(method) && handlers[method]
  if (typeof handle !== 'function') {
    const allow = METHODS.filter((name) => typeof handlers[name] === 'function' || (name === 'HEAD' && typeof handlers.GET === 'function'))
    return new Response('Method not allowed', { status: 405, headers: { allow: allow.join(', '), 'content-type': 'text/plain; charset=utf-8' } })
  }
  const res = await handle(event)
  if (!(res instanceof Response)) {
    const got = res === null ? 'null' : Array.isArray(res) ? 'an array' : typeof res
    throw new Error(`${route.file ?? route.pattern}: ${method} returned ${got}; an endpoint must return a Response. Page data comes from load, writes from actions (.pragma/docs/data.md)`)
  }
  return headFromGet ? new Response(null, res) : res
}

export function earlyHints(html) {
  if (!html) return null
  const seen = new Set()
  const hints = []
  for (const match of html.matchAll(/<link\b([^>]*)>/gi)) {
    const attrs = match[1]
    const rel = /\brel\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+))/i.exec(attrs)
    const href = /\bhref\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+))/i.exec(attrs)
    const relVal = (rel?.[1] ?? rel?.[2] ?? rel?.[3] ?? '').trim().toLowerCase()
    const hrefVal = (href?.[1] ?? href?.[2] ?? href?.[3] ?? '').trim()
    if (!hrefVal || /[>\r\n]/.test(hrefVal)) continue
    if (relVal !== 'stylesheet') continue
    const hint = `<${hrefVal}>; rel=preload; as=style`
    if (seen.has(hint)) continue
    seen.add(hint)
    hints.push(hint)
  }
  return hints.length ? hints.join(', ') : null
}

async function page(m, event) {
  const { route, params } = m
  const { request, query, log } = event
  let payload
  let body
  let status = 200
  const scope = { data: null, params, query, page: { loading: false, error: null } }
  try {
    const value = route.server?.load ? await route.server.load(event) : null
    payload = { data: value ?? null }
    scope.data = payload.data
    body = renderPage(route.template, scope, { page: route.page, components: route.components, log })
  } catch (err) {
    if (err instanceof Redirect) throw err
    if (err instanceof HttpError) {
      payload = { error: { ...err.body, status: err.status } }
      status = err.status
    } else {
      const message = err instanceof Error ? err.message : String(err)
      log.error(message, err)
      payload = { error: { message, status: 500 } }
      status = 500
    }
    body = undefined
  }
  const html = shellWith(route.shell, body, payload, scope)
  const headers = new Headers({ 'content-type': 'text/html; charset=utf-8', 'cache-control': 'private, no-cache' })
  if (status === 200 && !headers.has('link')) {
    const link = earlyHints(html)
    if (link) headers.set('link', link)
  }
  return new Response(request.method === 'HEAD' ? null : html, { status, headers })
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
