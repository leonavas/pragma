// DOM: `h` cria elementos e liga props/filhos reativos; `list` mantém uma
// lista com chave. Toda escrita no DOM que depende de estado acontece dentro
// de um effect criado aqui, nunca no código do app.
import { computed, effect, onCleanup, scope, untrack } from './reactive.js';

/** Lê um valor que pode ser estático ou um getter (signal/computed). */
export const read = (value) => (typeof value === 'function' ? value() : value);

function setProp(el, name, value) {
  if (name === 'class') el.className = value ?? '';
  else if (name in el) el[name] = value;
  else if (value == null || value === false) el.removeAttribute(name);
  else el.setAttribute(name, value === true ? '' : value);
}

/**
 * Cria um elemento.
 * - Elemento nativo: `on*` vira listener; prop com função vira binding
 *   reativo; o resto é propriedade ou atributo.
 * - Custom element (tag com hífen): `props` é entregue inteiro ao componente.
 * - Filhos: texto, nós, arrays ou funções (região reativa).
 */
export function h(tag, props, ...children) {
  const el = document.createElement(tag);
  if (tag.includes('-')) el.props = props ?? {};
  else {
    for (const name in props) {
      const value = props[name];
      // untrack: um handler nunca faz parte do effect que provocou o evento.
      if (name.startsWith('on')) el.addEventListener(name.slice(2), (event) => untrack(() => value(event)));
      else if (typeof value === 'function') effect(() => setProp(el, name, value()));
      else setProp(el, name, value);
    }
  }
  append(el, children);
  return el;
}

export function append(parent, child) {
  if (child == null || child === false || child === true) return;
  if (Array.isArray(child)) for (const item of child) append(parent, item);
  else if (typeof child === 'function') region(parent, child);
  else parent.append(child);
}

// Trecho do DOM recalculado quando `fn` muda. O que `fn` criar (effects,
// componentes) é descartado na próxima execução.
function region(parent, fn) {
  const start = parent.appendChild(document.createComment(''));
  const end = parent.appendChild(document.createComment(''));
  effect(() => {
    const value = fn();
    const first = start.nextSibling;
    const isText = typeof value === 'string' || typeof value === 'number';
    if (isText && first.nodeType === 3 && first.nextSibling === end) {
      first.data = value;
      return;
    }
    while (start.nextSibling !== end) start.nextSibling.remove();
    const fragment = document.createDocumentFragment();
    append(fragment, value);
    end.before(fragment);
  });
}

const move = (parent, node, before) =>
  node.isConnected && parent.moveBefore ? parent.moveBefore(node, before) : parent.insertBefore(node, before);

/**
 * Lista com chave: os filhos de `parent` passam a espelhar `items()`.
 * `render(item, key)` roda uma vez por chave; `item` é um getter que acompanha
 * o objeto atual daquela chave. Linhas removidas têm seus effects descartados.
 * @template T
 * @param {Element} parent
 * @param {() => T[]} items
 * @param {(item: T) => string | number} key
 * @param {(item: () => T, key: string | number) => Node} render
 */
export function list(parent, items, key, render) {
  const rows = new Map();
  const byKey = computed(() => new Map(items().map((item) => [key(item), item])));
  onCleanup(() => rows.forEach((row) => row.dispose()));
  effect(() => {
    const current = byKey();
    for (const [k, row] of rows) {
      if (current.has(k)) continue;
      row.dispose();
      row.node.remove();
      rows.delete(k);
    }
    let cursor = parent.firstChild;
    for (const k of current.keys()) {
      let row = rows.get(k);
      if (!row) {
        row = {};
        row.dispose = scope(() => {
          row.node = render(computed(() => byKey().get(k)), k);
        });
        rows.set(k, row);
      }
      if (row.node === cursor) cursor = cursor.nextSibling;
      else move(parent, row.node, cursor);
    }
  });
  return parent;
}
