// Lighthouse (categoria performance, preset mobile padrão = throttling simulado) em / de cada app, N runs.
// Cada run sob flock /tmp/pragma-bench.lock. Usa o Chromium do Playwright.
import { chromium } from 'playwright';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { APPS, withLock } from './lib.mjs';
const RUNS = Number(process.argv[2] || 3);
const apps = (process.argv[3] || 'sveltekit,astra,fable').split(',');
const RAW = new URL('./raw/', import.meta.url).pathname;
mkdirSync(RAW + 'lh', { recursive: true });
const chrome = chromium.executablePath();
const bin = new URL('./node_modules/.bin/lighthouse', import.meta.url).pathname;
const version = execFileSync(bin, ['--version']).toString().trim();
const out = {};
for (let i = 0; i < RUNS; i++) {
  for (const app of apps) {
    const file = `${RAW}lh/${app}-${i}.json`;
    await withLock(`lighthouse-${app}-${i}`, async () => {
      execFileSync(bin, [APPS[app] + '/', '--only-categories=performance', '--output=json', `--output-path=${file}`, '--quiet',
        `--chrome-path=${chrome}`, '--chrome-flags=--headless=new --no-sandbox'], { stdio: ['ignore', 'ignore', 'inherit'] });
    });
    const r = JSON.parse(readFileSync(file, 'utf8'));
    const a = r.audits;
    (out[app] ||= []).push({
      score: Math.round(r.categories.performance.score * 100),
      fcp: a['first-contentful-paint'].numericValue, lcp: a['largest-contentful-paint'].numericValue,
      tbt: a['total-blocking-time'].numericValue, si: a['speed-index'].numericValue, cls: a['cumulative-layout-shift'].numericValue,
      bytes: a['total-byte-weight'].numericValue, warnings: r.runWarnings, ua: r.environment?.hostUserAgent,
    });
    console.log(app, i, out[app].at(-1).score);
  }
}
writeFileSync(RAW + 'lighthouse.json', JSON.stringify({ lighthouse: version, chrome, runs: RUNS, data: out }, null, 1));
