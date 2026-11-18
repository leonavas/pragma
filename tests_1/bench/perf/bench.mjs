// Harness de desempenho dos 3 apps "Mercadinho".
// Uso: node bench.mjs <medida...> [--apps sveltekit,astra,fable] [--profiles A,B] [--runs 10]
// Medidas: load tti nav interact stress memory
// Cada bloco (medida × perfil × app) roda sob `flock /tmp/pragma-bench.lock` e grava raw/<medida>-<perfil>-<app>.json.
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';
import { APPS, PROFILES, N_PRODUCTS, INIT_SCRIPT, withLock, applyProfile, netCounter, sleep } from './lib.mjs';

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf('--' + k); return i >= 0 ? args[i + 1] : d; };
const measures = args.filter((a, i) => !a.startsWith('--') && !(i > 0 && args[i - 1].startsWith('--')));
const apps = opt('apps', 'sveltekit,astra,fable').split(',');
const profiles = opt('profiles', 'A,B').split(',');
const RUNS = Number(opt('runs', 10));
const RAW = new URL('./raw/', import.meta.url).pathname;
mkdirSync(RAW, { recursive: true });

const LAUNCH = { channel: 'chromium', headless: true }; // Chromium completo (headless novo), não o headless-shell
const VIEWPORT = { width: 1280, height: 720 };

async function newPage(browser, profile, extraInit = []) {
  const context = await browser.newContext({ viewport: VIEWPORT, serviceWorkers: 'block' });
  await context.addInitScript(INIT_SCRIPT);
  for (const s of extraInit) await context.addInitScript(s.fn, s.arg);
  const page = await context.newPage();
  const cdp = await applyProfile(context, page, profile);
  return { context, page, cdp };
}

// Arma, executa a ação de entrada, espera o resultado. Retorna ms (dom e próximo frame) por condição.
async function timed(page, type, conds, action, timeout = 20000) {
  const pre = await page.evaluate(([t, c, to]) => window.__bench.arm(t, c, to), [type, conds, timeout]);
  if (pre.length) throw new Error('condição já verdadeira antes da ação: ' + pre.join(','));
  await action();
  const r = await page.evaluate(() => window.__bench.pending);
  if (r.timeout) throw new Error('timeout esperando ' + JSON.stringify(conds) + ' ' + JSON.stringify(r));
  const out = {};
  for (const k of Object.keys(conds)) out[k] = { dom: r.dom[k] - r.t0, frame: r.frame[k] - r.t0 };
  return out;
}

const navLink = (page, name) => page.getByRole('banner').getByRole('link', { name, exact: true });
const settle = (page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => setTimeout(r, 50))));
const C = {
  painel: 'H.out("Cliques") !== undefined',
  tarefas: 'H.h1() === "Tarefas" && H.checks() > 0',
  lojaContent: 'H.h1() === "Loja"',
  lojaProducts: `H.h1() === "Loja" && H.productLinks() >= ${N_PRODUCTS}`,
};

// ---------- 1. Carga fria ----------
async function load(browser, base, profile) {
  const out = {};
  for (const path of ['/', '/tarefas', '/loja']) {
    out[path] = [];
    for (let i = 0; i < RUNS; i++) {
      const { context, page, cdp } = await newPage(browser, profile);
      const net = netCounter(cdp);
      await page.goto(base + path, { waitUntil: 'load', timeout: 60000 });
      if (path === '/loja') await page.waitForFunction((n) => document.querySelectorAll('main a[href^="/loja/"]').length >= n, N_PRODUCTS, { timeout: 30000 });
      await page.waitForLoadState('networkidle');
      await sleep(profile.cpu > 1 ? 1500 : 800); // deixa LCP/long tasks assentarem
      const m = await page.evaluate(() => {
        const nav = performance.getEntriesByType('navigation')[0];
        const fcp = performance.getEntriesByName('first-contentful-paint')[0]?.startTime ?? null;
        const B = window.__bench;
        const lts = B.lt.filter(([s]) => fcp === null || s >= fcp);
        const res = performance.getEntriesByType('resource');
        return {
          fcp, lcp: B.lcp, lcpEl: B.lcpEl,
          dcl: nav.domContentLoadedEventEnd, load: nav.loadEventEnd, ttfb: nav.responseStart,
          longTasks: B.lt.length, longTaskMs: B.lt.reduce((s, [, d]) => s + d, 0),
          tbt: lts.reduce((s, [, d]) => s + Math.max(0, d - 50), 0),
          decodedBytes: nav.decodedBodySize + res.reduce((s, r) => s + r.decodedBodySize, 0),
          domNodes: document.getElementsByTagName('*').length,
        };
      });
      m.requests = net.requests; m.bytes = net.bytes; m.urls = net.urls.map((u) => u.replace(base, ''));
      out[path].push(m);
      await context.close();
    }
  }
  return out;
}

// ---------- 2. Tempo até interativo prático ----------
async function tti(browser, base, profile) {
  const out = [];
  for (let i = 0; i < RUNS; i++) {
    const armAtStart = { fn: () => { window.__bench.arm(null, { ok: 'H.out("Cliques") !== undefined && H.out("Cliques") !== "0"' }, 60000); window.__bench.start(0); } };
    const { context, page } = await newPage(browser, profile, [armAtStart]);
    await page.goto(base + '/', { waitUntil: 'commit' });
    // Loop apertado: assim que o botão tem caixa, clique real (CDP Input) no centro; repete até o valor mudar.
    let clicks = 0, firstClick = null;
    const t = Date.now();
    for (;;) {
      const st = await page.evaluate(() => {
        if (window.__bench?.cur?.dom.ok) return { ok: true };
        const b = document.querySelector('[aria-label="Aumentar Cliques"]');
        const r = b?.getBoundingClientRect();
        return r && r.width ? { x: r.x + r.width / 2, y: r.y + r.height / 2, now: performance.now() } : null;
      }).catch(() => null);
      if (st?.ok) break;
      if (st) {
        await page.mouse.click(st.x, st.y);
        clicks++;
        if (firstClick === null) firstClick = st.now;
      }
      if (Date.now() - t > 60000) throw new Error('nunca ficou interativo');
    }
    await page.waitForFunction(() => window.__bench.cur.frame.ok);
    const r = await page.evaluate(() => window.__bench.cur);
    const nav = await page.evaluate(() => performance.getEntriesByType('navigation')[0].domContentLoadedEventEnd);
    out.push({ tti: r.frame.ok, ttiDom: r.dom.ok, firstClickAt: firstClick, clicks, dcl: nav });
    await context.close();
  }
  return out;
}

// ---------- 3. Navegação SPA ----------
async function nav(browser, base, profile) {
  const out = [];
  for (let i = 0; i < RUNS; i++) {
    const { context, page } = await newPage(browser, profile);
    await page.goto(base + '/', { waitUntil: 'load' });
    await page.waitForLoadState('networkidle');
    await settle(page);
    const r = {};
    r.toTarefas = (await timed(page, 'pointerdown', { c: C.tarefas }, () => navLink(page, 'Tarefas').click())).c;
    await settle(page);
    const loja = await timed(page, 'pointerdown', { content: C.lojaContent, products: C.lojaProducts }, () => navLink(page, 'Loja').click());
    r.toLoja = loja.content; r.toLojaProducts = loja.products;
    await page.waitForLoadState('networkidle'); await settle(page);
    const name = 'Café torrado 500 g';
    r.toDetail = (await timed(page, 'pointerdown',
      { c: `H.visible(H.backLink()) && H.addButtons() === 1 && H.hasText(${JSON.stringify(name)})` },
      () => page.locator('main').getByRole('link', { name, exact: true }).first().click())).c;
    await settle(page);
    r.back = (await timed(page, null, { c: `${C.lojaProducts} && !H.visible(H.backLink())` },
      () => page.evaluate(() => { window.__bench.start(); history.back(); }))).c;
    await settle(page);
    r.toPainel = (await timed(page, 'pointerdown', { c: C.painel }, () => navLink(page, 'Painel').click())).c;
    out.push(r);
    await context.close();
  }
  return out;
}

// ---------- 4. Latência de interação ----------
async function interact(browser, base, profile) {
  const out = [];
  for (let i = 0; i < RUNS; i++) {
    const { context, page } = await newPage(browser, profile);
    await page.goto(base + '/', { waitUntil: 'load' });
    await page.waitForLoadState('networkidle'); await settle(page);
    const r = {};
    r.counter = (await timed(page, 'pointerdown', { c: 'H.out("Cliques") === "1"' },
      () => page.locator('[aria-label="Aumentar Cliques"]').click())).c;

    await navLink(page, 'Tarefas').click();
    await page.waitForFunction(() => window.__bench.H.checks() === 3);
    await settle(page);
    const input = page.locator('main input:not([type=search]):not([type=checkbox])').first();
    await input.fill('Tarefa medida');
    await settle(page);
    r.addTodo = (await timed(page, 'keydown', { c: 'H.checks() === 4 && H.hasText("Tarefa medida")' }, () => input.press('Enter'))).c;
    await settle(page);
    const before = await page.evaluate(() => window.__bench.H.stat('Concluídas'));
    r.toggleTodo = (await timed(page, 'pointerdown', { c: `H.stat("Concluídas") !== ${JSON.stringify(before)}` },
      () => page.locator('main li input[type=checkbox]').first().click())).c;
    await settle(page);
    const search = page.locator('main input[type=search]');
    await search.focus();
    // "s" só aparece em "Pagar contas": 4 itens -> 1
    r.search = (await timed(page, 'keydown', { c: 'H.checks() === 1' }, () => page.keyboard.press('s'))).c;
    await settle(page);

    await navLink(page, 'Loja').click();
    await page.waitForFunction((n) => window.__bench.H.productLinks() >= n, N_PRODUCTS);
    await page.waitForLoadState('networkidle'); await settle(page);
    const badge = await page.evaluate(() => window.__bench.H.badge());
    r.addToCart = (await timed(page, 'pointerdown', { c: `H.badge() !== ${JSON.stringify(badge)}` },
      () => page.locator('main').getByRole('button', { name: 'Adicionar', exact: true }).first().click())).c;
    r.badgeAfter = await page.evaluate(() => window.__bench.H.badge());
    out.push(r);
    await context.close();
  }
  return out;
}

// ---------- 5. Estresse com 500 tarefas ----------
const seedTodos = (app) => {
  const list = Array.from({ length: 500 }, (_, k) => {
    const n = k + 1;
    // "i" só aparece em "impar" (250 itens); concluídas = pares (250 pendentes)
    return { id: 's' + n, title: `Tarefa ${n} ${n % 2 ? 'impar' : 'par'}`, done: n % 2 === 0 };
  });
  if (app === 'sveltekit') return { 'mercadinho:tarefas': JSON.stringify(list.map((t) => ({ id: t.id, titulo: t.title, feita: t.done }))) };
  if (app === 'fable') return { 'mercadinho:tarefas': JSON.stringify(list) };
  return { 'mercadinho-astra-v1': JSON.stringify({ todos: list, cart: [], theme: 'light' }) };
};
const seedInit = (app) => ({
  fn: (kv) => { if (!sessionStorage.getItem('__seeded')) { for (const [k, v] of Object.entries(kv)) localStorage.setItem(k, v); sessionStorage.setItem('__seeded', '1'); } },
  arg: seedTodos(app),
});

async function stress(browser, base, profile, app) {
  const out = [];
  for (let i = 0; i < RUNS; i++) {
    const r = {};
    // (a) carga fria de /tarefas até as 500 linhas existirem
    {
      const armAtStart = { fn: () => { window.__bench.arm(null, { rows: 'H.checks() >= 500' }, 60000); window.__bench.start(0); } };
      const { context, page } = await newPage(browser, profile, [seedInit(app), armAtStart]);
      await page.goto(base + '/tarefas', { waitUntil: 'load' });
      const res = await page.evaluate(() => window.__bench.pending);
      if (res.timeout) throw new Error('500 linhas não renderizaram');
      await page.waitForLoadState('networkidle');
      await sleep(profile.cpu > 1 ? 1500 : 800);
      const m = await page.evaluate(() => ({ lcp: window.__bench.lcp, fcp: performance.getEntriesByName('first-contentful-paint')[0]?.startTime }));
      r.coldRows = res.frame.rows; r.coldRowsDom = res.dom.rows; r.coldLcp = m.lcp; r.coldFcp = m.fcp;
      await context.close();
    }
    // (b) navegação SPA Painel -> Tarefas com 500, busca e filtro
    {
      const { context, page } = await newPage(browser, profile, [seedInit(app)]);
      await page.goto(base + '/', { waitUntil: 'load' });
      await page.waitForLoadState('networkidle'); await settle(page);
      r.spaRows = (await timed(page, 'pointerdown', { c: 'H.checks() >= 500' }, () => navLink(page, 'Tarefas').click())).c;
      await settle(page); await sleep(200);
      const search = page.locator('main input[type=search]');
      await search.focus();
      r.searchKey = (await timed(page, 'keydown', { c: 'H.checks() === 250' }, () => page.keyboard.press('i'))).c;
      await settle(page);
      r.searchClear = (await timed(page, 'keydown', { c: 'H.checks() === 500' }, () => page.keyboard.press('Backspace'))).c;
      await settle(page);
      const filter = page.locator('main').getByRole('button', { name: 'Pendentes', exact: true })
        .or(page.locator('main').getByRole('link', { name: 'Pendentes', exact: true }));
      r.filter = (await timed(page, 'pointerdown', { c: 'H.checks() === 250' }, () => filter.click())).c;
      await context.close();
    }
    out.push(r);
  }
  return out;
}

// ---------- 6. Memória / vazamento ----------
async function memory(browser, base, profile) {
  const out = [];
  for (let i = 0; i < RUNS; i++) {
    const { context, page, cdp } = await newPage(browser, profile);
    await cdp.send('Performance.enable');
    const snap = async () => {
      await sleep(300);
      await cdp.send('HeapProfiler.collectGarbage');
      await cdp.send('HeapProfiler.collectGarbage');
      const { metrics } = await cdp.send('Performance.getMetrics');
      const g = Object.fromEntries(metrics.map((m) => [m.name, m.value]));
      const doc = await page.evaluate(() => document.getElementsByTagName('*').length);
      return { heapUsed: g.JSHeapUsedSize, heapTotal: g.JSHeapTotalSize, nodes: g.Nodes, listeners: g.JSEventListeners, documents: g.Documents, docElements: doc };
    };
    await page.goto(base + '/', { waitUntil: 'load' });
    await page.waitForLoadState('networkidle'); await settle(page);
    const r = { initial: await snap() };
    const cycle = async () => {
      for (const [name, cond] of [['Tarefas', C.tarefas], ['Loja', C.lojaProducts], ['Painel', C.painel]]) {
        await navLink(page, name).click();
        await page.waitForFunction((src) => new Function('H', 'return (' + src + ')')(window.__bench.H), cond, { timeout: 30000 });
      }
    };
    await cycle();
    r.after1 = await snap();
    for (let k = 1; k < 10; k++) await cycle();
    r.after10 = await snap();
    for (let k = 10; k < 30; k++) await cycle();
    r.after30 = await snap();
    out.push(r);
    await context.close();
  }
  return out;
}

const FN = { load, tti, nav, interact, stress, memory };
const browserVersion = { value: null };
for (const m of measures) {
  if (!FN[m]) throw new Error('medida desconhecida: ' + m);
  for (const pk of profiles) {
    if (m === 'memory' && pk !== 'A') continue; // memória não depende de throttling (ver perf.md)
    for (const app of apps) {
      const label = `${m}-${pk}-${app}`;
      const t = Date.now();
      const data = await withLock(label, async () => {
        const browser = await chromium.launch(LAUNCH);
        browserVersion.value = browser.version();
        try { return await FN[m](browser, APPS[app], PROFILES[pk], app); } finally { await browser.close(); }
      });
      writeFileSync(RAW + label + '.json', JSON.stringify({ measure: m, profile: pk, app, runs: RUNS, chromium: browserVersion.value, at: new Date().toISOString(), data }, null, 1));
      console.log(`${label}: ok (${((Date.now() - t) / 1000).toFixed(1)} s)`);
    }
  }
}
