import { createServer } from 'node:http'
import { readFile, stat } from 'node:fs/promises'
import { extname, join, resolve, sep } from 'node:path'
import { Readable } from 'node:stream'
import { pipeline } from 'node:stream/promises'
import { fileURLToPath } from 'node:url'
import { compileRoutes, matchRoute } from './handler.js'
import { log } from './log.js'

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
  '.xml': 'application/xml; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.webp': 'image/webp',
  '.avif': 'image/avif',
  '.ico': 'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.otf': 'font/otf',
  '.wasm': 'application/wasm',
  '.pdf': 'application/pdf',
  '.mp4': 'video/mp4',
  '.webm': 'video/webm',
  '.mp3': 'audio/mpeg',
}

const toPath = (root) => (root instanceof URL || String(root).startsWith('file:') ? fileURLToPath(root) : String(root))

async function isFile(path) {
  try {
    return (await stat(path)).isFile()
  } catch {
    return false
  }
}

export function createStatic(root) {
  const dir = resolve(toPath(root))
  let manifest

  const routes = async () => {
    if (!manifest) {
      try {
        manifest = compileRoutes(JSON.parse(await readFile(join(dir, '_pragma', 'manifest.json'), 'utf8')))
      } catch {
        manifest = []
      }
    }
    return manifest
  }

  const inside = (rel) => {
    const file = resolve(dir, '.' + rel)
    return file === dir || file.startsWith(dir + sep) ? file : null
  }

  async function find(pathname) {
    let rel
    try {
      rel = decodeURIComponent(pathname)
    } catch {
      return null
    }
    if (rel.includes('\0')) return null
    const candidates = rel.endsWith('/') ? [rel + 'index.html'] : [rel, rel + '.html', rel + '/index.html']
    for (const c of candidates) {
      const file = inside(c)
      if (file && (await isFile(file))) return { file, rel: c }
    }
    const m = matchRoute(await routes(), pathname)
    if (m?.route.html) {
      const file = inside(m.route.html)
      if (file && (await isFile(file))) return { file, rel: m.route.html }
    }
    return null
  }

  return {
    async fetch(request) {
      if (request.method !== 'GET' && request.method !== 'HEAD') {
        return new Response('Método não permitido', { status: 405, headers: { allow: 'GET, HEAD' } })
      }
      const { pathname } = new URL(request.url)
      const found = await find(pathname)
      let status = 200
      let file = found?.file
      let rel = found?.rel
      if (!found) {
        status = 404
        file = join(dir, '_404.html')
        rel = '/_404.html'
        if (!(await isFile(file))) return new Response('Não encontrado', { status, headers: { 'content-type': 'text/plain; charset=utf-8' } })
      }
      const info = await stat(file)
      const etag = `W/"${info.size.toString(16)}-${Math.floor(info.mtimeMs).toString(16)}"`
      const headers = {
        'content-type': MIME[extname(file).toLowerCase()] ?? 'application/octet-stream',
        'cache-control': rel.startsWith('/assets/') ? 'public, max-age=31536000, immutable' : 'no-cache',
        'x-content-type-options': 'nosniff',
      }
      if (status === 200) {
        headers.etag = etag
        if (request.headers.get('if-none-match') === etag) return new Response(null, { status: 304, headers })
      }
      headers['content-length'] = String(info.size)
      return new Response(request.method === 'HEAD' ? null : await readFile(file), { status, headers })
    },
  }
}

function toRequest(req) {
  const url = `http://${req.headers.host ?? 'localhost'}${req.url}`
  const headers = new Headers()
  for (const [k, v] of Object.entries(req.headers)) {
    if (k.startsWith(':')) continue
    for (const one of Array.isArray(v) ? v : [v]) headers.append(k, one)
  }
  const init = { method: req.method, headers }
  if (req.method !== 'GET' && req.method !== 'HEAD') {
    init.body = Readable.toWeb(req)
    init.duplex = 'half'
  }
  return new Request(url, init)
}

async function send(res, response, method) {
  const headers = {}
  for (const [k, v] of response.headers) if (k !== 'set-cookie') headers[k] = v
  const cookies = response.headers.getSetCookie()
  if (cookies.length) headers['set-cookie'] = cookies
  res.writeHead(response.status, headers)
  if (method === 'HEAD' || !response.body) return res.end()
  await pipeline(Readable.fromWeb(response.body), res)
}

export function serve(handler, { root, port = 3000 } = {}) {
  const assets = createStatic(root)
  const env = { ...process.env, ASSETS: assets }
  const ctx = {
    waitUntil: (p) => Promise.resolve(p).catch((err) => log.error('waitUntil falhou', err)),
    passThroughOnException() {},
  }
  const server = createServer(async (req, res) => {
    try {
      await send(res, await handler.fetch(toRequest(req), env, ctx), req.method)
    } catch (err) {
      log.error(err instanceof Error ? err.message : String(err), err)
      if (!res.headersSent) res.writeHead(500, { 'content-type': 'text/plain; charset=utf-8' })
      res.end('Erro interno')
    }
  })
  server.listen(Number(port), () => {
    log.info(`pragma servindo em http://localhost:${server.address().port}`, { 'server.port': server.address().port })
  })
  return server
}
