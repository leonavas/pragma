// Reatividade: signal, computed, effect, batch, scope, onCleanup, untrack.
//
// Modelo push-pull em três estados. Uma escrita só marca os dependentes
// (DIRTY nos diretos, CHECK nos indiretos); o valor é recalculado quando alguém
// lê (computed) ou no flush (effect). Por isso um computed nunca roda duas vezes
// por escrita nem expõe estado intermediário (dependências em diamante).
//
// Regras do dialeto garantidas aqui:
// - escrever em signal dentro de effect/computed lança erro;
// - effects rodam na ordem de criação (o pai sempre antes dos filhos);
// - tudo que é criado dentro de um scope/effect é descartado junto com ele.

const CLEAN = 0;
const CHECK = 1;
const DIRTY = 2;

let observer = null; // computed/effect em execução (quem registra dependências)
let owner = null; // dono dos cleanups criados agora
let batchDepth = 0;
let flushing = false;
let nextId = 0;
let queue = [];

function createNode(fn, isComputed, equals) {
  const node = {
    id: nextId++,
    fn,
    value: undefined,
    error: null,
    equals,
    state: DIRTY,
    deps: new Set(),
    subs: isComputed ? new Set() : null,
    cleanups: [],
    disposed: false,
  };
  if (owner) owner.cleanups.push(() => dispose(node));
  return node;
}

function track(source) {
  if (observer) {
    observer.deps.add(source);
    source.subs.add(observer);
  }
}

function mark(node, state) {
  if (node.state >= state) return;
  const wasClean = node.state === CLEAN;
  node.state = state;
  if (!wasClean) return;
  if (node.subs) for (const sub of node.subs) mark(sub, CHECK);
  else queue.push(node);
}

function cleanup(node) {
  for (const dep of node.deps) dep.subs.delete(node);
  node.deps.clear();
  const cleanups = node.cleanups;
  node.cleanups = [];
  for (let i = cleanups.length - 1; i >= 0; i--) cleanups[i]();
}

function dispose(node) {
  node.disposed = true;
  node.state = CLEAN;
  cleanup(node);
}

function run(node) {
  cleanup(node);
  const prevObserver = observer;
  const prevOwner = owner;
  observer = owner = node;
  node.state = CLEAN; // antes de rodar: uma marcação durante a execução reagenda
  let value;
  let error = null;
  try {
    value = node.fn();
  } catch (thrown) {
    if (!node.subs) throw thrown;
    error = thrown; // computed guarda o erro e relança para quem ler
  } finally {
    observer = prevObserver;
    owner = prevOwner;
  }
  if (node.subs && (error || node.error || !node.equals(node.value, value))) {
    node.value = value;
    node.error = error;
    for (const sub of node.subs) sub.state = DIRTY;
  }
}

// Garante que o nó está atualizado. Em CHECK, confere as dependências na ordem
// em que foram lidas e para na primeira que mudou: um ramo condicional que
// deixou de valer nunca é recalculado.
function refresh(node) {
  if (node.state === CHECK) {
    for (const dep of node.deps) {
      if (dep.fn) refresh(dep);
      if (node.state === DIRTY) break;
    }
  }
  if (node.state === DIRTY) run(node);
  else node.state = CLEAN;
}

function flush() {
  if (flushing || batchDepth) return;
  flushing = true;
  let failed = false;
  let firstError;
  try {
    while (queue.length) {
      const pending = queue.sort((a, b) => a.id - b.id);
      queue = [];
      for (const node of pending) {
        if (node.disposed) continue;
        try {
          refresh(node);
        } catch (error) {
          if (!failed) firstError = error;
          failed = true;
        }
      }
    }
  } finally {
    flushing = false;
  }
  if (failed) throw firstError;
}

/**
 * Valor reativo. `s()` lê (e registra dependência), `s.set(v)` escreve,
 * `s.update(fn)` escreve a partir do valor atual.
 * @template T
 * @param {T} value
 */
export function signal(value, equals = Object.is) {
  const source = { value, subs: new Set() };
  const read = () => {
    track(source);
    return source.value;
  };
  read.set = (next) => {
    if (observer) {
      throw new Error('Escrita em signal dentro de effect/computed. Derive com computed() ou escreva no handler do evento.');
    }
    if (equals(source.value, next)) return;
    source.value = next;
    for (const sub of source.subs) mark(sub, DIRTY);
    flush();
  };
  read.update = (fn) => read.set(fn(source.value));
  return read;
}

/**
 * Valor derivado, memoizado e preguiçoso. Só notifica quem depende dele se o
 * resultado mudar.
 * @template T
 * @param {() => T} fn
 * @returns {() => T}
 */
export function computed(fn, equals = Object.is) {
  const node = createNode(fn, true, equals);
  return () => {
    if (!node.disposed) refresh(node);
    track(node);
    if (node.error) throw node.error;
    return node.value;
  };
}

/**
 * Roda `fn` agora e de novo sempre que algo lido por ela mudar.
 * Devolve a função que descarta o effect.
 * @param {() => void} fn
 */
export function effect(fn) {
  const node = createNode(fn, false);
  batch(() => run(node));
  return () => dispose(node);
}

/** Agrupa escritas: os effects rodam uma vez, ao fim do batch mais externo. */
export function batch(fn) {
  batchDepth++;
  try {
    return fn();
  } finally {
    batchDepth--;
    flush();
  }
}

/**
 * Escopo de descarte isolado: effects, computeds e cleanups criados em `fn`
 * são descartados juntos pela função devolvida. Dentro de `fn` nada é
 * rastreado pelo effect que estiver em execução.
 * @param {() => void} fn
 * @returns {() => void} dispose
 */
export function scope(fn) {
  const node = { cleanups: [], deps: new Set(), disposed: false, state: CLEAN };
  const prevObserver = observer;
  const prevOwner = owner;
  observer = null;
  owner = node;
  try {
    batch(fn);
  } finally {
    observer = prevObserver;
    owner = prevOwner;
  }
  return () => dispose(node);
}

/**
 * Roda `fn` fora do effect em execução: nada é rastreado e escritas são
 * permitidas. Existe para handlers de evento que o browser dispara de forma
 * síncrona durante uma escrita no DOM (ex.: `blur` ao ocultar o campo focado).
 */
export function untrack(fn) {
  const prevObserver = observer;
  observer = null;
  try {
    return fn();
  } finally {
    observer = prevObserver;
  }
}

/** Registra uma limpeza no scope/effect atual (timers, listeners externos). */
export function onCleanup(fn) {
  if (!owner) throw new Error('onCleanup() fora de um scope ou effect.');
  owner.cleanups.push(fn);
}
