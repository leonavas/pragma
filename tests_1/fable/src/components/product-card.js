import { define } from '../lib/component.js';
import { h } from '../lib/dom.js';
import { signal } from '../lib/reactive.js';
import { brl } from '../format.js';
import { addToCart, quantityInCart } from '../state/cart.js';

// Props: product, detail? (na página do produto o nome é o título, sem link).
define('m-product-card', ({ product, detail = false }) => {
  const { id, nome, categoria, preco, estoque } = product;
  const quantity = signal(1);
  const inCart = quantityInCart(id);
  return h(
    'article',
    { class: 'product-card' },
    detail ? h('h1', { class: 'product-name' }, nome) : h('h3', { class: 'product-name' }, h('a', { href: `/loja/${id}` }, nome)),
    h('p', { class: 'product-category' }, categoria),
    h('p', { class: 'product-price' }, brl(preco)),
    estoque === 0 ? h('p', { class: 'badge' }, 'Esgotado') : h('p', { class: 'product-stock' }, `${estoque} em estoque`),
    h('m-counter', { label: 'Qtd', initial: 1, step: 1, min: 1, max: estoque, onChange: (value) => quantity.set(value) }),
    h(
      'button',
      { type: 'button', class: 'button button-primary', disabled: () => inCart() >= estoque, onclick: () => addToCart(product, quantity()) },
      'Adicionar',
    ),
  );
});
