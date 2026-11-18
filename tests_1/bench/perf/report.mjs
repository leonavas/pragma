// Agrega raw/*.json em ../perf.json (brutos + medianas/p90) e gera as tabelas de ../perf.md.
import { readFileSync, readdirSync, writeFileSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { median, p90, PROFILES } from './lib.mjs';

const HERE = new URL('./', import.meta.url).pathname;
const RAW = HERE + 'raw/';
const APPS = ['sveltekit', 'astra', 'fable'];
const raw = {};
let chromiumVersion = null;
for (const f of readdirSync(RAW).filter((f) => f.endsWith('.json') && f !== 'lighthouse.json')) {
  const d = JSON.parse(readFileSync(RAW + f, 'utf8'));
  chromiumVersion = d.chromium;
  ((raw[d.measure] ||= {})[d.profile] ||= {})[d.app] = d.data;
}
const lh = existsSync(RAW + 'lighthouse.json') ? JSON.parse(readFileSync(RAW + 'lighthouse.json', 'utf8')) : null;
const pkg = (n) => JSON.parse(readFileSync(HERE + `node_modules/${n}/package.json`, 'utf8')).version;

// Métricas: [chave, rótulo, extrator(run) -> número, unidade]
const F = (k) => (r) => r[k];
const FR = (k) => (r) => r[k]?.frame;
const METRICS = {
  load: (path) => [
    [`${path} FCP`, (r) => r.fcp, 'ms'], [`${path} LCP`, (r) => r.lcp, 'ms'],
    [`${path} DOMContentLoaded`, (r) => r.dcl, 'ms'], [`${path} load`, (r) => r.load, 'ms'],
    [`${path} bytes transferidos`, (r) => r.bytes / 1024, 'KB'], [`${path} bytes decodificados`, (r) => r.decodedBytes / 1024, 'KB'],
    [`${path} requisições`, (r) => r.requests, ''], [`${path} long tasks (nº)`, (r) => r.longTasks, ''],
    [`${path} long tasks (ms somados)`, (r) => r.longTaskMs, 'ms'], [`${path} TBT (após FCP)`, (r) => r.tbt, 'ms'],
  ],
  tti: [
    ['TTI prático (nav → valor do + atualizado e pintado)', F('tti'), 'ms'],
    ['1º clique possível (botão com caixa)', F('firstClickAt'), 'ms'],
    ['cliques necessários', F('clicks'), ''],
  ],
  nav: [
    ['Painel → Tarefas', FR('toTarefas'), 'ms'],
    ['Tarefas → Loja (conteúdo/h1)', FR('toLoja'), 'ms'],
    ['Tarefas → Loja (produtos visíveis)', FR('toLojaProducts'), 'ms'],
    ['Tarefas → Loja (produtos − 400 ms simulados)', (r) => r.toLojaProducts?.frame - 400, 'ms'],
    ['Loja → detalhe do produto', FR('toDetail'), 'ms'],
    ['detalhe → Loja (history.back)', FR('back'), 'ms'],
    ['Loja → Painel', FR('toPainel'), 'ms'],
  ],
  interact: [
    ['Counter + (Painel)', FR('counter'), 'ms'],
    ['adicionar tarefa (Enter)', FR('addTodo'), 'ms'],
    ['marcar tarefa (checkbox → StatCard)', FR('toggleTodo'), 'ms'],
    ['digitar 1 char na busca (4 → 1 item)', FR('search'), 'ms'],
    ['Adicionar ao carrinho → CartBadge', FR('addToCart'), 'ms'],
  ],
  stress: [
    ['carga fria /tarefas até 500 linhas pintadas', F('coldRows'), 'ms'],
    ['carga fria /tarefas LCP', F('coldLcp'), 'ms'],
    ['SPA Painel → Tarefas até 500 linhas', FR('spaRows'), 'ms'],
    ['busca: digitar "i" (500 → 250)', FR('searchKey'), 'ms'],
    ['busca: Backspace (250 → 500)', FR('searchClear'), 'ms'],
    ['filtro: Pendentes (500 → 250)', FR('filter'), 'ms'],
  ],
  memory: [
    ['heap JS após carregar / (MB)', (r) => r.initial.heapUsed / 2 ** 20, 'MB'],
    ['heap após 1 ciclo (MB)', (r) => r.after1.heapUsed / 2 ** 20, 'MB'],
    ['heap após 10 ciclos (MB)', (r) => r.after10.heapUsed / 2 ** 20, 'MB'],
    ['heap após 30 ciclos (MB)', (r) => r.after30.heapUsed / 2 ** 20, 'MB'],
    ['Δ heap 1 → 30 ciclos (KB)', (r) => (r.after30.heapUsed - r.after1.heapUsed) / 1024, 'KB'],
    ['Δ heap 10 → 30 ciclos (KB)', (r) => (r.after30.heapUsed - r.after10.heapUsed) / 1024, 'KB'],
    ['nós (CDP Nodes) inicial', (r) => r.initial.nodes, ''],
    ['nós (CDP Nodes) após 1 / 30 ciclos', (r) => `${r.after1.nodes} / ${r.after30.nodes}`, 'txt'],
    ['elementos no document, inicial / após 30', (r) => `${r.initial.docElements} / ${r.after30.docElements}`, 'txt'],
    ['JSEventListeners inicial', (r) => r.initial.listeners, ''],
    ['JSEventListeners após 1 / 30 ciclos', (r) => `${r.after1.listeners} / ${r.after30.listeners}`, 'txt'],
  ],
};

const summary = {};
const stat = (runs, fn, unit) => {
  const v = runs.map((r) => { try { return fn(r); } catch { return NaN; } });
  if (unit === 'txt') { const c = {}; for (const x of v) c[x] = (c[x] || 0) + 1; return { mode: Object.entries(c).sort((a, b) => b[1] - a[1])[0][0], values: v }; }
  return { median: median(v), p90: p90(v), min: Math.min(...v.filter(Number.isFinite)), max: Math.max(...v.filter(Number.isFinite)), n: v.filter(Number.isFinite).length };
};
const fmt = (s, unit) => {
  if (!s) return '—';
  if (unit === 'txt') return s.mode;
  if (s.median === null) return '—';
  const d = unit === 'MB' ? 2 : unit === 'KB' ? 1 : 0;
  const m = s.median.toFixed(d);
  return unit === 'ms' ? `${m} (${s.p90.toFixed(0)})` : m;
};
const tables = {};
for (const [measure, byProfile] of Object.entries(raw)) {
  for (const [pk, byApp] of Object.entries(byProfile)) {
    const defs = measure === 'load' ? ['/', '/tarefas', '/loja'].flatMap((p) => METRICS.load(p).map(([l, f, u]) => [l, (r) => f(r[p]), u, p])) : METRICS[measure];
    const rows = [];
    for (const [label, fn, unit, path] of defs) {
      const cells = {};
      for (const app of APPS) {
        const data = byApp[app];
        if (!data) continue;
        const runs = measure === 'load' ? data[path].map((r) => ({ [path]: r })) : data;
        cells[app] = stat(runs, fn, unit);
      }
      ((summary[pk] ||= {})[measure] ||= {})[label] = Object.fromEntries(Object.entries(cells).map(([a, s]) => [a, { ...s, unit }]));
      rows.push(`| ${label}${unit && unit !== 'txt' ? ` [${unit}]` : ''} | ${APPS.map((a) => fmt(cells[a], unit)).join(' | ')} |`);
    }
    (tables[pk] ||= {})[measure] = `| Métrica | ${APPS.join(' | ')} |\n|---|---:|---:|---:|\n` + rows.join('\n');
  }
}
let lhTable = '';
if (lh) {
  const rows = [['score', (r) => r.score, ''], ['FCP', (r) => r.fcp, 'ms'], ['LCP', (r) => r.lcp, 'ms'], ['TBT', (r) => r.tbt, 'ms'], ['Speed Index', (r) => r.si, 'ms'], ['CLS', (r) => r.cls, 'x'], ['peso total [KB]', (r) => r.bytes / 1024, 'KB']];
  summary.lighthouse = {};
  lhTable = `| Lighthouse (mediana de ${lh.runs}; valores individuais do score) | ${APPS.join(' | ')} |\n|---|---:|---:|---:|\n` + rows.map(([l, fn, u]) => {
    const cells = APPS.map((a) => {
      const v = (lh.data[a] || []).map(fn);
      summary.lighthouse[l] = { ...(summary.lighthouse[l] || {}), [a]: { median: median(v), values: v } };
      const m = median(v);
      return m === null ? '—' : l === 'score' ? `**${m}** (${v.join(', ')})` : u === 'x' ? m.toFixed(3) : u === 'KB' ? m.toFixed(1) : m.toFixed(0);
    });
    return `| ${l}${u === 'ms' ? ' [ms]' : ''} | ${cells.join(' | ')} |`;
  }).join('\n');
}

const meta = {
  generatedAt: new Date().toISOString(),
  node: process.version,
  chromium: chromiumVersion,
  playwright: pkg('playwright'),
  lighthouse: lh?.lighthouse ?? null,
  os: execFileSync('uname', ['-sr']).toString().trim(),
  cpu: execFileSync('sh', ['-c', "grep -m1 'model name' /proc/cpuinfo | cut -d: -f2"]).toString().trim(),
  profiles: PROFILES,
  urls: { sveltekit: 'http://localhost:5171', astra: 'http://127.0.0.1:5172', fable: 'http://127.0.0.1:5173' },
  note: 'Tempos em ms. Células das tabelas: mediana (p90). Ver perf.md para metodologia.',
};
writeFileSync(HERE + '../perf.json', JSON.stringify({ meta, summary, raw, lighthouse: lh }, null, 1));
writeFileSync(HERE + 'tables.json', JSON.stringify({ meta, tables, lhTable }, null, 1));
for (const pk of Object.keys(tables)) for (const m of Object.keys(tables[pk])) console.log(`\n### ${pk} ${m}\n${tables[pk][m]}`);
console.log('\n' + lhTable);
console.log(JSON.stringify(meta, null, 1));

// ---- perf.md a partir do template + notas escritas à mão (perf.notes.md: seções "## Resumo" e "## Anomalias") ----
const notes = existsSync(HERE + 'perf.notes.md') ? readFileSync(HERE + 'perf.notes.md', 'utf8') : '';
const part = (h) => { const m = notes.match(new RegExp(`(^## ${h}[\\s\\S]*?)(?=^## |(?![\\s\\S]))`, 'm')); return m ? m[1].trim() : ''; };
let md = readFileSync(HERE + 'perf.template.md', 'utf8');
const vars = { ...meta, summary: part('Resumo'), anomalies: part('Anomalias'), lighthouse: lhTable || '_não rodou_' };
for (const pk of Object.keys(tables)) for (const m of Object.keys(tables[pk])) vars[`${pk}.${m}`] = tables[pk][m];
md = md.replace(/\{\{([\w.]+)\}\}/g, (_, k) => (vars[k] ?? `_(sem dados: ${k})_`));
writeFileSync(HERE + '../perf.md', md);
