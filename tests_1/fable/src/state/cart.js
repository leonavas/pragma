import { computed } from '../lib/reactive.js';
import { persisted } from './persisted.js';

// Cada item guarda nome, preço e estoque do produto: o carrinho aparece em
// páginas que não carregam o catálogo (Painel, header).
/** @typedef {{ id: string, nome: string, preco: number, estoque: number, qtd: number }} CartItem */

export const cart = persisted('mercadinho:carrinho', /** @type {CartItem[]} */ ([]));

export const cartCount = computed(() => cart().reduce((sum, item) => sum + item.qtd, 0));
export const cartTotal = computed(() => cart().reduce((sum, item) => sum + item.qtd * item.preco, 0));

/** Getter reativo da quantidade de um produto no carrinho. */
export const quantityInCart = (id) => computed(() => cart().find((item) => item.id === id)?.qtd ?? 0);

/** Soma `qtd` ao item, sem passar do estoque. */
export function addToCart(product, qtd) {
  const { id, nome, preco, estoque } = product;
  cart.update((items) => {
    const existing = items.find((item) => item.id === id);
    const next = Math.min(estoque, (existing?.qtd ?? 0) + qtd);
    if (next <= 0) return items;
    if (!existing) return [...items, { id, nome, preco, estoque, qtd: next }];
    return items.map((item) => (item.id === id ? { ...item, qtd: next } : item));
  });
}

/** Soma `delta` à quantidade do item, entre 1 e o estoque. */
export function changeQuantity(id, delta) {
  cart.update((items) =>
    items.map((item) =>
      item.id === id ? { ...item, qtd: Math.min(item.estoque, Math.max(1, item.qtd + delta)) } : item,
    ),
  );
}

export function removeFromCart(id) {
  cart.update((items) => items.filter((item) => item.id !== id));
}

export function clearCart() {
  cart.set([]);
}
