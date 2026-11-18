// Build de produção (dist/) e servidor de dev (--dev), só com esbuild.
import * as esbuild from 'esbuild';
import { cp, readFile, rm, writeFile } from 'node:fs/promises';

const dev = process.argv.includes('--dev');
const outdir = dev ? '.dev' : 'dist';
const host = process.env.HOST ?? '127.0.0.1';
const port = Number(process.env.PORT ?? 5173);

// index.html aponta para /assets/app.js e /assets/app.css; no build de
// produção os nomes ganham hash de conteúdo e o HTML é reescrito.
const writeIndex = {
  name: 'write-index',
  setup(build) {
    build.onEnd(async (result) => {
      if (result.errors.length) return;
      let html = await readFile('index.html', 'utf8');
      for (const file of Object.keys(result.metafile.outputs)) {
        const url = file.slice(outdir.length);
        if (url.endsWith('.js')) html = html.replace('/assets/app.js', url);
        if (url.endsWith('.css')) html = html.replace('/assets/app.css', url);
      }
      await writeFile(`${outdir}/index.html`, html);
    });
  },
};

const options = {
  entryPoints: [
    { in: 'src/main.js', out: 'app' },
    { in: 'src/styles/app.css', out: 'app' },
  ],
  outdir,
  entryNames: dev ? 'assets/[name]' : 'assets/[name]-[hash]',
  bundle: true,
  format: 'esm',
  target: 'es2022',
  minify: !dev,
  sourcemap: dev,
  metafile: true,
  legalComments: 'none',
  logLevel: 'info',
  plugins: [writeIndex],
};

await rm(outdir, { recursive: true, force: true });
await cp('public', outdir, { recursive: true });

if (dev) {
  const context = await esbuild.context(options);
  await context.watch();
  await context.serve({ host, port, servedir: outdir, fallback: `${outdir}/index.html` });
  // Encerra o processo do esbuild junto com este (Ctrl+C ou kill).
  for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => context.dispose().finally(() => process.exit(0)));
} else {
  await esbuild.build(options);
}
