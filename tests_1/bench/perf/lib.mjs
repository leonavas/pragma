// Utilitários comuns do harness: trava flock, perfis, helpers injetados na página e estatística.
import { spawn } from 'node:child_process';
import { once } from 'node:events';

export const APPS = {
  sveltekit: 'http://localhost:5171',
  astra: 'http://127.0.0.1:5172',
  fable: 'http://127.0.0.1:5173',
};
export const N_PRODUCTS = 9; // shared/produtos.json
export const LOCK = '/tmp/pragma-bench.lock';

export const PROFILES = {
  A: { name: 'A: sem throttling', cpu: 1, net: null },
  B: {
    name: 'B: mobile lento (CPU 4x + Slow 4G)',
    cpu: 4,
    // 150 ms RTT, 1,6 Mbps down, 750 Kbps up (bytes/s)
    net: { offline: false, latency: 150, downloadThroughput: 1_600_000 / 8, uploadThroughput: 750_000 / 8 },
  },
};

/** Executa fn segurando `flock /tmp/pragma-bench.lock` (mesmo arquivo usado pelo outro agente). */
export async function withLock(label, fn) {
  const t = Date.now();
  const p = spawn('flock', [LOCK, 'sh', '-c', 'echo L; exec cat >/dev/null'], { stdio: ['pipe', 'pipe', 'inherit'] });
  await new Promise((res, rej) => {
    p.stdout.once('data', res);
    p.once('exit', (c) => rej(new Error('flock saiu com ' + c)));
  });
  const waited = Date.now() - t;
  if (waited > 1000) console.error(`  [lock] ${label}: esperou ${(waited / 1000).toFixed(1)} s`);
  try {
    return await fn();
  } finally {
    p.stdin.end();
    await once(p, 'exit');
  }
}

/** Aplica o perfil via CDP na página (antes da navegação). Network.enable sempre, para contar bytes. */
export async function applyProfile(context, page, profile) {
  const cdp = await context.newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.setCacheDisabled', { cacheDisabled: false });
  if (profile.cpu > 1) await cdp.send('Emulation.setCPUThrottlingRate', { rate: profile.cpu });
  if (profile.net) await cdp.send('Network.emulateNetworkConditions', profile.net);
  return cdp;
}

/** Conta requisições e bytes (encodedDataLength = bytes na rede, com cabeçalhos) via CDP. */
export function netCounter(cdp) {
  const s = { requests: 0, bytes: 0, urls: [] };
  cdp.on('Network.requestWillBeSent', (e) => {
    if (e.request.url.startsWith('data:')) return;
    s.requests++;
    s.urls.push(e.request.url);
  });
  cdp.on('Network.loadingFinished', (e) => { s.bytes += e.encodedDataLength; });
  return s;
}

// Script injetado em todo documento antes do código do app (addInitScript).
// Igual para os 3 apps. Só instala observadores baratos; o MutationObserver só existe enquanto armado.
export const INIT_SCRIPT = () => {
  const B = (window.__bench = { lt: [], lcp: 0, lcpEl: '' });
  try {
    new PerformanceObserver((l) => { for (const e of l.getEntries()) B.lt.push([e.startTime, e.duration]); })
      .observe({ type: 'longtask', buffered: true });
    new PerformanceObserver((l) => {
      for (const e of l.getEntries()) { B.lcp = e.startTime; B.lcpEl = e.element ? e.element.tagName + '.' + (e.element.className || '') : ''; }
    }).observe({ type: 'largest-contentful-paint', buffered: true });
  } catch {}

  const H = (B.H = {
    main: () => document.querySelector('main') || document.body,
    out: (label) => document.querySelector(`[aria-label="Aumentar ${label}"]`)?.parentElement?.querySelector('output')?.textContent.trim(),
    badge: () => [...document.querySelectorAll('a[href="/loja"]')].find((a) => /carrinho/i.test(a.textContent))?.textContent.replace(/\s+/g, ' ').trim(),
    checks: () => (document.querySelector('main') || document.body).querySelectorAll('li input[type=checkbox]').length,
    h1: () => document.querySelector('main h1')?.textContent.trim(),
    productLinks: () => (document.querySelector('main') || document.body).querySelectorAll('a[href^="/loja/"]').length,
    visible: (el) => !!el && !el.hidden && el.getClientRects().length > 0,
    backLink: () => [...(document.querySelector('main') || document.body).querySelectorAll('a')].find((a) => a.textContent.includes('Voltar para a loja')),
    addButtons: () => [...(document.querySelector('main') || document.body).querySelectorAll('button')].filter((b) => b.textContent.trim() === 'Adicionar').length,
    stat: (title) => [...document.querySelectorAll('main h2, main h3')].find((h) => h.textContent.trim() === title)?.closest('article')?.textContent.replace(/\s+/g, ' ').trim(),
    hasText: (t) => (document.querySelector('main') || document.body).textContent.includes(t),
  });

  // Arma uma medição: t0 = timeStamp do próximo evento `type` (captura, na window) ou B.fire().
  // Cada condição registra quando ficou verdadeira (callback do MutationObserver) e o fim do
  // frame seguinte (rAF + MessageChannel, que roda depois do paint).
  B.arm = (type, conds, timeout = 20000) => {
    const fns = Object.fromEntries(Object.entries(conds).map(([k, src]) => [k, new Function('H', 'return (' + src + ')')]));
    const res = (B.cur = { t0: null, dom: {}, frame: {}, pre: [] });
    for (const [k, f] of Object.entries(fns)) { try { if (f(H)) res.pre.push(k); } catch {} }
    let mo, done;
    B.pending = new Promise((resolve) => {
      done = resolve;
      const check = () => {
        if (res.t0 === null) return;
        for (const [k, f] of Object.entries(fns)) {
          if (k in res.dom) continue;
          let ok = false;
          try { ok = f(H); } catch {}
          if (ok) {
            res.dom[k] = performance.now();
            requestAnimationFrame(() => {
              const ch = new MessageChannel();
              ch.port1.onmessage = () => {
                res.frame[k] = performance.now();
                if (Object.keys(res.frame).length === Object.keys(fns).length) { mo.disconnect(); resolve(res); }
              };
              ch.port2.postMessage(0);
            });
          }
        }
      };
      B._check = check;
      mo = new MutationObserver(check);
      mo.observe(document, { subtree: true, childList: true, characterData: true, attributes: true });
      if (type) {
        window.addEventListener(type, (e) => { res.t0 = e.timeStamp; queueMicrotask(check); }, { capture: true, once: true });
      }
      setTimeout(() => { mo.disconnect(); res.timeout = true; resolve(res); }, timeout);
    });
    return res.pre;
  };
  // t0 manual (ex.: history.back(), ou 0 = início da navegação).
  B.start = (t = performance.now()) => { B.cur.t0 = t; queueMicrotask(() => B._check()); return t; };
};

export const median = (a) => {
  const v = a.filter((x) => Number.isFinite(x)).sort((x, y) => x - y);
  if (!v.length) return null;
  const m = Math.floor(v.length / 2);
  return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
};
export const p90 = (a) => {
  const v = a.filter((x) => Number.isFinite(x)).sort((x, y) => x - y);
  if (!v.length) return null;
  return v[Math.min(v.length - 1, Math.ceil(0.9 * v.length) - 1)];
};
export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
