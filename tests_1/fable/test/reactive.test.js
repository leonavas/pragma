import { test } from 'node:test';
import assert from 'node:assert/strict';
import { signal, computed, effect, batch, scope, onCleanup, untrack } from '../src/lib/reactive.js';

test('signal: lê, escreve e ignora valor igual', () => {
  const n = signal(1);
  let runs = 0;
  effect(() => {
    n();
    runs++;
  });
  n.set(1);
  assert.equal(runs, 1);
  n.set(2);
  n.update((v) => v + 1);
  assert.equal(n(), 3);
  assert.equal(runs, 3);
});

test('computed: é preguiçoso e memoizado', () => {
  const n = signal(2);
  let runs = 0;
  const double = computed(() => {
    runs++;
    return n() * 2;
  });
  assert.equal(runs, 0);
  assert.equal(double(), 4);
  assert.equal(double(), 4);
  assert.equal(runs, 1);
  n.set(3);
  assert.equal(runs, 1, 'não recalcula sem leitura');
  assert.equal(double(), 6);
  assert.equal(runs, 2);
});

test('diamante: sem glitch e uma execução por escrita', () => {
  const a = signal(1);
  const b = computed(() => a() + 1);
  const c = computed(() => a() * 10);
  let dRuns = 0;
  const d = computed(() => {
    dRuns++;
    return `${b()}:${c()}`;
  });
  const seen = [];
  effect(() => seen.push(d()));
  a.set(2);
  a.set(3);
  assert.deepEqual(seen, ['2:10', '3:20', '4:30']);
  assert.equal(dRuns, 3);
});

test('computed com resultado igual não propaga', () => {
  const n = signal(1);
  const parity = computed(() => n() % 2);
  let runs = 0;
  effect(() => {
    parity();
    runs++;
  });
  n.set(3);
  n.set(5);
  assert.equal(runs, 1);
  n.set(6);
  assert.equal(runs, 2);
});

test('dependência condicional: ramo inativo não dispara nem é recalculado', () => {
  const useA = signal(true);
  const a = signal('a');
  const b = signal('b');
  let bRuns = 0;
  const bUpper = computed(() => {
    bRuns++;
    return b().toUpperCase();
  });
  const seen = [];
  effect(() => seen.push(useA() ? a() : bUpper()));
  b.set('b2');
  assert.deepEqual(seen, ['a']);
  assert.equal(bRuns, 0);
  useA.set(false);
  assert.deepEqual(seen, ['a', 'B2']);
  a.set('a2');
  assert.deepEqual(seen, ['a', 'B2'], 'a saiu das dependências');
  b.set('b3');
  assert.deepEqual(seen, ['a', 'B2', 'B3']);
});

test('dependência condicional: não avalia o ramo que a condição acabou de invalidar', () => {
  const list = signal([1]);
  const hasItems = computed(() => list().length > 0);
  const first = computed(() => {
    if (!list().length) throw new Error('ramo inválido avaliado');
    return list()[0];
  });
  const seen = [];
  effect(() => seen.push(hasItems() ? first() : 'vazio'));
  list.set([]);
  assert.deepEqual(seen, [1, 'vazio']);
});

test('batch: effects rodam uma vez, ao fim do batch mais externo', () => {
  const a = signal(1);
  const b = signal(1);
  const seen = [];
  effect(() => seen.push(a() + b()));
  batch(() => {
    a.set(2);
    batch(() => b.set(2));
    assert.deepEqual(seen, [2], 'nada roda dentro do batch');
    a.set(3);
  });
  assert.deepEqual(seen, [2, 5]);
});

test('batch: computed lido dentro do batch já vê o valor novo', () => {
  const a = signal(1);
  const double = computed(() => a() * 2);
  assert.equal(double(), 2);
  batch(() => {
    a.set(5);
    assert.equal(double(), 10);
  });
});

test('ordem: effects rodam na ordem de criação', () => {
  const n = signal(0);
  const viaComputed = computed(() => n() + 1);
  const order = [];
  effect(() => {
    viaComputed();
    order.push('primeiro');
  });
  effect(() => {
    n();
    order.push('segundo');
  });
  effect(() => {
    viaComputed();
    order.push('terceiro');
  });
  order.length = 0;
  n.set(1);
  assert.deepEqual(order, ['primeiro', 'segundo', 'terceiro']);
});

test('ordem: o pai roda antes do filho e o filho antigo não roda', () => {
  const n = signal(0);
  const order = [];
  effect(() => {
    const value = n();
    order.push(`pai ${value}`);
    effect(() => order.push(`filho ${value} vê ${n()}`));
  });
  n.set(1);
  assert.deepEqual(order, ['pai 0', 'filho 0 vê 0', 'pai 1', 'filho 1 vê 1']);
});

test('dispose: effect descartado não roda mais', () => {
  const n = signal(0);
  let runs = 0;
  const stop = effect(() => {
    n();
    runs++;
  });
  stop();
  n.set(1);
  assert.equal(runs, 1);
});

test('onCleanup: roda antes de cada reexecução e no descarte', () => {
  const n = signal(0);
  const log = [];
  const stop = effect(() => {
    const value = n();
    onCleanup(() => log.push(`limpa ${value}`));
  });
  n.set(1);
  assert.deepEqual(log, ['limpa 0']);
  stop();
  assert.deepEqual(log, ['limpa 0', 'limpa 1']);
});

test('scope: descarta effects, computeds e cleanups de uma vez', () => {
  const n = signal(0);
  const log = [];
  let double;
  const dispose = scope(() => {
    double = computed(() => n() * 2);
    effect(() => log.push(double()));
    effect(() => {
      effect(() => log.push(`aninhado ${n()}`));
    });
    onCleanup(() => log.push('limpeza'));
  });
  n.set(1);
  assert.deepEqual(log, [0, 'aninhado 0', 2, 'aninhado 1']);
  dispose();
  n.set(2);
  assert.deepEqual(log, [0, 'aninhado 0', 2, 'aninhado 1', 'limpeza']);
});

test('scope: não vaza dependência para o effect em execução', () => {
  const outer = signal(0);
  const inner = signal(0);
  let runs = 0;
  let disposeInner;
  effect(() => {
    outer();
    runs++;
    disposeInner = scope(() => inner());
  });
  inner.set(1);
  assert.equal(runs, 1);
  outer.set(1);
  assert.equal(runs, 2);
  disposeInner();
});

test('scope dentro de effect sobrevive à reexecução do effect', () => {
  const outer = signal(0);
  const n = signal(0);
  const log = [];
  let dispose;
  effect(() => {
    outer();
    dispose ??= scope(() => effect(() => log.push(n())));
  });
  outer.set(1);
  n.set(1);
  assert.deepEqual(log, [0, 1]);
  dispose();
  n.set(2);
  assert.deepEqual(log, [0, 1]);
});

test('escrever em signal dentro de effect ou computed lança erro', () => {
  const a = signal(0);
  const b = signal(0);
  assert.throws(() => effect(() => b.set(a() + 1)), /Escrita em signal/);
  const bad = computed(() => b.set(1));
  assert.throws(() => bad(), /Escrita em signal/);
  assert.equal(b(), 0);
});

test('escrita dentro de scope é permitida mesmo durante um effect', () => {
  const trigger = signal(0);
  const status = signal('idle');
  const seen = [];
  effect(() => seen.push(status()));
  effect(() => {
    trigger();
    scope(() => status.set('loading'));
  });
  assert.deepEqual(seen, ['idle', 'loading']);
});

test('erro em um effect não impede os outros e é relançado', () => {
  const n = signal(0);
  const seen = [];
  effect(() => {
    if (n() === 1) throw new Error('falhou');
  });
  effect(() => seen.push(n()));
  assert.throws(() => n.set(1), /falhou/);
  assert.deepEqual(seen, [0, 1]);
  n.set(2);
  assert.deepEqual(seen, [0, 1, 2]);
});

test('effect que falhou volta a rodar na próxima mudança', () => {
  const n = signal(0);
  const seen = [];
  effect(() => {
    if (n() === 1) throw new Error('falhou');
    seen.push(n());
  });
  assert.throws(() => n.set(1), /falhou/);
  n.set(2);
  assert.deepEqual(seen, [0, 2]);
});

test('computed que falha relança o erro e se recupera', () => {
  const n = signal(0);
  const safe = computed(() => {
    if (n() === 1) throw new Error('inválido');
    return n();
  });
  const seen = [];
  effect(() => seen.push(safe()));
  assert.throws(() => n.set(1), /inválido/);
  assert.throws(() => safe(), /inválido/);
  n.set(2);
  assert.deepEqual(seen, [0, 2]);
  assert.equal(safe(), 2);
});

test('onCleanup fora de escopo lança erro', () => {
  assert.throws(() => onCleanup(() => {}), /fora de um scope/);
});

test('untrack: lê sem registrar dependência e permite escrita adiada', () => {
  const a = signal(0);
  const b = signal(0);
  const flag = signal(false);
  let runs = 0;
  const seen = [];
  effect(() => seen.push(flag()));
  effect(() => {
    a();
    untrack(() => {
      b();
      flag.set(a() > 0); // como um handler disparado pelo DOM durante o effect
    });
    runs++;
  });
  b.set(1);
  assert.equal(runs, 1);
  a.set(1);
  assert.equal(runs, 2);
  assert.deepEqual(seen, [false, true]);
});
