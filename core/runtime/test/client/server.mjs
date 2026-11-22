import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { join, extname } from 'node:path'

const here = fileURLToPath(new URL('.', import.meta.url))
const fixtures = join(here, 'fixtures')
const runtime = join(here, '../..')
const mime = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.json': 'application/json', '.css': 'text/css' }
const wait = ms => new Promise(r => setTimeout(r, ms))

const match = (pattern, path) => {
  const a = pattern.split('/'), b = path.split('/')
  return a.length === b.length && a.every((s, i) => s[0] === ':' || s === b[i])
}

function send(res, status, body, type = 'application/json') {
  res.writeHead(status, { 'content-type': type, 'cache-control': 'no-store' })
  res.end(typeof body === 'string' || Buffer.isBuffer(body) ? body : JSON.stringify(body))
}

async function ndjson(res, name, accept) {
  res.writeHead(200, { 'content-type': 'application/x-ndjson', 'cache-control': 'no-store' })
  const chat = Buffer.from(JSON.stringify({ accept }) + '\n{"n":1}\n{"n":2,"s":"ã"}\n{"n":3}'), cut = chat.indexOf('ã') + 1
  const chunks = name === 'chat'
    ? [chat.subarray(0, chat.indexOf('{"n":2') + 3), chat.subarray(chat.indexOf('{"n":2') + 3, cut), chat.subarray(cut)]
    : Array.from({ length: 30 }, (_, i) => JSON.stringify({ n: i }) + '\n')
  for (const c of chunks) {
    if (res.destroyed) return
    res.write(c)
    await wait(150)
  }
  res.end()
}

async function data(path) {
  if (path.startsWith('/product/')) {
    await wait(300)
    const id = path.split('/')[2]
    return [200, { data: { product: { id, name: 'Product ' + id } } }]
  }
  if (path === '/error') return [500, { error: { message: 'Load went wrong' } }]
  if (path === '/gone') return [200, { redirect: { status: 303, location: '/b' } }]
  if (path === '/missing') return [404, { error: { message: 'No product', code: 'missing' } }]
  return [404, { error: { message: 'Route has no data' } }]
}

export function start() {
  const server = createServer(async (req, res) => {
    const url = new URL(req.url, 'http://x')
    const path = decodeURIComponent(url.pathname)
    try {
      if (path === '/_otel') return send(res, 204, '')
      if (path.startsWith('/_data/')) return send(res, ...(await data(path.slice(6))))
      if (path.startsWith('/_action/')) {
        let body = ''
        for await (const c of req) body += c
        const name = path.slice(path.lastIndexOf('/') + 1)
        await wait(100)
        if (name === 'fail') return send(res, 500, { error: { message: 'Failed' } })
        if (name === 'go') return send(res, 200, { redirect: { status: 303, location: '/b' } })
        if (name === 'conflict') return send(res, 409, { error: { message: 'Slot taken', field: 'day' } })
        if (name === 'plain') return send(res, 400, 'bad input', 'text/plain')
        if (name === 'chat' || name === 'slow') return ndjson(res, name, req.headers.accept)
        return send(res, 200, { name: name, route: path.slice(8, path.lastIndexOf('/')) || '/', input: JSON.parse(body) })
      }
      if (path === '/private' || path === '/private.html') {
        res.writeHead(303, { location: '/b', 'cache-control': 'no-store' })
        return res.end()
      }
      if (path === '/tidy.html') {
        res.writeHead(307, { location: '/tidy', 'cache-control': 'no-store' })
        return res.end()
      }
      let file
      if (path === '/tidy') file = join(fixtures, 'b.html')
      else if (path.startsWith('/core/')) file = join(runtime, path.slice(6))
      else if (extname(path)) file = join(fixtures, path)
      else {
        const manifest = JSON.parse(await readFile(join(fixtures, '_pragma/manifest.json'), 'utf8'))
        const e = manifest.find(e => !e.pattern.includes(':') && e.pattern === path) ?? manifest.find(e => match(e.pattern, path))
        if (!e) return send(res, 404, 'not found', 'text/plain')
        file = join(fixtures, e.html)
      }
      send(res, 200, await readFile(file), mime[extname(file)] ?? 'application/octet-stream')
    } catch {
      send(res, 404, 'not found', 'text/plain')
    }
  })
  return new Promise(r => server.listen(0, '127.0.0.1', () => r({ url: `http://127.0.0.1:${server.address().port}`, close: () => server.close() })))
}

if (import.meta.url === `file://${process.argv[1]}`) start().then(s => console.log(s.url))
