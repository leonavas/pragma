// Serve dist/ como um host estático com fallback de SPA para index.html.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';

const root = 'dist';
const host = process.env.HOST ?? '127.0.0.1';
const port = Number(process.env.PORT ?? 5173);
const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json' };

createServer(async (request, response) => {
  const pathname = decodeURIComponent(new URL(request.url, 'http://x').pathname);
  const file = join(root, normalize(pathname));
  const isAsset = extname(file) !== '';
  try {
    if (!file.startsWith(root)) throw new Error('fora de dist/');
    const body = await readFile(isAsset ? file : join(root, 'index.html'));
    const hashed = pathname.startsWith('/assets/');
    response.writeHead(200, {
      'content-type': types[isAsset ? extname(file) : '.html'] ?? 'application/octet-stream',
      'cache-control': hashed ? 'public, max-age=31536000, immutable' : 'no-cache',
    });
    response.end(body);
  } catch {
    response.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' });
    response.end('Não encontrado');
  }
}).listen(port, host, () => console.log(`Preview em http://${host}:${port}`));
