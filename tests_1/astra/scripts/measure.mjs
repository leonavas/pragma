import { readdir, readFile, writeFile } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
const run = promisify(execFile);
async function files(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const nested = await Promise.all(entries.map(e => e.isDirectory() ? files(`${dir}/${e.name}`) : [`${dir}/${e.name}`]));
  return nested.flat();
}
const source = (await files('src')).filter(f => f !== 'src/preflight.css');
source.push('index.html');
const lines = {};
for (const extension of ['js', 'svelte', 'css', 'html']) {
  lines[extension] = 0;
  for (const file of source.filter(f => f.endsWith(`.${extension}`))) lines[extension] += (await readFile(file, 'utf8')).split('\n').length - 1;
}
let runtimeLines = 0;
for (const file of source.filter(f => f.startsWith('src/lib/'))) runtimeLines += (await readFile(file, 'utf8')).split('\n').length - 1;
const assets = [];
for (const file of (await files('dist')).filter(f => /\.(js|css)$/.test(f))) {
  assets.push({ file, bytes: (await readFile(file)).length, gzip: (await run('gzip', ['-9', '-c', file], { encoding: 'buffer' })).stdout.length });
}
const sum = ext => assets.filter(a => a.file.endsWith(ext)).reduce((n, a) => n + a.gzip, 0);
const result = { assets, jsGzip: sum('.js'), firstVisitJsGzip: sum('.js'), cssGzip: sum('.css'), lines, runtimeLines, appJsExcludingRuntime: lines.js - runtimeLines };
await writeFile('evidence/measures.json', JSON.stringify(result, null, 2));
console.log(JSON.stringify(result, null, 2));
