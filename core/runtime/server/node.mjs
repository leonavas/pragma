import { createServer } from 'node:http'
import nodeModule from 'node:module'
import { readFileSync } from 'node:fs'
import { readFile, stat } from 'node:fs/promises'
import { extname, join, resolve, sep } from 'node:path'
import { Readable } from 'node:stream'
import { pipeline } from 'node:stream/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { ASSETS, IMMUTABLE } from '../contract.js'
import { createLogger, minSeverity, resourceFrom, writeLine } from './log.js'

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
  '.md': 'text/markdown; charset=utf-8',
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

async function statFile(path) {
  try {
    const info = await stat(path)
    return info.isFile() ? info : null
  } catch {
    return null
  }
}

function acceptsEncoding(header, encoding) {
  if (!header) return false
  for (const part of header.split(',')) {
    const [name, ...params] = part.split(';').map((s) => s.trim().toLowerCase())
    if (name !== encoding) continue
    const q = params.find((p) => p.startsWith('q='))
    if (q && Number(q.slice(2)) === 0) continue
    return true
  }
  return false
}

function createStatic(root) {
  const dir = resolve(toPath(root))

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
      if (file && (await statFile(file))) return { file, rel: c }
    }
    return null
  }

  return {
    async fetch(request) {
      if (request.method !== 'GET' && request.method !== 'HEAD') {
        return new Response('Method not allowed', { status: 405, headers: { allow: 'GET, HEAD' } })
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
        if (!(await statFile(file))) return new Response('Not found', { status, headers: { 'content-type': 'text/plain; charset=utf-8' } })
      }
      const contentType = MIME[extname(file).toLowerCase()] ?? 'application/octet-stream'
      let servedFile = file
      let encoding = null
      let info = null
      let hasCompressed = false
      if (!file.endsWith('.br') && !file.endsWith('.gz')) {
        const acceptEncoding = request.headers.get('accept-encoding')
        const brInfo = await statFile(file + '.br')
        if (brInfo) hasCompressed = true
        if (brInfo && acceptsEncoding(acceptEncoding, 'br')) {
          servedFile = file + '.br'
          encoding = 'br'
          info = brInfo
        } else {
          const gzInfo = await statFile(file + '.gz')
          if (gzInfo) hasCompressed = true
          if (gzInfo && acceptsEncoding(acceptEncoding, 'gzip')) {
            servedFile = file + '.gz'
            encoding = 'gzip'
            info = gzInfo
          }
        }
      }
      if (!info) info = await stat(file)
      const etag = `W/"${info.size.toString(16)}-${Math.floor(info.mtimeMs).toString(16)}${encoding ? `-${encoding}` : ''}"`
      const headers = {
        'content-type': contentType,
        'cache-control': rel.startsWith(ASSETS) ? IMMUTABLE : 'no-cache',
        'x-content-type-options': 'nosniff',
      }
      if (hasCompressed) headers.vary = 'accept-encoding'
      if (encoding) headers['content-encoding'] = encoding
      if (status === 200) {
        headers.etag = etag
        if (request.headers.get('if-none-match') === etag) return new Response(null, { status: 304, headers })
      }
      headers['content-length'] = String(info.size)
      return new Response(request.method === 'HEAD' ? null : await readFile(servedFile), { status, headers })
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

const wasmSource = (url) => `import { readFileSync } from 'node:fs'\nexport default new WebAssembly.Module(readFileSync(new URL(${JSON.stringify(url)})))`

function cloudflareStubSource(url, readFileSync) {
  const [specifier, parent] = url.slice('pragma-cloudflare:'.length).split('?')
  const parentUrl = decodeURIComponent(parent ?? '')
  const importer = parentUrl.startsWith('file:') ? readFileSync(new URL(parentUrl), 'utf8') : ''
  const names = new Set()
  for (const [, clause] of importer.matchAll(new RegExp(`import\\s*([^'"]*?)\\s*from\\s*['"]${specifier}['"]`, 'g'))) {
    const named = clause.match(/\{([^}]*)\}/)
    for (const part of named ? named[1].split(',') : []) names.add(part.trim().split(/\s+as\s+/)[0])
  }
  names.delete('')
  names.delete('default')
  const message = `${specifier} exists only on Cloudflare Workers, not on Node: read bindings from the env that load and actions receive (pragma dev --cf gives them in dev)`
  const lines = [
    `const message = ${JSON.stringify(message)}`,
    'const fail = () => { throw new Error(message) }',
    "const unavailable = () => new Proxy(function () {}, { get: (target, key) => (key === 'prototype' ? target.prototype : fail()), apply: fail, construct: fail })",
    'export default unavailable()',
  ]
  for (const name of names) {
    if (specifier === 'cloudflare:workers' && name === 'env') {
      lines.push("export const env = new Proxy({}, { get: (target, key) => (globalThis[Symbol.for('pragma.env')] ?? process.env)[key], has: (target, key) => key in (globalThis[Symbol.for('pragma.env')] ?? process.env) })")
    } else {
      lines.push(`export const ${name} = unavailable()`)
    }
  }
  return lines.join('\n')
}

function resolveCloudflare(specifier, context, nextResolve) {
  if (!specifier.startsWith('cloudflare:')) return nextResolve(specifier, context)
  return { url: 'pragma-cloudflare:' + specifier + '?' + encodeURIComponent(context.parentURL ?? ''), shortCircuit: true }
}

function loadNodeModule(url, context, nextLoad) {
  if (url.startsWith('pragma-cloudflare:')) return { format: 'module', source: cloudflareStubSource(url, readFileSync), shortCircuit: true }
  if (!url.startsWith('file:') || !url.endsWith('.wasm')) return nextLoad(url, context)
  return { format: 'module', source: wasmSource(url), shortCircuit: true }
}

export function registerNodeHooks() {
  if (nodeModule.registerHooks) return nodeModule.registerHooks({ resolve: resolveCloudflare, load: loadNodeModule })
  const hooks = [
    "import { readFileSync } from 'node:fs'",
    `const wasmSource = ${wasmSource}`,
    `${cloudflareStubSource}`,
    `export ${resolveCloudflare}`,
    `export ${loadNodeModule}`,
  ]
  nodeModule.register('data:text/javascript,' + encodeURIComponent(hooks.join('\n')))
}

async function cloudflarePlatform(configPath) {
  let wrangler
  try {
    const require = nodeModule.createRequire(join(process.cwd(), 'package.json'))
    wrangler = await import(pathToFileURL(require.resolve('wrangler')).href)
  } catch {
    throw new Error('pragma dev --cf needs wrangler in the app: run `npm i -D wrangler@4`')
  }
  const getPlatformProxy = wrangler.getPlatformProxy ?? wrangler.default?.getPlatformProxy
  return getPlatformProxy({ configPath, remoteBindings: true })
}

export function serve(handler, { root, port = 3000, service } = {}) {
  const log = createLogger({ resource: resourceFrom(process.env, service), min: minSeverity(process.env), sink: writeLine })
  const assets = createStatic(root)
  const env = { ...process.env, ASSETS: assets }
  globalThis[Symbol.for('pragma.env')] = env
  let ctx = { waitUntil: (p) => Promise.resolve(p).catch((err) => log.error('waitUntil failed', err)) }
  const cfConfig = process.env.PRAGMA_CF_CONFIG
  const ready = cfConfig
    ? cloudflarePlatform(cfConfig).then((proxy) => {
        Object.assign(env, proxy.env, { ASSETS: assets })
        ctx = proxy.ctx
        Object.defineProperty(globalThis, 'caches', { value: proxy.caches, writable: true, enumerable: true, configurable: true })
        process.once('SIGTERM', () => proxy.dispose().finally(() => process.exit(0)))
      })
    : Promise.resolve()
  const server = createServer(async (req, res) => {
    try {
      await send(res, await handler.fetch(toRequest(req), env, ctx), req.method)
    } catch (err) {
      log.error(err instanceof Error ? err.message : String(err), err)
      if (!res.headersSent) res.writeHead(500, { 'content-type': 'text/plain; charset=utf-8' })
      res.end('Internal error')
    }
  })
  ready.then(
    () =>
      server.listen(Number(port), () => {
        log.info(`pragma serving at http://localhost:${server.address().port}`, { 'server.port': server.address().port })
      }),
    (err) => {
      log.error(err instanceof Error ? err.message : String(err), err)
      process.exitCode = 1
    },
  )
  return server
}
