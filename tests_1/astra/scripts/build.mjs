import { build, context } from 'esbuild';
import { cp, mkdir, rm } from 'node:fs/promises';
const mode = process.argv[2];
const options = {
  entryPoints: ['src/app.js'], bundle: true, minify: mode !== 'dev',
  format: 'esm', target: ['es2022'], outdir: 'dist/assets',
  entryNames: 'app', legalComments: 'none', logLevel: 'info',
};
if (mode !== 'preview') {
  await rm('dist', { recursive: true, force: true });
  await mkdir('dist', { recursive: true });
  await cp('public', 'dist', { recursive: true });
  await cp('index.html', 'dist/index.html');
}
if (mode === 'dev' || mode === 'preview') {
  const ctx = await context(mode === 'dev' ? options : { stdin: { contents: '' }, write: false });
  if (mode === 'dev') await ctx.watch();
  await ctx.serve({ servedir: 'dist', fallback: 'dist/index.html', host: '127.0.0.1', port: 5172 });
  const stop = async () => { await ctx.dispose(); process.exit(0); };
  process.on('SIGINT', stop);
  process.on('SIGTERM', stop);
} else await build(options);
