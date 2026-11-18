import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { extname } from 'node:path'

const runtime = new URL('../../', import.meta.url)
const fixtures = new URL('./fixtures/', import.meta.url)
const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css' }

export function serve(port = 0) {
  const server = createServer(async (req, res) => {
    const path = new URL(req.url, 'http://x').pathname
    const file = path.startsWith('/core/') ? new URL(path.slice(6), runtime) : new URL('.' + path, fixtures)
    try {
      const body = await readFile(file)
      res.writeHead(200, { 'content-type': types[extname(file.pathname)] ?? 'application/octet-stream' })
      res.end(body)
    } catch {
      res.writeHead(404).end()
    }
  })
  return new Promise(r => server.listen(port, '127.0.0.1', () => r(server)))
}

if (import.meta.url == `file://${process.argv[1]}`) {
  const s = await serve(Number(process.env.PORT ?? 5181))
  process.stdout.write(`http://127.0.0.1:${s.address().port}/index.html\n`)
}
