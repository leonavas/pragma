import { batch, signal } from '../lib/reactive.js';

/** @typedef {{ id: string, nome: string, categoria: string, preco: number, estoque: number }} Product */

/** @type {ReturnType<typeof signal<'idle' | 'loading' | 'error' | 'ready'>>} */
export const productsStatus = signal('idle');
export const products = signal(/** @type {Product[]} */ ([]));

const DELAY_MS = 400; // atraso simulado pedido pela spec

/**
 * Carrega o catálogo uma vez; depois de pronto, fica em cache até o reload.
 * `fail` força o erro (usado por `?falhar=1`).
 */
export function loadProducts({ fail = false } = {}) {
  if (productsStatus() === 'ready' || productsStatus() === 'loading') return;
  productsStatus.set('loading');
  new Promise((resolve) => setTimeout(resolve, DELAY_MS))
    .then(() => {
      if (fail) throw new Error('Falha forçada por ?falhar=1');
      return fetch('/produtos.json');
    })
    .then((response) => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    })
    .then(
      (data) =>
        batch(() => {
          products.set(data);
          productsStatus.set('ready');
        }),
      () => productsStatus.set('error'),
    );
}

/** A URL atual pede a falha simulada? */
export const shouldFail = () => new URLSearchParams(location.search).get('falhar') === '1';
